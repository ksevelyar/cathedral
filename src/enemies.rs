use avian3d::prelude::{Collider, Position, RigidBody, Rotation};
use bevy::animation::{AnimatedBy, AnimationTargetId};
use bevy::prelude::*;
use bevy::world_serialization::{WorldAsset, WorldInstanceReady};

use crate::player::Player;
use crate::ragdoll::setup_ragdoll;
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

    fn update(&self, player: &Transform, enemy: &mut Transform, activity: &mut EnemyActivity, delta_secs: f32) {
        match self {
            Self::Fighter(fighter) => fighter.update(player, enemy, activity, delta_secs),
            Self::Gunner(gunner) => gunner.update(player, enemy, activity, delta_secs),
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

#[derive(Component, Default)]
struct EnemyActivity {
    state: AnimationState,
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

#[derive(Component, Clone, Copy, Default, PartialEq, Eq)]
enum AnimationState {
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

fn kill_enemy_on_hit(
    hit: On<EnemyHit>,
    alive_enemies: Query<(), AliveEnemy>,
    mut commands: Commands,
    children: Query<&Children>,
    mut animation_players: Query<&mut AnimationPlayer>,
    enemy_kinds: Query<&EnemyKind>,
    weapons: Query<(&Name, &GlobalTransform)>,
) {
    if alive_enemies.get(hit.entity).is_err() {
        return;
    }

    commands.entity(hit.entity).insert(Dying);
    stop_enemy_animation(hit.entity, &children, &mut animation_players);
    drop_weapon(hit.entity, &mut commands, &children, &enemy_kinds, &weapons);
}

fn stop_enemy_animation(
    enemy: Entity,
    children: &Query<&Children>,
    animation_players: &mut Query<&mut AnimationPlayer>,
) {
    for descendant in children.iter_descendants(enemy) {
        if let Ok(mut player) = animation_players.get_mut(descendant) {
            player.stop_all();
        }
    }
}

fn drop_weapon(
    enemy: Entity,
    commands: &mut Commands,
    children: &Query<&Children>,
    enemy_kinds: &Query<&EnemyKind>,
    weapons: &Query<(&Name, &GlobalTransform)>,
) {
    let Ok(kind) = enemy_kinds.get(enemy) else {
        return;
    };
    let collider_half_extents = kind.rig().weapon.collider_half_extents;

    for descendant in children.iter_descendants(enemy) {
        if !weapons.get(descendant).is_ok_and(|(name, _)| name.as_str() == "Weapon") {
            continue;
        }
        let Ok((_, global)) = weapons.get(descendant) else {
            continue;
        };
        let (scale, rotation, translation) = global.to_scale_rotation_translation();
        commands.entity(descendant).remove::<ChildOf>();
        commands.entity(descendant).insert((
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
    children: Query<&Children>,
    names: Query<&Name>,
    pending: Query<(Entity, &PendingAnimationSetup, &EnemyKind)>,
    mut graphs: ResMut<Assets<AnimationGraph>>,
    players: Query<&AnimationPlayer>,
) {
    for (enemy_entity, setup, kind) in &pending {
        let Some(armature) = children
            .iter_descendants(enemy_entity)
            .find(|&descendant| names.get(descendant).is_ok_and(|name| name.as_str() == "Armature"))
        else {
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

        commands.entity(enemy_entity).remove::<PendingAnimationSetup>();
        attach_weapon(
            &mut commands,
            enemy_entity,
            &children,
            &names,
            &animations,
            &kind.rig().weapon,
        );
    }
}

fn attach_weapon(
    commands: &mut Commands,
    enemy_entity: Entity,
    children: &Query<&Children>,
    names: &Query<&Name>,
    animations: &EnemyAnimations,
    weapon: &WeaponSpec,
) {
    let Some(right_hand) = children
        .iter_descendants(enemy_entity)
        .find(|&descendant| names.get(descendant).is_ok_and(|name| name.as_str() == "hand_r"))
    else {
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
    children: Query<&Children>,
    names: Query<&Name>,
    enemies: Query<(Entity, &EnemyActivity)>,
    mut armatures: Query<(&mut ArmatureAnimation, &mut AnimationPlayer)>,
) {
    for (enemy_entity, activity) in &enemies {
        let Some(armature) = children
            .iter_descendants(enemy_entity)
            .find(|&descendant| names.get(descendant).is_ok_and(|name| name.as_str() == "Armature"))
        else {
            continue;
        };

        let Ok((mut armature_animation, mut player)) = armatures.get_mut(armature) else {
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

fn enemy_behavior(
    time: Res<Time>,
    player: Query<&Transform, (With<Player>, Without<Enemy>)>,
    mut enemies: Query<(&EnemyKind, &mut Transform, &mut EnemyActivity), (AliveEnemy, Without<Player>)>,
) {
    let Ok(player_transform) = player.single() else {
        return;
    };
    let delta_secs = time.delta_secs();

    for (kind, mut enemy_transform, mut activity) in &mut enemies {
        activity.attack_cooldown -= delta_secs;
        kind.update(player_transform, &mut enemy_transform, &mut activity, delta_secs);
    }
}
