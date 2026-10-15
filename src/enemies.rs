use avian3d::prelude::{Collider, LinearVelocity, Position, RigidBody, Rotation};
use bevy::animation::{AnimatedBy, AnimationTargetId};
use bevy::prelude::*;
use bevy::world_serialization::{WorldAsset, WorldInstanceReady};
use rand::Rng;

use crate::bloodmarks::BloodMarks;
use crate::health::{CharacterDamaged, CharacterDied, Health};
use crate::movement::PhysicsWorld;
use crate::player::Player;
use crate::ragdoll::{BoneMap, OwnedByCharacter, RagdollBodyPart, RagdollData, setup_ragdoll};
use crate::shooting::{Magazine, Weapon, observe_muzzle_flash};
use crate::state::GameState;

mod fighter;
mod gunner;

pub struct EnemiesPlugin;

impl Plugin for EnemiesPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins((fighter::FighterPlugin, gunner::GunnerPlugin))
            .add_systems(Update, kill_enemy_on_death.run_if(in_state(GameState::Playing)))
            .add_systems(
                PostUpdate,
                (
                    setup_external_animation,
                    dance_when_player_is_dead,
                    update_enemy_animations,
                )
                    .chain(),
            );
    }
}

#[derive(Component)]
pub struct Enemy;

#[derive(Component)]
pub struct Dying;

#[derive(EntityEvent)]
pub(crate) struct EnemyHit {
    #[event_target]
    pub(crate) target: Entity,
    pub(crate) collider: Entity,
    pub(crate) shot_origin: Vec3,
    pub(crate) shot_direction: Dir3,
    pub(crate) impact_distance: f32,
    pub(crate) impulse: Vec3,
    pub(crate) weapon_caliber: f32,
}

#[derive(Clone, Copy)]
pub enum EnemyKind {
    Fighter,
    Gunner,
}

#[derive(Component, Clone)]
pub(super) struct EnemyRig {
    scene: &'static str,
    animation_source: &'static str,
    idle_animation: usize,
    moving_animation: usize,
    attack_animation: usize,
    reload_animation: Option<usize>,
    dance_animation: usize,
    idle_animation_seconds: f32,
    moving_animation_seconds: f32,
    dance_animation_seconds: f32,
    weapon: EquippedWeapon,
}

#[derive(Clone)]
pub(super) struct EquippedWeapon {
    pub(crate) weapon: Weapon,
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
    pub(super) pending_melee_hit: Option<f32>,
    pub(super) pending_melee_damage: f32,
    pub(super) pending_melee_range: f32,
}

