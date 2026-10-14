use avian3d::prelude::{Collider, Gravity, LinearVelocity, MoveAndSlide, Position, RigidBody, Rotation};
use bevy::animation::{AnimatedBy, AnimationTargetId};
use bevy::ecs::system::SystemParam;
use bevy::prelude::*;
use bevy::world_serialization::{WorldAsset, WorldInstanceReady};
use rand::Rng;

use crate::maps::CurrentMap;
use crate::movement::PhysicsWorld;
use crate::player::Player;
use crate::ragdoll::{BoneMap, OwnedByCharacter, exclude_character_bodies, setup_ragdoll};
use crate::shooting::{Weapon, WeaponFired, observe_muzzle_flash};
use crate::state::GameState;

mod fighter;
mod gunner;

pub use fighter::Fighter;
pub use gunner::Gunner;

pub struct EnemiesPlugin;

impl Plugin for EnemiesPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, update_behaviour.run_if(in_state(GameState::Playing)))
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
    pub fn get_display_name(&self) -> String {
        match self {
            Self::Fighter(_) => "Fighter".to_string(),
            Self::Gunner(_) => "Gunner".to_string(),
        }
    }

    fn get_rig(&self) -> &EnemyRig {
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
        physics_world: &PhysicsWorld,
    ) -> bool {
        match self {
            Self::Fighter(fighter) => {
                fighter.update(player, enemy, activity, physics_world);
                false
            }
            Self::Gunner(gunner) => gunner.update(player, enemy, activity, physics_world),
        }
    }
}

#[derive(Clone)]
struct EnemyRig {
    scene: &'static str,
    animation_source: &'static str,
    idle_animation: usize,
    moving_animation: usize,
    attack_animation: usize,
    weapon: EquippedWeapon,
}

#[derive(Clone)]
struct EquippedWeapon {
    weapon: Weapon,
    transform: Transform,
}

fn flatten_direction(player: &Transform, enemy: &Transform) -> Option<(Vec3, f32)> {
    let offset = player.translation - enemy.translation;
    let planar_offset = Vec3::new(offset.x, 0.0, offset.z);
    let distance = planar_offset.length();
    (distance > 0.0).then(|| (planar_offset / distance, distance))
}

#[derive(Component, Default)]
pub struct EnemyActivity {
    pub state: AnimationState,
    attack_cooldown: f32,
}

