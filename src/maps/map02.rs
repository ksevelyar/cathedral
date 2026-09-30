use std::f32::consts::{FRAC_PI_2, PI};

use bevy::prelude::*;

use super::pieces::{FLOOR_COLOR, WALL_COLOR, cuboid, door, stair, wall_torch};
use super::{EnemySpawn, Map};
use crate::enemies::{EnemyKind, Fighter, Gunner};

pub(super) fn definition() -> Map {
    Map {
        player_start: Vec3::new(0.0, 1.85, 12.0),
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
            cuboid(Vec3::ZERO, Vec3::new(30.0, 0.1, 30.0), FLOOR_COLOR),
            cuboid(Vec3::new(0.0, 1.5, -15.0), Vec3::new(30.0, 3.0, 0.1), WALL_COLOR),
            cuboid(Vec3::new(0.0, 1.5, 15.0), Vec3::new(30.0, 3.0, 0.1), WALL_COLOR),
            cuboid(Vec3::new(-15.0, 1.5, 0.0), Vec3::new(0.1, 3.0, 30.0), WALL_COLOR),
            cuboid(Vec3::new(15.0, 1.5, 0.0), Vec3::new(0.1, 3.0, 30.0), WALL_COLOR),
            cuboid(
                Vec3::new(-6.0, 1.0, -7.0),
                Vec3::new(4.0, 2.0, 4.0),
                Color::srgb(0.5, 0.5, 0.5),
            ),
            cuboid(
                Vec3::new(6.0, 1.0, -7.0),
                Vec3::new(4.0, 2.0, 4.0),
                Color::srgb(0.5, 0.5, 0.5),
            ),
            cuboid(
                Vec3::new(-6.0, 1.0, 7.0),
                Vec3::new(4.0, 2.0, 4.0),
                Color::srgb(0.5, 0.5, 0.5),
            ),
            cuboid(
                Vec3::new(6.0, 1.0, 7.0),
                Vec3::new(4.0, 2.0, 4.0),
                Color::srgb(0.5, 0.5, 0.5),
            ),
            stair(
                Transform::from_xyz(14.95, 0.05, 0.0).with_rotation(Quat::from_rotation_y(FRAC_PI_2)),
                4.0,
                15,
                0.1,
                1.0,
            ),
            stair(
                Transform::from_xyz(-14.95, 0.05, 0.0).with_rotation(Quat::from_rotation_y(-FRAC_PI_2)),
                4.0,
                15,
                0.1,
                1.0,
            ),
            door(Transform::from_xyz(0.0, 0.0, -14.95), 3.0, 2.5, 0.3),
            wall_torch(Transform::from_xyz(-8.0, 0.0, -14.95).with_rotation(Quat::from_rotation_y(PI))),
            wall_torch(Transform::from_xyz(8.0, 0.0, -14.95).with_rotation(Quat::from_rotation_y(PI))),
            wall_torch(Transform::from_xyz(-8.0, 0.0, 14.95)),
            wall_torch(Transform::from_xyz(8.0, 0.0, 14.95)),
        ],
    }
}
