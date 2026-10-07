use std::f32::consts::{FRAC_PI_2, PI};

use bevy::prelude::*;

use super::pieces::{build_default_cuboid, build_door, build_stair, build_wall_torch};
use super::{EnemySpawn, Map, PlayerPosition};
use crate::enemies::{EnemyKind, Fighter, Gunner};

pub(super) fn build_map() -> Map {
    Map {
        ambient: GlobalAmbientLight::NONE,
        clear_color: ClearColor(Color::srgb(0.05, 0.05, 0.05)),
        player_position: PlayerPosition::new(Vec3::new(0.0, 1.85, 12.0)),
        enemies: vec![
            EnemySpawn {
                position: Vec3::new(-6.0, 0.0, -10.0),
                kind: EnemyKind::Fighter(Fighter::default()),
            },
            EnemySpawn {
                position: Vec3::new(6.0, 0.0, -10.0),
                kind: EnemyKind::Fighter(Fighter::default()),
            },
            EnemySpawn {
                position: Vec3::new(0.0, 0.0, -12.0),
                kind: EnemyKind::Gunner(Gunner::default()),
            },
            EnemySpawn {
                position: Vec3::new(0.0, 0.0, -4.0),
                kind: EnemyKind::Gunner(Gunner::default()),
            },
        ],
        pieces: vec![
            build_default_cuboid(Vec3::ZERO, Vec3::new(30.0, 0.1, 30.0)),
            build_default_cuboid(Vec3::new(0.0, 1.5, -15.0), Vec3::new(30.0, 3.0, 0.1)),
            build_default_cuboid(Vec3::new(0.0, 1.5, 15.0), Vec3::new(30.0, 3.0, 0.1)),
            build_default_cuboid(Vec3::new(-15.0, 1.5, 0.0), Vec3::new(0.1, 3.0, 30.0)),
            build_default_cuboid(Vec3::new(15.0, 1.5, 0.0), Vec3::new(0.1, 3.0, 30.0)),
            build_default_cuboid(Vec3::new(-6.0, 1.0, -7.0), Vec3::new(4.0, 2.0, 4.0)),
            build_default_cuboid(Vec3::new(6.0, 1.0, -7.0), Vec3::new(4.0, 2.0, 4.0)),
            build_default_cuboid(Vec3::new(-6.0, 1.0, 7.0), Vec3::new(4.0, 2.0, 4.0)),
            build_default_cuboid(Vec3::new(6.0, 1.0, 7.0), Vec3::new(4.0, 2.0, 4.0)),
            build_stair(
                Transform::from_xyz(14.95, 0.05, 0.0).with_rotation(Quat::from_rotation_y(FRAC_PI_2)),
                4.0,
                15,
                0.1,
                1.0,
            ),
            build_stair(
                Transform::from_xyz(-14.95, 0.05, 0.0).with_rotation(Quat::from_rotation_y(-FRAC_PI_2)),
                4.0,
                15,
                0.1,
                1.0,
            ),
            build_door(Transform::from_xyz(0.0, 0.0, -14.95), 3.0, 2.5, 0.3),
            build_wall_torch(Transform::from_xyz(-8.0, 0.0, -14.95).with_rotation(Quat::from_rotation_y(PI))),
            build_wall_torch(Transform::from_xyz(8.0, 0.0, -14.95).with_rotation(Quat::from_rotation_y(PI))),
            build_wall_torch(Transform::from_xyz(-8.0, 0.0, 14.95)),
            build_wall_torch(Transform::from_xyz(8.0, 0.0, 14.95)),
        ],
    }
}
