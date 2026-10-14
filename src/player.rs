use avian3d::prelude::{
    Collider, Gravity, LinearVelocity, MoveAndSlide, Position, RigidBody, Rotation, SpatialQueryFilter,
};
use bevy::animation::{AnimatedBy, AnimationTargetId};
use bevy::audio::SpatialListener;
use bevy::camera::Exposure;
use bevy::camera::visibility::RenderLayers;
use bevy::input::mouse::AccumulatedMouseMotion;
use bevy::light::{NotShadowCaster, NotShadowReceiver, VolumetricFog};
use bevy::post_process::bloom::Bloom;
use bevy::prelude::*;
use bevy::world_serialization::WorldInstanceReady;

use crate::enemies::Dying;
use crate::health::{CharacterDied, Health};
use crate::maps::PlayerPosition;
use crate::movement::PhysicsWorld;
use crate::ragdoll::{
    BoneMap, OwnedByCharacter, PendingRagdollImpactRequest, RagdollBodyPart, RagdollData, exclude_character_bodies,
    setup_ragdoll,
};
use crate::shooting::{Gun, Reloading, ShadowGun, WeaponSlot};

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
        app.add_observer(prepare_player_animation)
            .add_observer(configure_player_shadow_render_layers)
            .add_observer(configure_view_model_shadow_behavior)
            .add_systems(
                PostUpdate,
                (
                    setup_player_animation,
                    attach_player_guns_to_hand,
                    fall_camera_to_head_on_death,
                )
                    .chain(),
            )
            .add_systems(
                Update,
                update_player_animation
                    .after(move_player)
                    .run_if(in_state(GameState::Playing)),
            );
    }
}

#[derive(Component)]
pub struct Player;

#[derive(Component)]
pub struct PlayerCamera;

#[derive(Component)]
struct PlayerVisual;

#[derive(Component, Default)]
pub struct PlayerLocomotion {
    is_moving: bool,
}

#[derive(Component)]
struct PlayerAnimations {
    idle: Handle<AnimationClip>,
    moving: Handle<AnimationClip>,
    reloading: Handle<AnimationClip>,
}

#[derive(Component)]
struct PendingPlayerAnimation;

#[derive(Component)]
struct PlayerArmature(Entity);

#[derive(Component)]
struct PlayerArmatureAnimation {
    idle: AnimationNodeIndex,
    moving: AnimationNodeIndex,
    reloading: AnimationNodeIndex,
    current: Option<PlayerAnimationState>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum PlayerAnimationState {
    Idle,
    Moving,
    Reloading,
}

#[derive(Component)]
struct HandAttachedPlayerGun;

type PlayerShadowProxyQuery<'world, 'state> = Query<'world, 'state, (), Or<(With<PlayerVisual>, With<ShadowGun>)>>;

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
const PLAYER_ANIMATION_ASSET_PATH: &str = "characters/animations/ual1-standard.glb";
const PLAYER_BODY_CAMERA_OFFSET: Vec3 = Vec3::new(0.0, -1.85, 0.3);
const PLAYER_IDLE_ANIMATION_INDEX: usize = 21;
const PLAYER_RELOAD_ANIMATION_INDEX: usize = 22;
const PLAYER_MOVING_ANIMATION_INDEX: usize = 36;

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
            PlayerLocomotion::default(),
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
            PlayerVisual,
            PlayerAnimations {
                idle: asset_server.load(
                    GltfAssetLabel::Animation(PLAYER_IDLE_ANIMATION_INDEX).from_asset(PLAYER_ANIMATION_ASSET_PATH),
                ),
                moving: asset_server.load(
                    GltfAssetLabel::Animation(PLAYER_MOVING_ANIMATION_INDEX).from_asset(PLAYER_ANIMATION_ASSET_PATH),
                ),
                reloading: asset_server.load(
                    GltfAssetLabel::Animation(PLAYER_RELOAD_ANIMATION_INDEX).from_asset(PLAYER_ANIMATION_ASSET_PATH),
                ),
            },
            WorldAssetRoot(asset_server.load(GltfAssetLabel::Scene(0).from_asset(PLAYER_BODY_ASSET_PATH))),
            RenderLayers::layer(1),
            Transform::from_translation(PLAYER_BODY_CAMERA_OFFSET)
                .with_rotation(Quat::from_rotation_y(std::f32::consts::PI)),
        ))
        .id();
    commands.entity(player_body_entity).observe(setup_ragdoll);
    commands.entity(player_entity).add_child(player_body_entity);
    commands.entity(player_entity).add_child(player_camera_entity);
    (player_entity, player_camera_entity)
}

