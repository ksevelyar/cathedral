use avian3d::prelude::{Gravity, LinearVelocity, MoveAndSlide};
use bevy::prelude::*;

use super::{
    AliveEnemy, AnimationState, ENEMY_COLLISION_CENTER_HEIGHT, ENEMY_COLLISION_RADIUS, EnemyActivity, EnemyBehavior,
    EnemyDefinition, EnemyLocomotion, EnemyRig, EnemySenses, EnemySpawnConfiguration, EquippedWeapon, Seed,
    deliver_pending_melee_hit, flatten_direction, move_enemy, spawn_enemy_base,
};
use crate::health::CharacterDamaged;
use crate::movement::PhysicsWorld;
use crate::player::Player;
use crate::ragdoll::{OwnedByCharacter, exclude_character_bodies};
use crate::shooting::{MeleeWeapon, Weapon};

use super::is_time_running;

pub(super) struct FighterPlugin;

impl Plugin for FighterPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Update,
            update_fighters
                .after(super::EnemyBehaviorSystems)
                .run_if(in_state(crate::state::GameState::Playing))
                .run_if(is_time_running),
        );
    }
}

#[derive(Component, Clone)]
pub(super) struct Fighter {
    rig: EnemyRig,
    move_speed: f32,
    attack_cooldown: f32,
    attack_animation_seconds: f32,
    damage_coefficient: f32,
    starting_health: f32,
}

impl Default for Fighter {
    fn default() -> Self {
        Self {
            rig: EnemyRig {
                scene: "characters/animations/ual1-standard.glb",
                animation_source: "characters/animations/ual1-standard.glb",
                idle_animation: 40,
                moving_animation: 36,
                attack_animation: 39,
                reload_animation: None,
                dance_animation: 3,
                idle_animation_seconds: 1.667,
                moving_animation_seconds: 0.667,
                dance_animation_seconds: 1.0,
                weapon: EquippedWeapon {
                    weapon: Weapon::Melee(MeleeWeapon::Katana),
                    transform: Transform::default(),
                },
            },
            move_speed: 3.0,
            attack_cooldown: 2.0,
            attack_animation_seconds: 1.53,
            damage_coefficient: 0.5,
            starting_health: 40.0,
        }
    }
}

pub(super) fn spawn(
    commands: &mut Commands,
    asset_server: &AssetServer,
    seed: Seed,
    sequence_number: usize,
    transform: Transform,
    spawn_configuration: EnemySpawnConfiguration,
    alive: bool,
) -> Entity {
    let fighter = Fighter::default();
    let definition = EnemyDefinition {
        display_name: "Fighter",
        rig: fighter.rig.clone(),
        starting_health: fighter.starting_health,
        patrol_positions: spawn_configuration.patrol_positions,
        initial_behavior: spawn_configuration.initial_behavior,
    };
    let fighter_entity = spawn_enemy_base(
        commands,
        asset_server,
        definition,
        seed,
        sequence_number,
        transform,
        alive,
    );
    let cooldown_jitter_range = 0.15;
    let cooldown_multiplier = 1.0 + (seed.calculate_unit_value(6) * 2.0 - 1.0) * cooldown_jitter_range;
    commands.entity(fighter_entity).insert((
        fighter.clone(),
        FighterAttackJitter(cooldown_multiplier),
        EnemyLocomotion {
            move_speed: fighter.move_speed,
        },
        EnemySenses {
            vision_range: 6.0,
            vision_half_angle_radians: 70.0_f32.to_radians(),
            detection_seconds: 2.0,
            hearing_range: 5.0,
        },
    ));
    if alive {
        commands.entity(fighter_entity).insert(EnemyActivity {
            attack_cooldown: fighter.attack_cooldown * cooldown_multiplier,
            ..default()
        });
    }
    fighter_entity
}

type FighterMovementContext<'world, 'state> = (Res<'world, Time>, MoveAndSlide<'world, 'state>, Res<'world, Gravity>);
type PlayerTargetQuery<'world, 'state> =
    Query<'world, 'state, (Entity, &'static Transform), (With<Player>, Without<super::Enemy>)>;
#[derive(Component)]
struct FighterAttackJitter(f32);

type FighterQuery<'world, 'state> = Query<
    'world,
    'state,
    (
        Entity,
        &'static Fighter,
        &'static FighterAttackJitter,
        &'static EnemyBehavior,
        &'static mut Transform,
        &'static mut EnemyActivity,
        &'static mut LinearVelocity,
    ),
    (AliveEnemy, With<Fighter>),
>;

fn update_fighters(
    (time, move_and_slide, gravity): FighterMovementContext,
    player: PlayerTargetQuery,
    owned_bodies: Query<(Entity, &OwnedByCharacter)>,
    mut fighters: FighterQuery,
    mut character_damaged: MessageWriter<CharacterDamaged>,
) {
    let Ok((player_entity, player_transform)) = player.single() else {
        return;
    };
    let delta_secs = time.delta_secs();

    for (fighter_entity, fighter, attack_jitter, behavior, mut fighter_transform, mut activity, mut velocity) in
        &mut fighters
    {
        if !matches!(behavior, EnemyBehavior::Engaged) {
            continue;
        }
        activity.attack_cooldown -= delta_secs;
        deliver_pending_melee_hit(
            fighter_entity,
            player_entity,
            player_transform,
            &mut fighter_transform,
            &mut activity,
            delta_secs,
            &mut character_damaged,
        );
        let filter = exclude_character_bodies(&[fighter_entity], &owned_bodies);
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
            &mut fighter_transform,
            &mut activity,
            &mut velocity,
            |fighter_transform, activity, physics_world| {
                fighter.update(
                    player_transform,
                    fighter_transform,
                    activity,
                    physics_world,
                    attack_jitter.0,
                )
            },
        );
        if attack_started {
            stage_melee_hit(fighter, &mut activity);
        }
    }
}

impl Fighter {
    fn update(
        &self,
        player: &Transform,
        enemy: &mut Transform,
        activity: &mut EnemyActivity,
        physics_world: &PhysicsWorld,
        attack_cooldown_multiplier: f32,
    ) -> bool {
        let Some((direction, distance)) = flatten_direction(player, enemy) else {
            return false;
        };
        let weapon_range = self.rig.weapon.weapon.get_range();
        if distance <= weapon_range {
            activity.attack_on_cooldown(
                self.attack_cooldown * attack_cooldown_multiplier,
                self.attack_animation_seconds,
            )
        } else {
            activity.state = AnimationState::Moving;
            enemy.look_to(-direction, Vec3::Y);
            enemy.translation += physics_world.steer_direction(
                enemy.translation,
                direction * (self.move_speed * physics_world.get_delta_secs()),
            );
            false
        }
    }
}

fn stage_melee_hit(fighter: &Fighter, activity: &mut EnemyActivity) {
    let katana_contact_delay = 0.5;
    activity.pending_melee_hit = Some(katana_contact_delay);
    activity.pending_melee_damage = fighter.rig.weapon.weapon.get_damage() * fighter.damage_coefficient;
    activity.pending_melee_range = fighter.rig.weapon.weapon.get_range();
}
