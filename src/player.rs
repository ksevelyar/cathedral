use avian3d::prelude::{
    Collider, Gravity, LinearVelocity, MoveAndSlide, Position, RigidBody, Rotation, SpatialQueryFilter,
};
use bevy::audio::SpatialListener;
use bevy::camera::Exposure;
use bevy::input::mouse::AccumulatedMouseMotion;
use bevy::light::VolumetricFog;
use bevy::post_process::bloom::Bloom;
use bevy::prelude::*;

use crate::enemies::Dying;
use crate::health::{CharacterDied, Health};
use crate::maps::PlayerPosition;
use crate::movement::PhysicsWorld;
use crate::ragdoll::{
    OwnedByCharacter, PendingRagdollImpactRequest, RagdollBodyPart, RagdollData, exclude_character_bodies,
    setup_ragdoll,
};
use crate::shooting::{Gun, WeaponSlot};

fn exclude_player_bodies(
    player_entity: Entity,
    owned_bodies: &Query<(Entity, &OwnedByCharacter)>,
    player_children: &Query<&Children>,
) -> SpatialQueryFilter {
    let mut owner_entities: Vec<Entity> = Vec::new();
    if let Ok(children) = player_children.get(player_entity) {
        owner_entities.extend(children.iter());
    }
    owner_entities.push(player_entity);
    exclude_character_bodies(&owner_entities, owned_bodies)
}
use crate::state::GameState;

pub struct PlayerPlugin;

impl Plugin for PlayerPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Update,
            (
                apply_mouse_look.run_if(not(in_state(GameState::Paused))),
                settle_player_on_startup,
                move_player,
                kill_player_on_death,
            )
                .run_if(in_state(GameState::Playing)),
        );
        app.add_systems(PostUpdate, fall_camera_to_head_on_death);
    }
}

#[derive(Component)]
pub struct Player;

#[derive(Component)]
pub struct PlayerCamera;

#[derive(Component)]
pub struct CameraState {
    pub yaw: f32,
    pub pitch: f32,
}

impl Default for CameraState {
    fn default() -> Self {
        Self { yaw: 0.0, pitch: 0.0 }
    }
}

const MOUSE_SENSITIVITY: f32 = 0.003;
const PLAYER_MOVE_SPEED: f32 = 5.0;
const CAMERA_PITCH_LIMIT: f32 = std::f32::consts::FRAC_PI_2 - 0.01;
const PLAYER_COLLISION_RADIUS: f32 = 0.4;
const PLAYER_EYE_HEIGHT: f32 = 1.8;
const PLAYER_BODY_ASSET_PATH: &str = "characters/bodies/female.gltf";
const PLAYER_BODY_CAMERA_OFFSET: Vec3 = Vec3::new(0.0, -1.85, 0.3);

pub fn setup_player(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    start_position: Option<Res<PlayerPosition>>,
) {
    let player_position = start_position
        .map(|start_position| *start_position)
        .unwrap_or_else(|| PlayerPosition::new(Vec3::ZERO));
    spawn_player(&mut commands, &asset_server, player_position);
}

pub fn spawn_player(
    commands: &mut Commands,
    asset_server: &AssetServer,
    player_position: PlayerPosition,
) -> (Entity, Entity) {
    let camera_bloom_intensity = 0.01;
    let player_starting_health = 100.0;
    let human_ear_gap = 0.15;
    let (yaw, pitch) = compute_look_angles(player_position.position, player_position.look_at);
    let player_entity = commands
        .spawn((
            Transform::from_translation(player_position.position).with_rotation(Quat::from_euler(
                EulerRot::YXZ,
                yaw,
                0.0,
                0.0,
            )),
            CameraState { yaw, pitch },
            LinearVelocity::default(),
            Name::new("Player"),
            Health::new(player_starting_health),
            Player,
        ))
        .id();
    let player_camera_entity = commands
        .spawn((
            Camera3d::default(),
            Exposure::INDOOR,
            Bloom {
                intensity: camera_bloom_intensity,
                ..Bloom::NATURAL
            },
            VolumetricFog {
                ambient_intensity: 0.0,
                step_count: 48,
                ..default()
            },
            Transform::from_rotation(Quat::from_euler(EulerRot::YXZ, 0.0, pitch, 0.0)),
            SpatialListener::new(human_ear_gap),
            PlayerCamera,
        ))
        .id();
    let player_body_entity = commands
        .spawn((
            WorldAssetRoot(asset_server.load(GltfAssetLabel::Scene(0).from_asset(PLAYER_BODY_ASSET_PATH))),
            Transform::from_translation(PLAYER_BODY_CAMERA_OFFSET),
        ))
        .id();
    commands.entity(player_body_entity).observe(setup_ragdoll);
    commands.entity(player_entity).add_child(player_body_entity);
    commands.entity(player_entity).add_child(player_camera_entity);
    (player_entity, player_camera_entity)
}

