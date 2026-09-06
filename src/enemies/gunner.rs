use bevy::prelude::*;

use super::{AnimationState, EnemyActivity, EnemyRig, WeaponSpec, planar_direction};

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
                scene: "universal-base-characters/Base Characters/Godot - UE/Superhero_Female_FullBody.gltf",
                animation_source: "animation/Unreal-Godot/UAL1_Standard.glb",
                idle_animation: 21,
                moving_animation: 36,
                attack_animation: 23,
                weapon: WeaponSpec {
                    path: "weapon/pistol.glb",
                    scale: 0.15,
                    rotation_euler_yxz: (1.5240, 0.0217, 1.3841),
                    translation: Vec3::new(-0.05, 0.0, 0.0),
                    collider_half_extents: Vec3::new(0.88, 0.15, 0.1),
                },
            },
            move_speed: 1.0,
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
        delta_secs: f32,
    ) {
        let Some((direction, distance)) = planar_direction(player, enemy) else {
            return;
        };

        if distance < self.flee_distance {
            let away_direction = -direction;
            enemy.look_to(direction, Vec3::Y);
            enemy.translation += away_direction * (self.move_speed * delta_secs);
            activity.state = AnimationState::Moving;
        } else if distance > self.attack_distance {
            enemy.look_to(-direction, Vec3::Y);
            enemy.translation += direction * (self.move_speed * delta_secs);
            activity.state = AnimationState::Moving;
        } else {
            enemy.look_to(-direction, Vec3::Y);
            activity.attack_on_cooldown(self.attack_cooldown, self.attack_animation_seconds);
        }
    }
}
