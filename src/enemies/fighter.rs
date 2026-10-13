use bevy::prelude::*;

use super::{AnimationState, EnemyActivity, EnemyRig, EquippedWeapon, flatten_direction};
use crate::movement::PhysicsWorld;
use crate::shooting::{MeleeWeapon, Weapon};

#[derive(Clone)]
pub struct Fighter {
    pub(super) rig: EnemyRig,
    move_speed: f32,
    reach: f32,
    attack_cooldown: f32,
    attack_animation_seconds: f32,
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
                weapon: EquippedWeapon {
                    weapon: Weapon::Melee(MeleeWeapon::Katana),
                    transform: Transform::default(),
                },
            },
            move_speed: 3.0,
            reach: 2.5,
            attack_cooldown: 2.0,
            attack_animation_seconds: 1.53,
        }
    }
}

impl Fighter {
    pub(super) fn update(
        &self,
        player: &Transform,
        enemy: &mut Transform,
        activity: &mut EnemyActivity,
        physics_world: &PhysicsWorld,
    ) {
        let Some((direction, distance)) = flatten_direction(player, enemy) else {
            return;
        };

        if distance <= self.reach {
            let _ = activity.attack_on_cooldown(self.attack_cooldown, self.attack_animation_seconds);
        } else {
            activity.state = AnimationState::Moving;
            enemy.look_to(-direction, Vec3::Y);
            enemy.translation += physics_world.steer_direction(
                enemy.translation,
                direction * (self.move_speed * physics_world.get_delta_secs()),
            );
        }
    }
}