fn prepare_player_animation(
    scene_ready: On<WorldInstanceReady>,
    mut commands: Commands,
    player_visuals: Query<(), With<PlayerVisual>>,
) {
    if player_visuals.contains(scene_ready.entity) {
        commands.entity(scene_ready.entity).insert(PendingPlayerAnimation);
    }
}

fn configure_player_shadow_render_layers(
    scene_ready: On<WorldInstanceReady>,
    mut commands: Commands,
    player_shadow_proxies: PlayerShadowProxyQuery,
    children: Query<&Children>,
) {
    if player_shadow_proxies.contains(scene_ready.entity) {
        apply_shadow_render_layer(&mut commands, &children, scene_ready.entity);
    }
}

fn configure_view_model_shadow_behavior(
    scene_ready: On<WorldInstanceReady>,
    mut commands: Commands,
    visible_guns: Query<(), With<Gun>>,
    children: Query<&Children>,
) {
    if visible_guns.contains(scene_ready.entity) {
        prevent_view_model_shadows(&mut commands, &children, scene_ready.entity);
    }
}

fn prevent_view_model_shadows(commands: &mut Commands, children: &Query<&Children>, entity: Entity) {
    commands.entity(entity).insert((NotShadowCaster, NotShadowReceiver));
    let Ok(entity_children) = children.get(entity) else {
        return;
    };
    for child in entity_children.iter() {
        prevent_view_model_shadows(commands, children, child);
    }
}

fn apply_shadow_render_layer(commands: &mut Commands, children: &Query<&Children>, entity: Entity) {
    commands.entity(entity).insert(RenderLayers::layer(1));
    let Ok(entity_children) = children.get(entity) else {
        return;
    };
    for child in entity_children.iter() {
        apply_shadow_render_layer(commands, children, child);
    }
}

fn setup_player_animation(
    mut commands: Commands,
    pending_visuals: Query<(Entity, &PlayerAnimations, &BoneMap), With<PendingPlayerAnimation>>,
    children: Query<&Children>,
    names: Query<&Name>,
    animation_players: Query<&AnimationPlayer>,
    mut graphs: ResMut<Assets<AnimationGraph>>,
) {
    for (visual_entity, animations, bones) in &pending_visuals {
        let Some(armature_entity) = bones.get("Armature") else {
            continue;
        };
        let (mut animation_graph, idle_animation) = AnimationGraph::from_clip(animations.idle.clone());
        let moving_animation = animation_graph.add_clip(animations.moving.clone(), 1.0, animation_graph.root);
        let reloading_animation = animation_graph.add_clip(animations.reloading.clone(), 1.0, animation_graph.root);
        let animation_graph_handle = graphs.add(animation_graph);
        if animation_players.get(armature_entity).is_err() {
            commands.entity(armature_entity).insert(AnimationPlayer::default());
            build_animation_targets(
                &mut commands,
                &children,
                &names,
                armature_entity,
                armature_entity,
                vec![],
            );
        }
        commands.entity(armature_entity).insert((
            AnimationGraphHandle(animation_graph_handle),
            PlayerArmatureAnimation {
                idle: idle_animation,
                moving: moving_animation,
                reloading: reloading_animation,
                current: None,
            },
        ));
        commands
            .entity(visual_entity)
            .insert(PlayerArmature(armature_entity))
            .remove::<PendingPlayerAnimation>();
    }
}