#[derive(Component)]
pub struct PlayerDead;

type AlivePlayerQuery<'w, 's> =
    Query<'w, 's, (Entity, &'static LinearVelocity, &'static Children), (With<Player>, Without<PlayerDead>)>;

type DeadPlayerQuery<'w, 's> =
    Query<'w, 's, (&'static Transform, &'static Children), (With<Player>, With<PlayerDead>, Without<PlayerCamera>)>;

fn kill_player_on_death(
    mut deaths: MessageReader<CharacterDied>,
    players: AlivePlayerQuery,
    ragdoll_roots: Query<(), With<RagdollData>>,
    guns: Query<(Entity, &WeaponSlot, &GlobalTransform), With<Gun>>,
    child_ofs: Query<&ChildOf>,
    mut next_state: ResMut<NextState<GameState>>,
    mut commands: Commands,
) {
    for death in deaths.read() {
        let Ok((player_entity, velocity, children)) = players.get(death.character) else {
            continue;
        };
        let Some(ragdoll_root) = children.iter().find(|child| ragdoll_roots.contains(*child)) else {
            continue;
        };
        drop_player_weapons(player_entity, &mut commands, &guns, &child_ofs);
        commands.entity(player_entity).insert(PlayerDead);
        commands.entity(ragdoll_root).insert((
            Dying,
            LinearVelocity(velocity.0),
            PendingRagdollImpactRequest {
                body_part: RagdollBodyPart::Head,
                impulse: Vec3::ZERO,
                point: Vec3::ZERO,
            },
        ));
        next_state.set(GameState::GameOver);
    }
}

fn is_descendant_of_entity(candidate: Entity, ancestor: Entity, child_ofs: &Query<&ChildOf>) -> bool {
    let mut current = candidate;
    while let Ok(parent) = child_ofs.get(current).map(|child_of| child_of.parent()) {
        if parent == ancestor {
            return true;
        }
        current = parent;
    }
    false
}

fn drop_player_weapons(
    player_entity: Entity,
    commands: &mut Commands,
    guns: &Query<(Entity, &WeaponSlot, &GlobalTransform), With<Gun>>,
    child_ofs: &Query<&ChildOf>,
) {
    for (weapon_entity, weapon_slot, global_transform) in guns {
        if !is_descendant_of_entity(weapon_entity, player_entity, child_ofs) {
            continue;
        }
        let (scale, rotation, translation) = global_transform.to_scale_rotation_translation();
        let collider_half_extents = weapon_slot.0.get_collider_half_extents();
        commands.entity(weapon_entity).remove::<ChildOf>();
        commands.entity(weapon_entity).insert((
            OwnedByCharacter(player_entity),
            Transform {
                translation,
                rotation,
                scale,
            },
            RigidBody::Dynamic,
            Collider::cuboid(
                collider_half_extents.x,
                collider_half_extents.y,
                collider_half_extents.z,
            ),
            Position(translation),
            Rotation(rotation),
        ));
    }
}

fn fall_camera_to_head_on_death(
    dead_players: DeadPlayerQuery,
    ragdoll_roots: Query<&RagdollData>,
    body_positions: Query<&Position, With<RagdollBodyPart>>,
    mut cameras: Query<&mut Transform, (With<PlayerCamera>, Without<Player>)>,
) {
    let camera_height_above_head = 0.2;
    for (player_transform, children) in &dead_players {
        for child in children.iter() {
            let Ok(ragdoll) = ragdoll_roots.get(child) else {
                continue;
            };
            let Some(head_body) = ragdoll.find_part(RagdollBodyPart::Head) else {
                continue;
            };
            let Ok(head_position) = body_positions.get(head_body) else {
                continue;
            };
            let head_offset = head_position.0 + Vec3::Y * camera_height_above_head - player_transform.translation;
            if let Ok(mut camera_transform) = cameras.single_mut() {
                camera_transform.translation = player_transform.rotation.inverse() * head_offset;
            }
        }
    }
}

pub fn compute_look_angles(eye: Vec3, look_at: Vec3) -> (f32, f32) {
    let direction = (look_at - eye).normalize();
    ((-direction.x).atan2(-direction.z), direction.y.asin())
}

pub type PlayerBodyTransformQuery<'w, 's> =
    Query<'w, 's, (&'static mut Transform, &'static mut CameraState), (With<Player>, Without<PlayerCamera>)>;

pub type PlayerCameraTransformQuery<'w, 's> =
    Query<'w, 's, &'static mut Transform, (With<PlayerCamera>, Without<Player>)>;

