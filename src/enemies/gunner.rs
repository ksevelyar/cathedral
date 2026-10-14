use avian3d::prelude::{Gravity, LinearVelocity, MoveAndSlide, Position};
use bevy::ecs::system::SystemParam;
use bevy::prelude::*;
use rand::Rng;

use super::{
    AliveEnemy, AnimationSeed, AnimationState, ENEMY_COLLISION_CENTER_HEIGHT, ENEMY_COLLISION_RADIUS, EnemyActivity,
    EnemyDefinition, EnemyRig, EnemyWeapon, EquippedWeapon, find_player_head_position, flatten_direction, move_enemy,
    spawn_enemy_base,
};
use crate::movement::PhysicsWorld;
use crate::player::Player;
use crate::ragdoll::{OwnedByCharacter, RagdollBodyPart, RagdollData, exclude_character_bodies};
use crate::shooting::{Magazine, RangedWeapon, Weapon, WeaponFired, WeaponFiringSystems, WeaponReloadStarted};
use crate::state::ColliderInspection;

const GUNNER_ACCURACY: f64 = 0.65;
const GUNNER_MISS_DISTANCE: f32 = 0.75;
const PISTOL_SHOOT_ANIMATION_SECONDS: f32 = 0.633;

pub(super) struct GunnerPlugin;

impl Plugin for GunnerPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Update,
            update_gunners
                .in_set(WeaponFiringSystems)
                .run_if(in_state(crate::state::GameState::Playing))
                .run_if(collider_inspection_is_disabled),
        );
    }
}

#[derive(Component, Clone)]
pub(super) struct Gunner {
    rig: EnemyRig,
    move_speed: f32,
    flee_distance: f32,
    attack_distance: f32,
    attack_cooldown: f32,
    attack_animation_seconds: f32,
    damage_coefficient: f32,
    starting_health: f32,
}

impl Default for Gunner {
    fn default() -> Self {
        Self {
            rig: EnemyRig {
                scene: "characters/bodies/female.gltf",
                animation_source: "characters/animations/ual1-standard.glb",
                idle_animation: 21,
                moving_animation: 36,
                attack_animation: 23,
                reload_animation: Some(22),
                dance_animation: 3,
                idle_animation_seconds: 1.667,
                moving_animation_seconds: 0.667,
                dance_animation_seconds: 1.0,
                weapon: EquippedWeapon {
                    weapon: Weapon::Ranged(RangedWeapon::PISTOL),
                    transform: Transform {
                        translation: Vec3::new(-0.05, 0.0, 0.0),
                        rotation: Quat::from_euler(EulerRot::YXZ, 1.5240, 0.0217, 1.3841),
                        scale: Vec3::splat(0.15),
                    },
                },
            },
            move_speed: 4.0,
            flee_distance: 4.0,
            attack_distance: 12.0,
            attack_cooldown: 4.0,
            attack_animation_seconds: PISTOL_SHOOT_ANIMATION_SECONDS,
            damage_coefficient: 0.5,
            starting_health: 20.0,
        }
    }
}

pub(super) fn spawn(
    commands: &mut Commands,
    asset_server: &AssetServer,
    sequence_number: usize,
    transform: Transform,
    alive: bool,
) -> Entity {
    let gunner = Gunner::default();
    let definition = EnemyDefinition {
        display_name: "Gunner",
        rig: gunner.rig.clone(),
        starting_health: gunner.starting_health,
    };
    let gunner_entity = spawn_enemy_base(commands, asset_server, definition, sequence_number, transform, alive);
    let animation_seed = AnimationSeed(sequence_number as u64);
    commands.entity(gunner_entity).insert((
        gunner,
        GunnerAttack::default(),
        GunnerAttackJitter(animation_seed.calculate_unit_value(6)),
    ));
    gunner_entity
}

