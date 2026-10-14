use bevy::prelude::*;

use super::objects::Object;
use super::{CurrentMap, EnemySpawn, Map, PlayerPosition};
use crate::enemies::EnemyKind;

pub const TEST_FLOOR_SIZE: Vec3 = Vec3::new(60.0, 0.1, 60.0);
pub const TALL_PILLAR_POSITION: Vec3 = Vec3::new(0.0, 10.0, 0.0);
pub const TALL_PILLAR_SIZE: Vec3 = Vec3::new(1.0, 20.0, 1.0);
pub const STAIRCASE_APEX_POSITION: Vec3 = Vec3::new(0.0, 1.025, 0.0);
pub const STAIRCASE_APEX_SIZE: Vec3 = Vec3::new(4.0, 2.05, 1.0);

pub(super) fn build_map(map: &CurrentMap) -> Map {
    match map {
        CurrentMap::FlatFloor => Map {
            ambient: GlobalAmbientLight::NONE,
            clear_color: ClearColor(Color::srgb(0.05, 0.05, 0.05)),
            fog_volume: None,
            player_position: PlayerPosition::new(Vec3::new(0.0, 0.5, 0.0)),
            enemies: vec![],
            objects: vec![build_floor()],
        },
        CurrentMap::TallPillar => Map {
            ambient: GlobalAmbientLight::NONE,
            clear_color: ClearColor(Color::srgb(0.05, 0.05, 0.05)),
            fog_volume: None,
            player_position: PlayerPosition::new(Vec3::new(-6.0, 1.5, 0.0)),
            enemies: vec![],
            objects: vec![build_floor(), build_tall_pillar()],
        },
        CurrentMap::Staircase => Map {
            ambient: GlobalAmbientLight::NONE,
            clear_color: ClearColor(Color::srgb(0.05, 0.05, 0.05)),
            fog_volume: None,
            player_position: PlayerPosition::new(Vec3::new(0.0, 1.5, 12.0)),
            enemies: vec![],
            objects: build_staircase_objects(),
        },
        CurrentMap::ColliderInspection => Map {
            ambient: GlobalAmbientLight::default(),
            clear_color: ClearColor(Color::srgb(0.05, 0.05, 0.05)),
            fog_volume: None,
            player_position: PlayerPosition {
                position: Vec3::new(0.0, 1.8, 3.0),
                look_at: Vec3::new(0.0, 1.0, 0.0),
            },
            enemies: vec![EnemySpawn {
                position: Vec3::ZERO,
                kind: EnemyKind::Fighter,
            }],
            objects: vec![build_floor()],
        },
        _ => unreachable!("test map definitions only cover test maps"),
    }
}

fn build_floor() -> Object {
    Object::Floor {
        transform: Transform::IDENTITY,
        size: TEST_FLOOR_SIZE,
    }
}

fn build_tall_pillar() -> Object {
    Object::Wall {
        transform: Transform::from_translation(TALL_PILLAR_POSITION),
        size: TALL_PILLAR_SIZE,
    }
}

fn build_staircase_objects() -> Vec<Object> {
    let mut objects = vec![build_floor()];
    for step_index in 0..4u32 {
        let step_center_height = 0.3 + step_index as f32 * 0.5;
        let up_z = 4.5 - step_index as f32;
        let down_z = -4.5 + step_index as f32;
        for step_z in [up_z, down_z] {
            objects.push(Object::Wall {
                transform: Transform::from_translation(Vec3::new(0.0, step_center_height, step_z)),
                size: Vec3::new(4.0, 0.5, 1.0),
            });
        }
    }
    objects.push(Object::Wall {
        transform: Transform::from_translation(STAIRCASE_APEX_POSITION),
        size: STAIRCASE_APEX_SIZE,
    });
    objects
}
