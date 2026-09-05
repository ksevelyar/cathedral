use avian3d::prelude::{Collider, Position, RigidBody, Rotation};
use bevy::animation::{AnimatedBy, AnimationTargetId};
use bevy::prelude::*;
use bevy::world_serialization::{WorldAsset, WorldInstanceReady};

use crate::player::Player;
use crate::ragdoll::setup_ragdoll;
use crate::state::GameState;

pub struct EnemiesPlugin;

impl Plugin for EnemiesPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<EnemyBehaviorEnabled>()
            .add_systems(Update, update_enemy_behavior.run_if(in_state(GameState::Playing)))
            .add_systems(
                PostUpdate,
                (
                    stop_enemy_animation,
                    drop_weapon_on_death,
                    setup_external_animation,
                    update_enemy_animations,
                ),
            );
    }
}

#[derive(Resource, Default)]
struct EnemyBehaviorEnabled;

#[derive(Component)]
pub struct Enemy;

#[derive(Component)]
pub struct Dying;

#[derive(Component, Clone, Copy, Debug, PartialEq, Eq)]
pub enum EnemyKind {
    Fighter,
    Gunner,
}

const ENEMY_MOVE_SPEED: f32 = 1.0;
const FIGHTER_REACH_DISTANCE: f32 = 2.5;
const GUNNER_MIN_DISTANCE: f32 = 4.0;
const GUNNER_MAX_DISTANCE: f32 = 12.0;
const ATTACK_COOLDOWN: f32 = 2.0;

#[derive(Clone, Component)]
struct EnemyAnimations {
    idle: Handle<AnimationClip>,
    move_: Handle<AnimationClip>,
    attack: Handle<AnimationClip>,
    weapon: Handle<WorldAsset>,
}

#[derive(Component, Clone, Copy, Default, PartialEq, Eq)]
enum AnimationState {
    #[default]
    Idle,
    Moving,
    Attacking,
}

#[derive(Component)]
struct PlayingAnimationState(Option<AnimationState>);

#[derive(Component)]
struct AttackCooldownTimer(f32);

#[derive(Component)]
struct EnemyAnimationGraphs {
    idle_index: AnimationNodeIndex,
    move_index: AnimationNodeIndex,
    attack_index: AnimationNodeIndex,
}

struct EnemyDefinition {
    scene_path: &'static str,
    animation_source_path: &'static str,
    idle_animation_index: usize,
    move_animation_index: usize,
    attack_animation_index: usize,
    attack_animation_seconds: f32,
    weapon: WeaponAttachment,
}

struct WeaponAttachment {
    path: &'static str,
    scale: f32,
    rotation_euler_yxz: (f32, f32, f32),
    translation: (f32, f32, f32),
    collider_half_extents: (f32, f32, f32),
}

const FIGHTER_ATTACK_ANIMATION_SECONDS: f32 = 1.53;
const GUNNER_ATTACK_ANIMATION_SECONDS: f32 = 1.0;

impl EnemyKind {
    fn definition(self) -> EnemyDefinition {
        match self {
            Self::Fighter => EnemyDefinition {
                scene_path: "animation/Unreal-Godot/UAL1_Standard.glb",
                animation_source_path: "animation/Unreal-Godot/UAL1_Standard.glb",
                idle_animation_index: 40,
                move_animation_index: 36,
                attack_animation_index: 39,
                attack_animation_seconds: FIGHTER_ATTACK_ANIMATION_SECONDS,
                weapon: WeaponAttachment {
                    path: "weapon/katana.glb",
                    scale: 1.0,
                    rotation_euler_yxz: (0.0, 0.0, 0.0),
                    translation: (0.0, 0.0, 0.0),
                    collider_half_extents: (0.01, 0.04, 0.35),
                },
            },
            Self::Gunner => EnemyDefinition {
                scene_path: "universal-base-characters/Base Characters/Godot - UE/Superhero_Female_FullBody.gltf",
                animation_source_path: "animation/Unreal-Godot/UAL1_Standard.glb",
                idle_animation_index: 21,
                move_animation_index: 36,
                attack_animation_index: 23,
                attack_animation_seconds: GUNNER_ATTACK_ANIMATION_SECONDS,
                weapon: WeaponAttachment {
                    path: "weapon/pistol.glb",
                    scale: 0.15,
                    rotation_euler_yxz: (1.5240, 0.0217, 1.3841),
                    translation: (-0.05, 0.0, 0.0),
                    collider_half_extents: (0.88, 0.15, 0.1),
                },
            },
        }
    }
}

#[derive(Clone, Copy)]
pub enum EnemyLifeState {
    Alive,
    Dead,
}

pub struct EnemySpawn {
    pub kind: EnemyKind,
    pub transform: Transform,
    pub life_state: EnemyLifeState,
}

