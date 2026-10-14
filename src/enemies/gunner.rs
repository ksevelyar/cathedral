use avian3d::prelude::{Gravity, LinearVelocity, MoveAndSlide, Position};
use bevy::prelude::*;
use rand::Rng;

use super::{
    AliveEnemy, AnimationState, ENEMY_COLLISION_CENTER_HEIGHT, ENEMY_COLLISION_RADIUS, EnemyActivity, EnemyDefinition,
    EnemyRig, EnemyWeapon, EquippedWeapon, find_player_head_position, flatten_direction, move_enemy, spawn_enemy_base,
};
use crate::maps::CurrentMap;
use crate::movement::PhysicsWorld;
use crate::player::Player;
use crate::ragdoll::{OwnedByCharacter, RagdollBodyPart, RagdollData, exclude_character_bodies};
use crate::shooting::{RangedWeapon, Weapon, WeaponFired};
use crate::state::ColliderInspection;

const GUNNER_ACCURACY: f64 = 0.65;
const GUNNER_MISS_DISTANCE: f32 = 0.75;

pub(super) struct GunnerPlugin;

impl Plugin for GunnerPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Update,
            update_gunners
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
                weapon: EquippedWeapon {
                    weapon: Weapon::Ranged(RangedWeapon::Pistol),
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
            attack_animation_seconds: 1.0,
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
    commands.entity(gunner_entity).insert(gunner);
    gunner_entity
}

type GunnerMovementContext<'world, 'state> = (
    Res<'world, CurrentMap>,
    Res<'world, Time>,
    MoveAndSlide<'world, 'state>,
    Res<'world, Gravity>,
);
type PlayerTargetQuery<'world, 'state> =
    Query<'world, 'state, (Entity, &'static Transform), (With<Player>, Without<super::Enemy>)>;
type GunnerWeaponContext<'world, 'state> = (
    Query<'world, 'state, &'static EnemyWeapon>,
    Query<'world, 'state, (&'static Weapon, &'static GlobalTransform)>,
    Query<'world, 'state, &'static RagdollData>,
    Query<'world, 'state, &'static Position, With<RagdollBodyPart>>,
);
type GunnerQuery<'world, 'state> = Query<
    'world,
    'state,
    (
        Entity,
        &'static Gunner,
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
    (current_map, time, move_and_slide, gravity): GunnerMovementContext,
    player: PlayerTargetQuery,
    player_children: Query<&Children, With<Player>>,
    owned_bodies: Query<(Entity, &OwnedByCharacter)>,
    (enemy_weapons, weapons, ragdoll_roots, ragdoll_body_positions): GunnerWeaponContext,
    mut gunners: GunnerQuery,
    mut weapon_fired: MessageWriter<WeaponFired>,
) {
    if *current_map == CurrentMap::ColliderInspection {
        return;
    }
    let Ok((player_entity, player_transform)) = player.single() else {
        return;
    };
    let delta_secs = time.delta_secs();

    for (gunner_entity, gunner, mut gunner_transform, mut activity, mut velocity) in &mut gunners {
        activity.attack_cooldown -= delta_secs;
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
        let attack_started = move_enemy(
            &physics_world,
            &mut gunner_transform,
            &mut activity,
            &mut velocity,
            |gunner_transform, activity, physics_world| {
                gunner.update(player_transform, gunner_transform, activity, physics_world)
            },
        );
        if !attack_started {
            continue;
        }
        let Ok(enemy_weapon) = enemy_weapons.get(gunner_entity) else {
            continue;
        };
        let Ok((weapon, weapon_transform)) = weapons.get(enemy_weapon.0) else {
            continue;
        };
        let origin = weapon_transform.translation();
        let aim_target =
            find_player_head_position(player_entity, &player_children, &ragdoll_roots, &ragdoll_body_positions)
                .unwrap_or(player_transform.translation);
        let target = calculate_gunner_target(aim_target);
        let Some(direction) = Dir3::new(target - origin).ok() else {
            continue;
        };
        weapon_fired.write(WeaponFired {
            shooter_entity: Some(gunner_entity),
            weapon_entity: enemy_weapon.0,
            weapon: *weapon,
            damage_coefficient: gunner.damage_coefficient,
            origin,
            direction,
        });
    }
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
            return activity.attack_on_cooldown(self.attack_cooldown, self.attack_animation_seconds);
        }
        false
    }
}
