use avian3d::prelude::{
    Collider, Gravity, LinearVelocity, MoveAndSlide, Position, RigidBody, Rotation, ShapeCastConfig, SpatialQuery,
    SpatialQueryFilter,
};
use bevy::animation::{AnimatedBy, AnimationTargetId};
use bevy::ecs::system::SystemParam;
use bevy::prelude::*;
use bevy::world_serialization::{WorldAsset, WorldInstanceReady};

use crate::collision::{integrate_fall, slide_translation, snapped_ground_height};
use crate::player::Player;
use crate::ragdoll::{BoneMap, OwnedByEnemy, RagdollBodyPart, setup_ragdoll};
use crate::state::GameState;

mod fighter;
mod gunner;

pub use fighter::Fighter;
pub use gunner::Gunner;

pub struct EnemiesPlugin;

impl Plugin for EnemiesPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, enemy_behavior.run_if(in_state(GameState::Playing)))
            .add_systems(PostUpdate, (setup_external_animation, update_enemy_animations));
    }
}

#[derive(Component)]
pub struct Enemy;

#[derive(Component)]
pub struct Dying;

#[derive(EntityEvent)]
pub(crate) struct EnemyHit {
    pub(crate) entity: Entity,
    pub(crate) body: Entity,
    pub(crate) impulse: Vec3,
    pub(crate) point: Vec3,
}

#[derive(Component, Clone)]
pub enum EnemyKind {
    Fighter(Fighter),
    Gunner(Gunner),
}

impl EnemyKind {
    fn rig(&self) -> &EnemyRig {
        match self {
            Self::Fighter(fighter) => &fighter.rig,
            Self::Gunner(gunner) => &gunner.rig,
        }
    }

    fn update(
        &self,
        player: &Transform,
        enemy: &mut Transform,
        activity: &mut EnemyActivity,
        delta_secs: f32,
        obstacles: Obstacles,
    ) {
        match self {
            Self::Fighter(fighter) => fighter.update(player, enemy, activity, delta_secs, obstacles),
            Self::Gunner(gunner) => gunner.update(player, enemy, activity, delta_secs, obstacles),
        }
    }
}

#[derive(Clone, Copy)]
struct EnemyRig {
    scene: &'static str,
    animation_source: &'static str,
    idle_animation: usize,
    moving_animation: usize,
    attack_animation: usize,
    weapon: WeaponSpec,
}

#[derive(Clone, Copy)]
struct WeaponSpec {
    path: &'static str,
    scale: f32,
    rotation_euler_yxz: (f32, f32, f32),
    translation: Vec3,
    collider_half_extents: Vec3,
}

fn planar_direction(player: &Transform, enemy: &Transform) -> Option<(Vec3, f32)> {
    let offset = player.translation - enemy.translation;
    let planar_offset = Vec3::new(offset.x, 0.0, offset.z);
    let distance = planar_offset.length();
    (distance > 0.0).then(|| (planar_offset / distance, distance))
}

const OBSTACLE_PROBE_DISTANCE: f32 = 2.0;
const OBSTACLE_AVOIDANCE_ANGLES_DEGREES: [f32; 5] = [0.0, 45.0, -45.0, 90.0, -90.0];

#[derive(Clone, Copy)]
struct Obstacles<'a> {
    spatial_query: &'a SpatialQuery<'a, 'a>,
    radius: f32,
    center_height_offset: f32,
    filter: &'a SpatialQueryFilter,
}

impl Obstacles<'_> {
    fn clear_direction(&self, from: Vec3, desired: Vec3) -> Vec3 {
        avoid_obstacles(
            self.spatial_query,
            from,
            self.radius,
            self.center_height_offset,
            self.filter,
            desired,
        )
    }
}

fn avoid_obstacles(
    spatial_query: &SpatialQuery,
    from: Vec3,
    radius: f32,
    center_height_offset: f32,
    filter: &SpatialQueryFilter,
    desired: Vec3,
) -> Vec3 {
    let distance = desired.length();
    if distance == 0.0 {
        return Vec3::ZERO;
    }
    let direction = desired / distance;
    for angle_degrees in OBSTACLE_AVOIDANCE_ANGLES_DEGREES {
        let candidate = Quat::from_rotation_y(angle_degrees.to_radians()) * direction;
        let Ok(candidate_direction) = Dir3::new(candidate) else {
            continue;
        };
        if spatial_query
            .cast_shape(
                &Collider::sphere(radius),
                from + Vec3::Y * center_height_offset,
                Quat::IDENTITY,
                candidate_direction,
                &ShapeCastConfig::from_max_distance(OBSTACLE_PROBE_DISTANCE),
                filter,
            )
            .is_none()
        {
            return candidate_direction * distance;
        }
    }
    Vec3::ZERO
}