pub fn reset_player(
    mut player_query: PlayerBodyTransformQuery,
    mut camera_query: PlayerCameraTransformQuery,
    start_position: &PlayerPosition,
) {
    let Ok((mut transform, mut camera_state)) = player_query.single_mut() else {
        return;
    };

    let (yaw, pitch) = compute_look_angles(start_position.position, start_position.look_at);
    transform.translation = start_position.position;
    transform.rotation = Quat::from_euler(EulerRot::YXZ, yaw, 0.0, 0.0);
    camera_state.yaw = yaw;
    camera_state.pitch = pitch;
    let Ok(mut camera_transform) = camera_query.single_mut() else {
        return;
    };
    camera_transform.rotation = Quat::from_euler(EulerRot::YXZ, 0.0, pitch, 0.0);
}

pub fn settle_player_on_startup(
    mut player: Query<(Entity, &mut Transform, &mut LinearVelocity), Added<Player>>,
    move_and_slide: MoveAndSlide,
    owned_bodies: Query<(Entity, &OwnedByCharacter)>,
    player_children: Query<&Children>,
) {
    for (player_entity, mut transform, mut velocity) in &mut player {
        let filter = exclude_player_bodies(player_entity, &owned_bodies, &player_children);
        let physics_world = PhysicsWorld::new(
            &move_and_slide,
            0.0,
            0.0,
            &filter,
            PLAYER_COLLISION_RADIUS,
            0.0,
            PLAYER_EYE_HEIGHT,
        );
        if let Some(ground_height) = physics_world.find_ground_height(transform.translation) {
            transform.translation.y = ground_height + PLAYER_EYE_HEIGHT;
            velocity.y = 0.0;
        }
    }
}

pub fn apply_mouse_look(
    mouse_motion: Res<AccumulatedMouseMotion>,
    mut query: PlayerBodyTransformQuery,
    mut camera_query: PlayerCameraTransformQuery,
) {
    let Ok((mut transform, mut camera_state)) = query.single_mut() else {
        return;
    };

    let mouse_delta = mouse_motion.delta;
    if mouse_delta == Vec2::ZERO {
        return;
    }

    camera_state.yaw -= mouse_delta.x * MOUSE_SENSITIVITY;
    camera_state.pitch =
        (camera_state.pitch - mouse_delta.y * MOUSE_SENSITIVITY).clamp(-CAMERA_PITCH_LIMIT, CAMERA_PITCH_LIMIT);

    transform.rotation = Quat::from_euler(EulerRot::YXZ, camera_state.yaw, 0.0, 0.0);
    if let Ok(mut camera_transform) = camera_query.single_mut() {
        camera_transform.rotation = Quat::from_euler(EulerRot::YXZ, 0.0, camera_state.pitch, 0.0);
    }
}

pub fn move_player(
    keyboard: Res<ButtonInput<KeyCode>>,
    time: Res<Time>,
    move_and_slide: MoveAndSlide,
    gravity: Res<Gravity>,
    owned_bodies: Query<(Entity, &OwnedByCharacter)>,
    player_children: Query<&Children>,
    mut query: Query<(Entity, &mut Transform, &CameraState, &mut LinearVelocity), With<Player>>,
) {
    let Ok((player_entity, mut transform, camera_state, mut velocity)) = query.single_mut() else {
        return;
    };

    let forward_direction = Vec3::new(-camera_state.yaw.sin(), 0.0, -camera_state.yaw.cos());
    let right_direction = Vec3::new(camera_state.yaw.cos(), 0.0, -camera_state.yaw.sin());

    let mut movement_direction = Vec3::ZERO;
    if keyboard.pressed(KeyCode::KeyW) {
        movement_direction += forward_direction;
    }
    if keyboard.pressed(KeyCode::KeyS) {
        movement_direction -= forward_direction;
    }
    if keyboard.pressed(KeyCode::KeyD) {
        movement_direction += right_direction;
    }
    if keyboard.pressed(KeyCode::KeyA) {
        movement_direction -= right_direction;
    }

    let filter = exclude_player_bodies(player_entity, &owned_bodies, &player_children);
    let physics_world = PhysicsWorld::new(
        &move_and_slide,
        gravity.0.y,
        time.delta_secs(),
        &filter,
        PLAYER_COLLISION_RADIUS,
        0.0,
        PLAYER_EYE_HEIGHT,
    );

    if movement_direction != Vec3::ZERO {
        let desired_translation = movement_direction.normalize() * PLAYER_MOVE_SPEED * time.delta_secs();
        let position = transform.translation;
        transform.translation += physics_world.slide(position, desired_translation);
    }
    physics_world.integrate_fall(&mut velocity, &mut transform.translation);
}
