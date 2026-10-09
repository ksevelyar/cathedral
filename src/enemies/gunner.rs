use bevy::prelude::*;

use super::{AnimationState, EnemyActivity, EnemyRig, WeaponSpec, flatten_direction};
use crate::movement::PhysicsWorld;

#[derive(Clone)]
pub struct Gunner {
    pub(super) rig: EnemyRig,
    move_speed: f32,
    flee_distance: f32,
    attack_distance: f32,
    attack_cooldown: f32,
    attack_animation_seconds: f32,
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
                weapon: WeaponSpec {
                    path: "weapons/pistol.glb",
                    scale: 0.15,
                    rotation_euler_yxz: (1.5240, 0.0217, 1.3841),
                    translation: Vec3::new(-0.05, 0.0, 0.0),
                    collider_half_extents: Vec3::new(0.88, 0.15, 0.1),
                },
            },
            move_speed: 4.0,
            flee_distance: 4.0,
            attack_distance: 12.0,
            attack_cooldown: 2.0,
            attack_animation_seconds: 1.0,
        }
    }
}

impl Gunner {
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
            activity.attack_on_cooldown(self.attack_cooldown, self.attack_animation_seconds);
        }
    }
}