impl EnemyActivity {
    pub(super) fn attack_on_cooldown(&mut self, cooldown_seconds: f32, animation_seconds: f32) -> bool {
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
    Reloading,
    Dancing,
}

#[derive(Component, Clone, Copy)]
pub(crate) struct Seed(u64);

impl Seed {
    pub(crate) fn calculate_unit_value(self, salt: u64) -> f32 {
        let mixed_seed = self.0.wrapping_add(salt).wrapping_mul(0x9E37_79B9_7F4A_7C15);
        let mixed_seed = (mixed_seed ^ (mixed_seed >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        let mixed_seed = (mixed_seed ^ (mixed_seed >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        let random_bits = mixed_seed ^ (mixed_seed >> 31);
        (random_bits >> 40) as f32 / ((1_u32 << 24) - 1) as f32
    }
}

#[derive(Clone, Component)]
struct EnemyAnimations {
    idle: Handle<AnimationClip>,
    moving: Handle<AnimationClip>,
    attack: Handle<AnimationClip>,
    reload: Option<Handle<AnimationClip>>,
    dance: Handle<AnimationClip>,
    weapon: Handle<WorldAsset>,
}

#[derive(Component)]
struct ArmatureAnimation {
    idle: AnimationNodeIndex,
    moving: AnimationNodeIndex,
    attacking: AnimationNodeIndex,
    reloading: Option<AnimationNodeIndex>,
    dancing: AnimationNodeIndex,
    idle_phase_seconds: f32,
    moving_phase_seconds: f32,
    dance_phase_seconds: f32,
    idle_playback_speed: f32,
    moving_playback_speed: f32,
    current: Option<AnimationState>,
}

#[derive(Component)]
struct EnemyArmature(Entity);

#[derive(Component)]
pub(crate) struct ManualAnimationControl;

#[derive(Component)]
struct EnemyWeapon(Entity);

pub(crate) fn spawn_enemy(
    commands: &mut Commands,
    asset_server: &AssetServer,
    kind: EnemyKind,
    sequence_number: usize,
    transform: Transform,
    alive: bool,
) -> Entity {
    let seed = Seed(rand::rng().random());
    match kind {
        EnemyKind::Fighter => fighter::spawn(commands, asset_server, seed, sequence_number, transform, alive),
        EnemyKind::Gunner => gunner::spawn(commands, asset_server, seed, sequence_number, transform, alive),
    }
}

pub(super) struct EnemyDefinition {
    pub(super) display_name: &'static str,
    pub(super) rig: EnemyRig,
    pub(super) starting_health: f32,
}

pub(super) fn spawn_enemy_base(
    commands: &mut Commands,
    asset_server: &AssetServer,
    definition: EnemyDefinition,
    seed: Seed,
    sequence_number: usize,
    transform: Transform,
    alive: bool,
) -> Entity {
    let rig = definition.rig.clone();
    let mut enemy = commands.spawn((
        Enemy,
        seed,
        Name::new(format!("{} {}", definition.display_name, sequence_number + 1)),
        WorldAssetRoot(asset_server.load(GltfAssetLabel::Scene(0).from_asset(definition.rig.scene))),
        definition.rig,
        transform,
        Health::new(definition.starting_health),
        BloodMarks::default(),
    ));
    if alive {
        enemy.insert((
            EnemyAnimations {
                idle: asset_server.load(GltfAssetLabel::Animation(rig.idle_animation).from_asset(rig.animation_source)),
                moving: asset_server
                    .load(GltfAssetLabel::Animation(rig.moving_animation).from_asset(rig.animation_source)),
                attack: asset_server
                    .load(GltfAssetLabel::Animation(rig.attack_animation).from_asset(rig.animation_source)),
                reload: rig.reload_animation.map(|animation_index| {
                    asset_server.load(GltfAssetLabel::Animation(animation_index).from_asset(rig.animation_source))
                }),
                dance: asset_server
                    .load(GltfAssetLabel::Animation(rig.dance_animation).from_asset(rig.animation_source)),
                weapon: asset_server
                    .load(GltfAssetLabel::Scene(0).from_asset(rig.weapon.weapon.get_model_asset_path())),
            },
            EnemyActivity::default(),
            LinearVelocity::default(),
        ));
    } else {
        enemy.insert(Dying);
    }
    enemy.observe(setup_ragdoll).observe(prepare_enemy_animation).id()
}

fn kill_enemy_on_death(
    mut deaths: MessageReader<CharacterDied>,
    mut commands: Commands,
    armatures: Query<&EnemyArmature>,
    mut animation_players: Query<&mut AnimationPlayer>,
    enemies: Query<(&EnemyRig, &EnemyWeapon)>,
    weapon_transforms: Query<&GlobalTransform>,
    alive_enemies: Query<(), AliveEnemy>,
) {
    for death in deaths.read() {
        if !alive_enemies.contains(death.character) {
            continue;
        }
        kill_enemy(
            death.character,
            &mut commands,
            &armatures,
            &mut animation_players,
            &enemies,
            &weapon_transforms,
        );
    }
}

fn kill_enemy(
    enemy: Entity,
    commands: &mut Commands,
    armatures: &Query<&EnemyArmature>,
    animation_players: &mut Query<&mut AnimationPlayer>,
    enemies: &Query<(&EnemyRig, &EnemyWeapon)>,
    weapon_transforms: &Query<&GlobalTransform>,
) {
    commands.entity(enemy).insert(Dying);
    if let Ok(mut animation_player) = armatures
        .get(enemy)
        .and_then(|armature| animation_players.get_mut(armature.0))
    {
        animation_player.stop_all();
    }
    drop_weapon(enemy, commands, enemies, weapon_transforms);
}

fn drop_weapon(
    enemy: Entity,
    commands: &mut Commands,
    enemies: &Query<(&EnemyRig, &EnemyWeapon)>,
    weapon_transforms: &Query<&GlobalTransform>,
) {
    let Ok((rig, weapon)) = enemies.get(enemy) else {
        return;
    };
    let Ok(global) = weapon_transforms.get(weapon.0) else {
        return;
    };
    let collider_half_extents = rig.weapon.weapon.get_collider_half_extents();

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
    pending: Query<(Entity, &PendingAnimationSetup, &EnemyRig, &Seed)>,

    mut graphs: ResMut<Assets<AnimationGraph>>,
    players: Query<&AnimationPlayer>,
) {
    for (enemy_entity, setup, rig, seed) in &pending {
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
        let reloading = animations
            .reload
            .as_ref()
            .map(|reload_animation| graph.add_clip(reload_animation.clone(), 1.0, graph.root));
        let dancing = graph.add_clip(animations.dance.clone(), 1.0, graph.root);
        let graph_handle = graphs.add(graph);
        let idle_phase_seconds = seed.calculate_unit_value(1) * rig.idle_animation_seconds;
        let moving_phase_seconds = seed.calculate_unit_value(2) * rig.moving_animation_seconds;
        let dance_phase_seconds = seed.calculate_unit_value(3) * rig.dance_animation_seconds;
        let playback_speed_jitter = 0.1;
        let idle_playback_speed = 1.0 + (seed.calculate_unit_value(4) * 2.0 - 1.0) * playback_speed_jitter;
        let moving_playback_speed = 1.0 + (seed.calculate_unit_value(5) * 2.0 - 1.0) * playback_speed_jitter;

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
                reloading,
                dancing,
                idle_phase_seconds,
                moving_phase_seconds,
                dance_phase_seconds,
                idle_playback_speed,
                moving_playback_speed,
                current: None,
            },
        ));

        commands
            .entity(enemy_entity)
            .insert(EnemyArmature(armature))
            .remove::<PendingAnimationSetup>();
        attach_weapon(&mut commands, enemy_entity, bones, &animations, &rig.weapon);
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

    let mut weapon_entity = commands.spawn((
        Name::new("Weapon"),
        WorldAssetRoot(animations.weapon.clone()),
        weapon.weapon,
        weapon.transform,
    ));
    if weapon.weapon.get_magazine_capacity().is_some() {
        weapon_entity.insert(Magazine::create_full(weapon.weapon));
    }
    let weapon_entity = weapon_entity.id();

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

fn dance_when_player_is_dead(
    players: Query<(), (With<Player>, With<crate::player::PlayerDead>)>,
    mut enemies: Query<&mut EnemyActivity, (With<Enemy>, Without<Dying>)>,
) {
    if players.is_empty() {
        return;
    }
    for mut activity in &mut enemies {
        activity.state = AnimationState::Dancing;
    }
}

type EnemyAnimationQuery<'world, 'state> = Query<
    'world,
    'state,
    (&'static EnemyActivity, &'static EnemyArmature),
    (Without<Dying>, Without<ManualAnimationControl>),
>;

fn update_enemy_animations(
    enemies: EnemyAnimationQuery,
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
            AnimationState::Reloading => armature_animation.reloading.unwrap_or(armature_animation.idle),
            AnimationState::Dancing => armature_animation.dancing,
        };
        let (phase_seconds, playback_speed, repeats) = match activity.state {
            AnimationState::Idle => (
                armature_animation.idle_phase_seconds,
                armature_animation.idle_playback_speed,
                true,
            ),
            AnimationState::Moving => (
                armature_animation.moving_phase_seconds,
                armature_animation.moving_playback_speed,
                true,
            ),
            AnimationState::Attacking | AnimationState::Reloading => (0.0, 1.0, false),
            AnimationState::Dancing => (armature_animation.dance_phase_seconds, 1.0, true),
        };

        player.stop_all();
        let active_animation = player.start(node_index);
        active_animation.set_seek_time(phase_seconds).set_speed(playback_speed);
        if repeats {
            active_animation.repeat();
        }
        armature_animation.current = Some(activity.state);
    }
}

pub(crate) type AliveEnemy = (With<Enemy>, Without<Dying>);
pub(super) const ENEMY_COLLISION_RADIUS: f32 = 0.35;
pub(super) const ENEMY_COLLISION_CENTER_HEIGHT: f32 = 1.0;

pub(super) fn move_enemy(
    physics_world: &PhysicsWorld,
    enemy_transform: &mut Transform,
    activity: &mut EnemyActivity,
    velocity: &mut LinearVelocity,
    update_enemy: impl FnOnce(&mut Transform, &mut EnemyActivity, &PhysicsWorld) -> bool,
) -> bool {
    let previous_translation = enemy_transform.translation;
    let attack_started = update_enemy(enemy_transform, activity, physics_world);
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
    attack_started
}

pub(super) fn deliver_pending_melee_hit(
    enemy_entity: Entity,
    player_entity: Entity,
    player_transform: &Transform,
    enemy_transform: &mut Transform,
    activity: &mut EnemyActivity,
    delta_secs: f32,
    character_damaged: &mut MessageWriter<CharacterDamaged>,
) {
    let Some(remaining) = activity.pending_melee_hit.as_mut() else {
        return;
    };
    *remaining -= delta_secs;
    if *remaining > 0.0 {
        return;
    }
    activity.pending_melee_hit = None;
    let weapon_range = activity.pending_melee_range;
    if flatten_direction(player_transform, enemy_transform).is_some_and(|(_, distance)| distance <= weapon_range) {
        let damage = activity.pending_melee_damage;
        character_damaged.write(CharacterDamaged {
            character: player_entity,
            amount: damage,
            attacker: Some(enemy_entity),
            body_part: None,
        });
    }
}

pub(super) fn find_player_head_position(
    player_entity: Entity,
    player_children: &Query<&Children, With<Player>>,
    ragdoll_roots: &Query<&RagdollData>,
    ragdoll_body_positions: &Query<&Position, With<RagdollBodyPart>>,
) -> Option<Vec3> {
    let player_children = player_children.get(player_entity).ok()?;
    let ragdoll_root = player_children.iter().find(|child| ragdoll_roots.contains(*child))?;
    let head_body = ragdoll_roots.get(ragdoll_root).ok()?.find_part(RagdollBodyPart::Head)?;
    Some(ragdoll_body_positions.get(head_body).ok()?.0)
}