pub fn spawn_enemy(commands: &mut Commands, asset_server: &AssetServer, spawn: EnemySpawn) -> Entity {
    let definition = spawn.kind.definition();
    let mut enemy = commands.spawn((
        Enemy,
        WorldAssetRoot(asset_server.load(GltfAssetLabel::Scene(0).from_asset(definition.scene_path))),
        spawn.transform,
        spawn.kind,
    ));
    match spawn.life_state {
        EnemyLifeState::Alive => {
            enemy.insert((
                EnemyAnimations {
                    idle: asset_server.load(
                        GltfAssetLabel::Animation(definition.idle_animation_index)
                            .from_asset(definition.animation_source_path),
                    ),
                    move_: asset_server.load(
                        GltfAssetLabel::Animation(definition.move_animation_index)
                            .from_asset(definition.animation_source_path),
                    ),
                    attack: asset_server.load(
                        GltfAssetLabel::Animation(definition.attack_animation_index)
                            .from_asset(definition.animation_source_path),
                    ),
                    weapon: asset_server.load(GltfAssetLabel::Scene(0).from_asset(spawn.kind.definition().weapon.path)),
                },
                AnimationState::default(),
                AttackCooldownTimer(0.0),
            ));
        }
        EnemyLifeState::Dead => {
            enemy.insert(Dying);
        }
    }
    enemy.observe(setup_ragdoll).observe(prepare_enemy_animation).id()
}

#[derive(Component)]
struct PendingAnimationSetup {
    animations: EnemyAnimations,
}

fn prepare_enemy_animation(
    scene_ready: On<WorldInstanceReady>,
    mut commands: Commands,
    enemy_animations: Query<&EnemyAnimations>,
    behavior_enabled: Option<Res<EnemyBehaviorEnabled>>,
) {
    if behavior_enabled.is_none() {
        return;
    }
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
    for (enemy_entity, setup, enemy_kind) in &pending {
        let Some(armature) = children
            .iter_descendants(enemy_entity)
            .find(|&descendant| names.get(descendant).is_ok_and(|name| name.as_str() == "Armature"))
        else {
            continue;
        };

        let animations = setup.animations.clone();
        let (mut graph, idle_index) = AnimationGraph::from_clip(animations.idle.clone());
        let move_index = graph.add_clip(animations.move_.clone(), 1.0, graph.root);
        let attack_index = graph.add_clip(animations.attack.clone(), 1.0, graph.root);
        let graph_handle = graphs.add(graph);
        let animation_graphs = EnemyAnimationGraphs {
            idle_index,
            move_index,
            attack_index,
        };

        if players.get(armature).is_err() {
            commands.entity(armature).insert(AnimationPlayer::default());
            build_animation_targets(&mut commands, &children, &names, armature, armature, vec![]);
        }
        commands.entity(armature).insert((
            AnimationGraphHandle(graph_handle.clone()),
            animation_graphs,
            PlayingAnimationState(None),
        ));

        commands.entity(enemy_entity).remove::<PendingAnimationSetup>();
        attach_weapon(
            &mut commands,
            enemy_entity,
            &children,
            &names,
            &animations,
            &enemy_kind.definition().weapon,
        );
    }
}

fn attach_weapon(
    commands: &mut Commands,
    enemy_entity: Entity,
    children: &Query<&Children>,
    names: &Query<&Name>,
    animations: &EnemyAnimations,
    weapon: &WeaponAttachment,
) {
    let Some(right_hand) = children
        .iter_descendants(enemy_entity)
        .find(|&descendant| names.get(descendant).is_ok_and(|name| name.as_str() == "hand_r"))
    else {
        return;
    };

    let (yaw, pitch, roll) = weapon.rotation_euler_yxz;
    let (x, y, z) = weapon.collider_half_extents;
    let weapon_entity = commands
        .spawn((
            Name::new("Weapon"),
            WorldAssetRoot(animations.weapon.clone()),
            WeaponColliderHalfExtents(Vec3::new(x, y, z)),
            Transform {
                translation: weapon.translation.into(),
                rotation: Quat::from_euler(EulerRot::YXZ, yaw, pitch, roll),
                scale: Vec3::splat(weapon.scale),
            },
        ))
        .id();

    commands.entity(right_hand).add_children(&[weapon_entity]);
}

#[derive(Component)]
struct WeaponColliderHalfExtents(Vec3);

