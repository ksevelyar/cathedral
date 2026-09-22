use bevy::prelude::*;

use super::{EnemySpawn, Map, cuboid, floor, stair, wall};
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
        geometry: vec![
            floor(Vec3::ZERO, Vec3::new(30.0, 0.1, 30.0)),
            wall(Vec3::new(0.0, 1.5, -15.0), Vec3::new(30.0, 3.0, 0.1)),
            wall(Vec3::new(0.0, 1.5, 15.0), Vec3::new(30.0, 3.0, 0.1)),
            wall(Vec3::new(-15.0, 1.5, 0.0), Vec3::new(0.1, 3.0, 30.0)),
            wall(Vec3::new(15.0, 1.5, 0.0), Vec3::new(0.1, 3.0, 30.0)),
            cuboid(Vec3::new(-6.0, 1.0, -7.0), Vec3::new(4.0, 2.0, 4.0)),
            cuboid(Vec3::new(6.0, 0.75, -7.0), Vec3::new(4.0, 1.5, 4.0)),
            stair(Vec3::new(-2.0, 0.05, 6.0), Vec3::new(0.0, 0.0, -1.0), 4.0, 4, 0.5, 1.0),
        ],
    }
}