type GunnerMovementContext<'world, 'state> = (Res<'world, Time>, MoveAndSlide<'world, 'state>, Res<'world, Gravity>);
type PlayerTargetQuery<'world, 'state> =
    Query<'world, 'state, (Entity, &'static Transform), (With<Player>, Without<super::Enemy>)>;
type GunnerWeaponContext<'world, 'state> = (
    Query<'world, 'state, &'static EnemyWeapon>,
    Query<'world, 'state, (&'static Weapon, &'static GlobalTransform, &'static mut Magazine)>,
    Query<'world, 'state, &'static RagdollData>,
    Query<'world, 'state, &'static Position, With<RagdollBodyPart>>,
);
#[derive(Component, Default)]
struct GunnerAttack {
    remaining_cooldown_seconds: Option<f32>,
    firing_elapsed_seconds: Option<f32>,
    reload_elapsed_seconds: Option<f32>,
}

#[derive(Component)]
struct GunnerAttackJitter(f32);

#[derive(SystemParam)]
struct GunnerMessageContext<'w> {
    weapon_fired: MessageWriter<'w, WeaponFired>,
    weapon_reload_started: MessageWriter<'w, WeaponReloadStarted>,
}

type GunnerQuery<'world, 'state> = Query<
    'world,
    'state,
    (
        Entity,
        &'static Gunner,
        &'static GunnerAttackJitter,
        &'static mut GunnerAttack,
        &'static mut Transform,
        &'static mut EnemyActivity,
        &'static mut LinearVelocity,
    ),
    (AliveEnemy, With<Gunner>),
>;

fn collider_inspection_is_disabled(collider_inspection: Res<ColliderInspection>) -> bool {
    !collider_inspection.0
}

fn update_gunners(
    (time, move_and_slide, gravity): GunnerMovementContext,
    player: PlayerTargetQuery,
    player_children: Query<&Children, With<Player>>,
    owned_bodies: Query<(Entity, &OwnedByCharacter)>,
    (enemy_weapons, mut weapons, ragdoll_roots, ragdoll_body_positions): GunnerWeaponContext,
    mut gunners: GunnerQuery,
    mut gunner_messages: GunnerMessageContext,
) {
    let Ok((player_entity, player_transform)) = player.single() else {
        return;
    };
    let delta_secs = time.delta_secs();

    for (gunner_entity, gunner, attack_jitter, mut attack, mut gunner_transform, mut activity, mut velocity) in
        &mut gunners
    {
        let filter = exclude_character_bodies(&[gunner_entity], &owned_bodies);
        let physics_world = PhysicsWorld::new(
            &move_and_slide,
            gravity.0.y,
            delta_secs,
            &filter,
            ENEMY_COLLISION_RADIUS,
            ENEMY_COLLISION_CENTER_HEIGHT,
            0.0,
        );
        let player_is_in_attack_range = move_enemy(
            &physics_world,
            &mut gunner_transform,
            &mut activity,
            &mut velocity,
            |gunner_transform, activity, physics_world| {
                gunner.update(player_transform, gunner_transform, activity, physics_world)
            },
        );
        if !player_is_in_attack_range {
            *attack = GunnerAttack::default();
            continue;
        }
        let Ok(enemy_weapon) = enemy_weapons.get(gunner_entity) else {
            continue;
        };
        let Ok((weapon, weapon_transform, mut magazine)) = weapons.get_mut(enemy_weapon.0) else {
            continue;
        };
        if update_gunner_reload(*weapon, &mut magazine, &mut attack, &mut activity, delta_secs) {
            continue;
        }
        if !advance_gunner_attack(gunner, attack_jitter, &mut attack, &mut activity, delta_secs) {
            continue;
        }
        if !magazine.consume_round() {
            begin_gunner_reload(
                *weapon,
                &mut attack,
                &mut activity,
                &mut gunner_messages.weapon_reload_started,
            );
            continue;
        }
        let origin = weapon_transform.translation();
        let aim_target =
            find_player_head_position(player_entity, &player_children, &ragdoll_roots, &ragdoll_body_positions)
                .unwrap_or(player_transform.translation);
        let target = calculate_gunner_target(aim_target);
        let Some(direction) = Dir3::new(target - origin).ok() else {
            continue;
        };
        gunner_messages.weapon_fired.write(WeaponFired {
            shooter_entity: Some(gunner_entity),
            weapon_entity: enemy_weapon.0,
            weapon: *weapon,
            damage_coefficient: gunner.damage_coefficient,
            origin,
            direction,
        });
        if magazine.get_current_ammunition() == 0 {
            begin_gunner_reload(
                *weapon,
                &mut attack,
                &mut activity,
                &mut gunner_messages.weapon_reload_started,
            );
        }
    }
}

fn update_gunner_reload(
    weapon: Weapon,
    magazine: &mut Magazine,
    attack: &mut GunnerAttack,
    activity: &mut EnemyActivity,
    delta_secs: f32,
) -> bool {
    let Some(reload_elapsed_seconds) = attack.reload_elapsed_seconds.as_mut() else {
        return false;
    };
    *reload_elapsed_seconds += delta_secs;
    activity.state = AnimationState::Reloading;
    let Some(reload_duration) = weapon.get_reload_duration() else {
        return false;
    };
    if *reload_elapsed_seconds >= reload_duration.as_secs_f32() {
        magazine.refill(weapon);
        attack.reload_elapsed_seconds = None;
    }
    true
}

fn begin_gunner_reload(
    weapon: Weapon,
    attack: &mut GunnerAttack,
    activity: &mut EnemyActivity,
    weapon_reload_started: &mut MessageWriter<WeaponReloadStarted>,
) {
    attack.remaining_cooldown_seconds = None;
    attack.firing_elapsed_seconds = None;
    attack.reload_elapsed_seconds = Some(0.0);
    activity.state = AnimationState::Reloading;
    weapon_reload_started.write(WeaponReloadStarted(weapon));
}

fn advance_gunner_attack(
    gunner: &Gunner,
    attack_jitter: &GunnerAttackJitter,
    attack: &mut GunnerAttack,
    activity: &mut EnemyActivity,
    delta_secs: f32,
) -> bool {
    let cooldown_jitter_range = 0.15;
    let cooldown_multiplier = 1.0 + (attack_jitter.0 * 2.0 - 1.0) * cooldown_jitter_range;
    let cooldown_seconds = gunner.attack_cooldown * cooldown_multiplier;

    if let Some(firing_elapsed_seconds) = attack.firing_elapsed_seconds.as_mut() {
        *firing_elapsed_seconds += delta_secs;
        activity.state = AnimationState::Attacking;
        if *firing_elapsed_seconds >= gunner.attack_animation_seconds {
            attack.firing_elapsed_seconds = None;
            attack.remaining_cooldown_seconds = Some(cooldown_seconds);
        }
        return false;
    }

    let remaining_cooldown_seconds = attack.remaining_cooldown_seconds.get_or_insert(cooldown_seconds);
    *remaining_cooldown_seconds -= delta_secs;
    if *remaining_cooldown_seconds > 0.0 {
        activity.state = AnimationState::Idle;
        return false;
    }

    attack.remaining_cooldown_seconds = None;
    attack.firing_elapsed_seconds = Some(0.0);
    activity.state = AnimationState::Attacking;
    true
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

impl Gunner {
    fn update(
        &self,
        player: &Transform,
        enemy: &mut Transform,
        activity: &mut EnemyActivity,
        physics_world: &PhysicsWorld,
    ) -> bool {
        let Some((direction, distance)) = flatten_direction(player, enemy) else {
            return false;
        };
        if distance < self.flee_distance {
            let away_direction = -direction;
            enemy.look_to(direction, Vec3::Y);
            enemy.translation += physics_world.steer_direction(
                enemy.translation,
                away_direction * (self.move_speed * physics_world.get_delta_secs()),
            );
            activity.state = AnimationState::Moving;
        } else if distance > self.attack_distance {
            enemy.look_to(-direction, Vec3::Y);
            enemy.translation += physics_world.steer_direction(
                enemy.translation,
                direction * (self.move_speed * physics_world.get_delta_secs()),
            );
            activity.state = AnimationState::Moving;
        } else {
            enemy.look_to(-direction, Vec3::Y);
            activity.state = AnimationState::Idle;
            return true;
        }
        false
    }
}