#[derive(Component, Default)]
pub struct EnemyActivity {
    pub state: AnimationState,
    attack_cooldown: f32,
}

impl EnemyActivity {
    fn attack_on_cooldown(&mut self, cooldown_seconds: f32, animation_seconds: f32) {
        if self.attack_cooldown <= 0.0 {
            self.attack_cooldown = cooldown_seconds;
        }
        let attacking = self.attack_cooldown > cooldown_seconds - animation_seconds;
        self.state = if attacking {
            AnimationState::Attacking
        } else {
            AnimationState::Idle
        };
    }
}

#[derive(Component, Clone, Copy, Default, PartialEq, Eq, Debug)]
pub enum AnimationState {
    #[default]
    Idle,
    Moving,
    Attacking,
}

#[derive(Clone, Component)]
struct EnemyAnimations {
    idle: Handle<AnimationClip>,
    moving: Handle<AnimationClip>,
    attack: Handle<AnimationClip>,
    weapon: Handle<WorldAsset>,
}

#[derive(Component)]
struct ArmatureAnimation {
    idle: AnimationNodeIndex,
    moving: AnimationNodeIndex,
    attacking: AnimationNodeIndex,
    current: Option<AnimationState>,
}

#[derive(Component)]
struct EnemyArmature(Entity);

#[derive(Component)]
struct EnemyWeapon(Entity);

pub fn spawn_enemy(
    commands: &mut Commands,
    asset_server: &AssetServer,
    kind: EnemyKind,
    transform: Transform,
    alive: bool,
) -> Entity {
    let rig = *kind.rig();
    let mut enemy = commands.spawn((
        Enemy,
        WorldAssetRoot(asset_server.load(GltfAssetLabel::Scene(0).from_asset(rig.scene))),
        kind,
        transform,
    ));
    if alive {
        enemy.insert((
            EnemyAnimations {
                idle: asset_server.load(GltfAssetLabel::Animation(rig.idle_animation).from_asset(rig.animation_source)),
                moving: asset_server
                    .load(GltfAssetLabel::Animation(rig.moving_animation).from_asset(rig.animation_source)),
                attack: asset_server
                    .load(GltfAssetLabel::Animation(rig.attack_animation).from_asset(rig.animation_source)),
                weapon: asset_server.load(GltfAssetLabel::Scene(0).from_asset(rig.weapon.path)),
            },
            EnemyActivity::default(),
            LinearVelocity::default(),
        ));
    } else {
        enemy.insert(Dying);
    }
    enemy
        .observe(setup_ragdoll)
        .observe(prepare_enemy_animation)
        .observe(kill_enemy_on_hit)
        .id()
}

#[derive(SystemParam)]
pub(crate) struct RagdollDiagnostics<'w, 's> {
    alive_enemies: Query<'w, 's, (), AliveEnemy>,
    enemy_transforms: Query<'w, 's, &'static Transform, With<Enemy>>,
    owned_bodies: Query<
        'w,
        's,
        (
            Entity,
            &'static OwnedByEnemy,
            &'static RagdollBodyPart,
            &'static Position,
        ),
    >,
}

impl RagdollDiagnostics<'_, '_> {
    fn is_alive(&self, enemy: Entity) -> bool {
        self.alive_enemies.get(enemy).is_ok()
    }

    fn log_kill(&self, enemy: Entity, body: Entity) {
        let root_position = self
            .enemy_transforms
            .get(enemy)
            .map(|transform| transform.translation)
            .unwrap_or_default();
        let hit_part = self
            .owned_bodies
            .iter()
            .find(|(entity, owner, ..)| owner.0 == enemy && *entity == body)
            .map(|(_, _, part, _)| format!("{:?}", part))
            .unwrap_or_else(|| "unknown".to_string());
        let parts = self
            .owned_bodies
            .iter()
            .filter(|(_, owner, ..)| owner.0 == enemy)
            .map(|(_, _, part, position)| {
                format!(
                    "{:?}@{:.2?}({:.1}m)",
                    part,
                    position.0,
                    position.0.distance(root_position)
                )
            })
            .collect::<Vec<_>>()
            .join(" ");
        info!(
            "RAGDOLL KILL root={:?} body={:?} hit_part={} root_pos={:.2?} parts=[{}]",
            enemy, body, hit_part, root_position, parts
        );
    }
}