fn drop_weapon_on_death(
    mut commands: Commands,
    children: Query<&Children>,
    names: Query<&Name>,
    dying_enemies: Query<Entity, Added<Dying>>,
    global_transforms: Query<&GlobalTransform>,
    colliders: Query<&WeaponColliderHalfExtents>,
) {
    for enemy in &dying_enemies {
        for descendant in children.iter_descendants(enemy) {
            if !names.get(descendant).is_ok_and(|name| name.as_str() == "Weapon") {
                continue;
            }
            let Ok(collider) = colliders.get(descendant) else {
                continue;
            };
            let Ok(global) = global_transforms.get(descendant) else {
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
                Collider::cuboid(collider.0.x, collider.0.y, collider.0.z),
                Position(translation),
                Rotation(rotation),
            ));
        }
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

fn stop_enemy_animation(
    children: Query<&Children>,
    dying_enemies: Query<Entity, Added<Dying>>,
    mut players: Query<&mut AnimationPlayer>,
) {
    for enemy in &dying_enemies {
        for descendant in children.iter_descendants(enemy) {
            if let Ok(mut player) = players.get_mut(descendant) {
                player.stop_all();
            }
        }
    }
}

fn update_enemy_animations(
    children: Query<&Children>,
    names: Query<&Name>,
    enemies: Query<(Entity, &AnimationState)>,
    mut armatures: Query<(&EnemyAnimationGraphs, &mut PlayingAnimationState, &mut AnimationPlayer)>,
) {
    for (enemy_entity, desired_state) in &enemies {
        let Some(armature) = children
            .iter_descendants(enemy_entity)
            .find(|&descendant| names.get(descendant).is_ok_and(|name| name.as_str() == "Armature"))
        else {
            continue;
        };

        let Ok((animation_graphs, mut playing_state, mut player)) = armatures.get_mut(armature) else {
            continue;
        };

        if playing_state.0 == Some(*desired_state) {
            continue;
        }

        let node_index = match desired_state {
            AnimationState::Idle => animation_graphs.idle_index,
            AnimationState::Moving => animation_graphs.move_index,
            AnimationState::Attacking => animation_graphs.attack_index,
        };

        player.stop_all();
        player.start(node_index).repeat();
        playing_state.0 = Some(*desired_state);
    }
}

type AliveEnemy = (With<Enemy>, Without<Dying>, Without<Player>);

fn update_enemy_behavior(
    time: Res<Time>,
    player: Query<&Transform, (With<Player>, Without<Enemy>)>,
    mut enemies: Query<
        (
            &EnemyKind,
            &mut Transform,
            &mut AnimationState,
            &mut AttackCooldownTimer,
        ),
        AliveEnemy,
    >,
) {
    let Ok(player_transform) = player.single() else {
        return;
    };

    for (kind, mut enemy_transform, mut anim_state, mut cooldown) in &mut enemies {
        cooldown.0 -= time.delta_secs();
        let definition = kind.definition();

        let offset = player_transform.translation - enemy_transform.translation;
        let planar_offset = Vec3::new(offset.x, 0.0, offset.z);
        let distance = planar_offset.length();

        match kind {
            EnemyKind::Fighter => {
                if distance <= FIGHTER_REACH_DISTANCE {
                    if cooldown.0 <= 0.0 {
                        cooldown.0 = ATTACK_COOLDOWN;
                    }
                    let attacking = cooldown.0 > ATTACK_COOLDOWN - definition.attack_animation_seconds;
                    let desired_state = if attacking {
                        AnimationState::Attacking
                    } else {
                        AnimationState::Idle
                    };
                    anim_state.set_if_neq(desired_state);
                } else {
                    anim_state.set_if_neq(AnimationState::Moving);
                    let direction = planar_offset / distance;
                    enemy_transform.look_to(-direction, Vec3::Y);
                    let available_distance = distance - FIGHTER_REACH_DISTANCE;
                    let movement = (ENEMY_MOVE_SPEED * time.delta_secs()).min(available_distance);
                    enemy_transform.translation += direction * movement;
                }
            }
            EnemyKind::Gunner => {
                if distance < GUNNER_MIN_DISTANCE {
                    let away_direction = -(planar_offset / distance);
                    enemy_transform.look_to(-away_direction, Vec3::Y);
                    let movement = ENEMY_MOVE_SPEED * time.delta_secs();
                    enemy_transform.translation += away_direction * movement;
                    anim_state.set_if_neq(AnimationState::Moving);
                } else if distance > GUNNER_MAX_DISTANCE {
                    let toward_direction = planar_offset / distance;
                    enemy_transform.look_to(-toward_direction, Vec3::Y);
                    let movement = ENEMY_MOVE_SPEED * time.delta_secs();
                    enemy_transform.translation += toward_direction * movement;
                    anim_state.set_if_neq(AnimationState::Moving);
                } else {
                    let direction = planar_offset / distance;
                    enemy_transform.look_to(-direction, Vec3::Y);
                    if cooldown.0 <= 0.0 {
                        cooldown.0 = ATTACK_COOLDOWN;
                    }
                    let attacking = cooldown.0 > ATTACK_COOLDOWN - definition.attack_animation_seconds;
                    let desired_state = if attacking {
                        AnimationState::Attacking
                    } else {
                        AnimationState::Idle
                    };
                    anim_state.set_if_neq(desired_state);
                }
            }
        }
    }
}
