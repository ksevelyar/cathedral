use avian3d::prelude::{Gravity, LinearVelocity, MoveAndSlide, SpatialQueryFilter};
use bevy::camera::Exposure;
use bevy::input::mouse::AccumulatedMouseMotion;
use bevy::light::VolumetricFog;
use bevy::post_process::bloom::Bloom;
use bevy::prelude::*;

use crate::maps::PlayerPosition;
use crate::movement::PhysicsWorld;
use crate::ragdoll::{OwnedByCharacter, exclude_character_bodies, setup_ragdoll};

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
            )
                .run_if(in_state(GameState::Playing)),
        );
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
    let camera_bloom_intensity = 0.01;
    let player_position = start_position
        .map(|start_position| *start_position)
        .unwrap_or_else(|| PlayerPosition::new(Vec3::ZERO));
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