fn kill_enemy_on_hit(
    hit: On<EnemyHit>,
    mut commands: Commands,
    armatures: Query<&EnemyArmature>,
    mut animation_players: Query<&mut AnimationPlayer>,
    enemies: Query<(&EnemyKind, &EnemyWeapon)>,
    weapon_transforms: Query<&GlobalTransform>,
    ragdoll: RagdollDiagnostics,
) {
    if !ragdoll.is_alive(hit.entity) {
        return;
    }

    ragdoll.log_kill(hit.entity, hit.body);

    commands.entity(hit.entity).insert(Dying);
    if let Ok(mut player) = armatures
        .get(hit.entity)
        .and_then(|armature| animation_players.get_mut(armature.0))
    {
        player.stop_all();
    }
    drop_weapon(hit.entity, &mut commands, &enemies, &weapon_transforms);
}

fn drop_weapon(
    enemy: Entity,
    commands: &mut Commands,
    enemies: &Query<(&EnemyKind, &EnemyWeapon)>,
    weapon_transforms: &Query<&GlobalTransform>,
) {
    let Ok((kind, weapon)) = enemies.get(enemy) else {
        return;
    };
    let Ok(global) = weapon_transforms.get(weapon.0) else {
        return;
    };
    let collider_half_extents = kind.rig().weapon.collider_half_extents;

    let (scale, rotation, translation) = global.to_scale_rotation_translation();
    commands.entity(weapon.0).remove::<ChildOf>();
    commands.entity(weapon.0).insert((
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

#[derive(Component)]
struct PendingAnimationSetup {
    animations: EnemyAnimations,
}

fn prepare_enemy_animation(
    scene_ready: On<WorldInstanceReady>,
    mut commands: Commands,
    enemy_animations: Query<&EnemyAnimations>,
) {
    let Ok(animations) = enemy_animations.get(scene_ready.entity) else {
        return;
    };
    commands.entity(scene_ready.entity).insert(PendingAnimationSetup {
        animations: animations.clone(),
    });
}

fn setup_external_animation(
    mut commands: Commands,
    bone_maps: Query<&BoneMap>,
    children: Query<&Children>,
    names: Query<&Name>,
    pending: Query<(Entity, &PendingAnimationSetup, &EnemyKind)>,
    mut graphs: ResMut<Assets<AnimationGraph>>,
    players: Query<&AnimationPlayer>,
) {
    for (enemy_entity, setup, kind) in &pending {
        let Ok(bones) = bone_maps.get(enemy_entity) else {
            continue;
        };
        let Some(armature) = bones.get("Armature") else {
            continue;
        };

        let animations = setup.animations.clone();
        let (mut graph, idle) = AnimationGraph::from_clip(animations.idle.clone());
        let moving = graph.add_clip(animations.moving.clone(), 1.0, graph.root);
        let attacking = graph.add_clip(animations.attack.clone(), 1.0, graph.root);
        let graph_handle = graphs.add(graph);

        if players.get(armature).is_err() {
            commands.entity(armature).insert(AnimationPlayer::default());
            build_animation_targets(&mut commands, &children, &names, armature, armature, vec![]);
        }
        commands.entity(armature).insert((
            AnimationGraphHandle(graph_handle.clone()),
            ArmatureAnimation {
                idle,
                moving,
                attacking,
                current: None,
            },
        ));

        commands
            .entity(enemy_entity)
            .insert(EnemyArmature(armature))
            .remove::<PendingAnimationSetup>();
        attach_weapon(&mut commands, enemy_entity, bones, &animations, &kind.rig().weapon);
    }
}

fn attach_weapon(
    commands: &mut Commands,
    enemy_entity: Entity,
    bones: &BoneMap,
    animations: &EnemyAnimations,
    weapon: &WeaponSpec,
) {
    let Some(right_hand) = bones.get("hand_r") else {
        return;
    };

    let (yaw, pitch, roll) = weapon.rotation_euler_yxz;
    let weapon_entity = commands
        .spawn((
            Name::new("Weapon"),
            WorldAssetRoot(animations.weapon.clone()),
            Transform {
                translation: weapon.translation,
                rotation: Quat::from_euler(EulerRot::YXZ, yaw, pitch, roll),
                scale: Vec3::splat(weapon.scale),
            },
        ))
        .id();

    commands.entity(right_hand).add_children(&[weapon_entity]);
    commands.entity(enemy_entity).insert(EnemyWeapon(weapon_entity));
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
        let mut path = path;
        path.push(name.clone());
        commands
            .entity(current)
            .insert((AnimationTargetId::from_names(path.iter()), AnimatedBy(animation_root)));
        if let Ok(descendants) = children.get(current) {
            for descendant in descendants.iter() {
                let path = path.clone();
                build_animation_targets(commands, children, names, animation_root, descendant, path);
            }
        }
    }
}

fn update_enemy_animations(
    enemies: Query<(&EnemyActivity, &EnemyArmature)>,
    mut armatures: Query<(&mut ArmatureAnimation, &mut AnimationPlayer)>,
) {
    for (activity, armature) in &enemies {
        let Ok((mut armature_animation, mut player)) = armatures.get_mut(armature.0) else {
            continue;
        };

        if armature_animation.current == Some(activity.state) {
            continue;
        }

        let node_index = match activity.state {
            AnimationState::Idle => armature_animation.idle,
            AnimationState::Moving => armature_animation.moving,
            AnimationState::Attacking => armature_animation.attacking,
        };

        player.stop_all();
        player.start(node_index).repeat();
        armature_animation.current = Some(activity.state);
    }
}

pub(crate) type AliveEnemy = (With<Enemy>, Without<Dying>);
type EnemyBehavior = (
    Entity,
    &'static EnemyKind,
    &'static mut Transform,
    &'static mut EnemyActivity,
    &'static mut LinearVelocity,
);

const ENEMY_COLLISION_RADIUS: f32 = 0.35;
const ENEMY_COLLISION_CENTER_HEIGHT: f32 = 1.0;

fn enemy_behavior(
    time: Res<Time>,
    move_and_slide: MoveAndSlide,
    gravity: Res<Gravity>,
    player: Query<&Transform, (With<Player>, Without<Enemy>)>,
    mut enemies: Query<EnemyBehavior, (AliveEnemy, Without<Player>)>,
    owned_bodies: Query<(Entity, &OwnedByEnemy)>,
) {
    let Ok(player_transform) = player.single() else {
        return;
    };
    let delta_secs = time.delta_secs();

    for (enemy_entity, kind, mut enemy_transform, mut activity, mut velocity) in &mut enemies {
        activity.attack_cooldown -= delta_secs;
        let filter = own_collider_filter(enemy_entity, &owned_bodies);
        let obstacles = Obstacles {
            spatial_query: &move_and_slide.spatial_query,
            radius: ENEMY_COLLISION_RADIUS,
            center_height_offset: ENEMY_COLLISION_CENTER_HEIGHT,
            filter: &filter,
        };
        let previous_translation = enemy_transform.translation;
        kind.update(
            player_transform,
            &mut enemy_transform,
            &mut activity,
            delta_secs,
            obstacles,
        );
        let desired_translation = enemy_transform.translation - previous_translation;
        enemy_transform.translation = previous_translation
            + slide_translation(
                &move_and_slide,
                previous_translation,
                ENEMY_COLLISION_RADIUS,
                ENEMY_COLLISION_CENTER_HEIGHT,
                &filter,
                desired_translation,
                delta_secs,
            );
        let ground_height = snapped_ground_height(
            &move_and_slide.spatial_query,
            enemy_transform.translation + Vec3::Y * ENEMY_COLLISION_CENTER_HEIGHT,
            &filter,
        );
        integrate_fall(
            &mut velocity,
            &mut enemy_transform.translation.y,
            ground_height,
            0.0,
            delta_secs,
            gravity.0.y,
        );
    }
}

fn own_collider_filter(enemy_entity: Entity, owned_bodies: &Query<(Entity, &OwnedByEnemy)>) -> SpatialQueryFilter {
    SpatialQueryFilter::default().with_excluded_entities(
        owned_bodies
            .iter()
            .filter(|(_, owner)| owner.0 == enemy_entity)
            .map(|(body_entity, _)| body_entity),
    )
}
