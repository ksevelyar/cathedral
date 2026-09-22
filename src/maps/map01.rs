use bevy::prelude::*;

use super::{EnemySpawn, Map, cuboid, floor, stair, wall};
use crate::enemies::{EnemyKind, Fighter, Gunner};

pub(super) fn definition() -> Map {
    Map {
        player_start: Vec3::new(0.0, 1.85, 12.0),
        enemies: vec![
            EnemySpawn {
                position: Vec3::new(-8.0, 0.0, -8.0),
                kind: EnemyKind::Gunner(Gunner::default()),
            },
            EnemySpawn {
                position: Vec3::new(8.0, 0.0, -8.0),
                kind: EnemyKind::Gunner(Gunner::default()),
            },
            EnemySpawn {
                position: Vec3::new(0.0, 0.0, -10.0),
                kind: EnemyKind::Fighter(Fighter::default()),
            },
        ],
        geometry: vec![
            floor(Vec3::ZERO, Vec3::new(30.0, 0.1, 30.0)),
            wall(Vec3::new(0.0, 1.5, -15.0), Vec3::new(30.0, 3.0, 0.1)),
            wall(Vec3::new(0.0, 1.5, 15.0), Vec3::new(30.0, 3.0, 0.1)),
            wall(Vec3::new(-15.0, 1.5, 0.0), Vec3::new(0.1, 3.0, 30.0)),
            wall(Vec3::new(15.0, 1.5, 0.0), Vec3::new(0.1, 3.0, 30.0)),
            cuboid(Vec3::new(-10.0, 10.0, -10.0), Vec3::new(2.0, 20.0, 2.0)),
            cuboid(Vec3::new(10.0, 10.0, -10.0), Vec3::new(2.0, 20.0, 2.0)),
            cuboid(Vec3::new(-10.0, 10.0, 10.0), Vec3::new(2.0, 20.0, 2.0)),
            cuboid(Vec3::new(10.0, 10.0, 10.0), Vec3::new(2.0, 20.0, 2.0)),
            stair(Vec3::new(-2.0, 0.05, 5.0), Vec3::NEG_Z, 4.0, 4, 0.5, 1.0),
            cuboid(Vec3::new(0.0, 1.025, 0.0), Vec3::new(4.0, 2.05, 1.0)),
            stair(Vec3::new(-2.0, 0.05, -5.0), Vec3::Z, 4.0, 4, 0.5, 1.0),
        ],
    }
}