fn build_animation_targets(
    commands: &mut Commands,
    children: &Query<&Children>,
    names: &Query<&Name>,
    animation_root: Entity,
    current: Entity,
    path: Vec<Name>,
) {
    if let Ok(name) = names.get(current) {
        let mut animation_path = path;
        animation_path.push(name.clone());
        commands.entity(current).insert((
            AnimationTargetId::from_names(animation_path.iter()),
            AnimatedBy(animation_root),
        ));
        if let Ok(descendants) = children.get(current) {
            for descendant in descendants.iter() {
                build_animation_targets(
                    commands,
                    children,
                    names,
                    animation_root,
                    descendant,
                    animation_path.clone(),
                );
            }
        }
    }
}

fn attach_player_guns_to_hand(
    mut commands: Commands,
    player_visuals: Query<(&BoneMap, &PlayerArmature), With<PlayerVisual>>,
    shadow_guns: Query<Entity, (With<ShadowGun>, Without<HandAttachedPlayerGun>)>,
) {
    let Ok((bones, _)) = player_visuals.single() else {
        return;
    };
    let Some(right_hand_entity) = bones.get("hand_r") else {
        return;
    };
    for shadow_gun in &shadow_guns {
        commands.entity(shadow_gun).insert(HandAttachedPlayerGun);
        commands.entity(right_hand_entity).add_child(shadow_gun);
    }
}

fn update_player_animation(
    players: Query<(&PlayerLocomotion, &Children), With<Player>>,
    player_visuals: Query<&PlayerArmature, With<PlayerVisual>>,
    reloading_guns: Query<(), (With<Gun>, With<Reloading>)>,
    mut armatures: Query<(&mut PlayerArmatureAnimation, &mut AnimationPlayer)>,
) {
    let Ok((locomotion, player_children)) = players.single() else {
        return;
    };
    let Some(player_armature) = player_children.iter().find_map(|child| player_visuals.get(child).ok()) else {
        return;
    };
    let Ok((mut animation, mut animation_player)) = armatures.get_mut(player_armature.0) else {
        return;
    };
    let animation_state = if !reloading_guns.is_empty() {
        PlayerAnimationState::Reloading
    } else if locomotion.is_moving {
        PlayerAnimationState::Moving
    } else {
        PlayerAnimationState::Idle
    };
    if animation.current == Some(animation_state) {
        return;
    }
    let (animation_node, repeats) = match animation_state {
        PlayerAnimationState::Idle => (animation.idle, true),
        PlayerAnimationState::Moving => (animation.moving, true),
        PlayerAnimationState::Reloading => (animation.reloading, false),
    };
    animation_player.stop_all();
    let active_animation = animation_player.start(animation_node);
    if repeats {
        active_animation.repeat();
    }
    animation.current = Some(animation_state);
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
    mut query: Query<
        (
            Entity,
            &mut Transform,
            &CameraState,
            &mut LinearVelocity,
            &mut PlayerLocomotion,
        ),
        With<Player>,
    >,
) {
    let Ok((player_entity, mut transform, camera_state, mut velocity, mut locomotion)) = query.single_mut() else {
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

    locomotion.is_moving = false;
    if movement_direction != Vec3::ZERO {
        let desired_translation = movement_direction.normalize() * PLAYER_MOVE_SPEED * time.delta_secs();
        let position = transform.translation;
        let actual_translation = physics_world.slide(position, desired_translation);
        let minimum_movement_distance_squared = 0.000_001;
        locomotion.is_moving = actual_translation.xz().length_squared() > minimum_movement_distance_squared;
        transform.translation += actual_translation;
    }
    physics_world.integrate_fall(&mut velocity, &mut transform.translation);
}
