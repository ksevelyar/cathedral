use bevy::prelude::*;

use super::{AnimationState, EnemyActivity, EnemyRig, Obstacles, WeaponSpec, planar_direction};

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
                scene: "animation/Unreal-Godot/UAL1_Standard.glb",
                animation_source: "animation/Unreal-Godot/UAL1_Standard.glb",
                idle_animation: 40,
                moving_animation: 36,
                attack_animation: 39,
                weapon: WeaponSpec {
                    path: "weapon/katana.glb",
                    scale: 1.0,
                    rotation_euler_yxz: (0.0, 0.0, 0.0),
                    translation: Vec3::ZERO,
                    collider_half_extents: Vec3::new(0.01, 0.04, 0.35),
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
        delta_secs: f32,
        obstacles: Obstacles,
    ) {
        let Some((direction, distance)) = planar_direction(player, enemy) else {
            return;
        };

        if distance <= self.reach {
            activity.attack_on_cooldown(self.attack_cooldown, self.attack_animation_seconds);
        } else {
            activity.state = AnimationState::Moving;
            enemy.look_to(-direction, Vec3::Y);
            enemy.translation +=
                obstacles.clear_direction(enemy.translation, direction * (self.move_speed * delta_secs));
        }
    }
}