impl EnemyActivity {
    fn attack_on_cooldown(&mut self, cooldown_seconds: f32, animation_seconds: f32) -> bool {
        let attack_started = self.attack_cooldown <= 0.0;
        if attack_started {
            self.attack_cooldown = cooldown_seconds;
        }
        let attacking = self.attack_cooldown > cooldown_seconds - animation_seconds;
        self.state = if attacking {
            AnimationState::Attacking
        } else {
            AnimationState::Idle
        };
        attack_started
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
    sequence_number: usize,
    transform: Transform,
    alive: bool,
) -> Entity {
    let rig = kind.get_rig().clone();
    let mut enemy = commands.spawn((
        Enemy,
        Name::new(format!("{} {}", kind.get_display_name(), sequence_number + 1)),
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
                weapon: asset_server
                    .load(GltfAssetLabel::Scene(0).from_asset(rig.weapon.weapon.get_model_asset_path())),
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

fn kill_enemy_on_hit(
    hit: On<EnemyHit>,
    mut commands: Commands,
    armatures: Query<&EnemyArmature>,
    mut animation_players: Query<&mut AnimationPlayer>,
    enemies: Query<(&EnemyKind, &EnemyWeapon)>,
    weapon_transforms: Query<&GlobalTransform>,
    alive_enemies: Query<(), AliveEnemy>,
) {
    if !alive_enemies.contains(hit.entity) {
        return;
    }

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
    let collider_half_extents = kind.get_rig().weapon.weapon.get_collider_half_extents();

    let (scale, rotation, translation) = global.to_scale_rotation_translation();
    commands.entity(weapon.0).remove::<ChildOf>();
    commands.entity(weapon.0).insert((
        OwnedByCharacter(enemy),
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
        attach_weapon(&mut commands, enemy_entity, bones, &animations, &kind.get_rig().weapon);
    }
}

fn attach_weapon(
    commands: &mut Commands,
    enemy_entity: Entity,
    bones: &BoneMap,
    animations: &EnemyAnimations,
    weapon: &EquippedWeapon,
) {
    let Some(right_hand) = bones.get("hand_r") else {
        return;
    };

    let weapon_entity = commands
        .spawn((
            Name::new("Weapon"),
            WorldAssetRoot(animations.weapon.clone()),
            weapon.weapon,
            weapon.transform,
        ))
        .id();

    observe_muzzle_flash(commands, weapon_entity);
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
    enemies: Query<(&EnemyActivity, &EnemyArmature), Without<Dying>>,
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
const GUNNER_ACCURACY: f64 = 0.65;
const GUNNER_MISS_DISTANCE: f32 = 0.75;

#[derive(SystemParam)]
struct EnemyMovementContext<'w, 's> {
    current_map: Res<'w, CurrentMap>,
    time: Res<'w, Time>,
    move_and_slide: MoveAndSlide<'w, 's>,
    gravity: Res<'w, Gravity>,
}

fn update_behaviour(
    movement_context: EnemyMovementContext,
    player: Query<&Transform, (With<Player>, Without<Enemy>)>,
    mut enemies: Query<EnemyBehavior, (AliveEnemy, Without<Player>)>,
    enemy_weapons: Query<&EnemyWeapon>,
    weapons: Query<(&Weapon, &GlobalTransform)>,
    owned_bodies: Query<(Entity, &OwnedByCharacter)>,
    mut weapon_fired: MessageWriter<WeaponFired>,
) {
    if *movement_context.current_map == CurrentMap::ColliderInspection {
        return;
    }
    let Ok(player_transform) = player.single() else {
        return;
    };
    let delta_secs = movement_context.time.delta_secs();

    for (enemy_entity, kind, mut enemy_transform, mut activity, mut velocity) in &mut enemies {
        activity.attack_cooldown -= delta_secs;
        let filter = exclude_character_bodies(&[enemy_entity], &owned_bodies);
        let physics_world = PhysicsWorld::new(
            &movement_context.move_and_slide,
            movement_context.gravity.0.y,
            delta_secs,
            &filter,
            ENEMY_COLLISION_RADIUS,
            ENEMY_COLLISION_CENTER_HEIGHT,
            0.0,
        );
        let gunner_fired = move_enemy(
            &physics_world,
            player_transform,
            kind,
            &mut enemy_transform,
            &mut activity,
            &mut velocity,
        );
        if !gunner_fired {
            continue;
        }
        let Ok(enemy_weapon) = enemy_weapons.get(enemy_entity) else {
            continue;
        };
        let Ok((weapon, weapon_transform)) = weapons.get(enemy_weapon.0) else {
            continue;
        };
        let origin = weapon_transform.translation();
        let target = calculate_gunner_target(player_transform.translation);
        let Some(direction) = Dir3::new(target - origin).ok() else {
            continue;
        };
        weapon_fired.write(WeaponFired {
            shooter_entity: Some(enemy_entity),
            weapon_entity: enemy_weapon.0,
            weapon: *weapon,
            origin,
            direction,
        });
    }
}

fn move_enemy(
    physics_world: &PhysicsWorld,
    player: &Transform,
    kind: &EnemyKind,
    enemy_transform: &mut Transform,
    activity: &mut EnemyActivity,
    velocity: &mut LinearVelocity,
) -> bool {
    let previous_translation = enemy_transform.translation;
    let gunner_fired = kind.update(player, enemy_transform, activity, physics_world);
    let desired_translation = enemy_transform.translation - previous_translation;
    enemy_transform.translation = previous_translation + physics_world.slide(previous_translation, desired_translation);
    physics_world.integrate_fall(velocity, &mut enemy_transform.translation);
    let horizontal_travel = Vec3::new(
        enemy_transform.translation.x - previous_translation.x,
        0.0,
        enemy_transform.translation.z - previous_translation.z,
    );
    let delta_secs = physics_world.get_delta_secs();
    let horizontal_run_velocity = if delta_secs > 0.0 {
        horizontal_travel / delta_secs
    } else {
        Vec3::ZERO
    };
    *velocity = LinearVelocity(Vec3::new(
        horizontal_run_velocity.x,
        velocity.y,
        horizontal_run_velocity.z,
    ));
    gunner_fired
}

fn calculate_gunner_target(player_position: Vec3) -> Vec3 {
    let mut random_number_generator = rand::rng();
    if random_number_generator.random_bool(GUNNER_ACCURACY) {
        return player_position;
    }
    let horizontal_shift = Vec2::new(
        random_number_generator.random_range(-GUNNER_MISS_DISTANCE..=GUNNER_MISS_DISTANCE),
        random_number_generator.random_range(-GUNNER_MISS_DISTANCE..=GUNNER_MISS_DISTANCE),
    );
    player_position + Vec3::new(horizontal_shift.x, horizontal_shift.y, 0.0)
}
