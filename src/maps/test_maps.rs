use bevy::prelude::*;

use super::pieces::{FLOOR_COLOR, Piece, WALL_COLOR, cuboid};
use super::{CurrentMap, Map};

pub const TEST_FLOOR_SIZE: Vec3 = Vec3::new(60.0, 0.1, 60.0);
pub const TALL_PILLAR_POSITION: Vec3 = Vec3::new(0.0, 10.0, 0.0);
pub const TALL_PILLAR_SIZE: Vec3 = Vec3::new(1.0, 20.0, 1.0);
pub const STAIRCASE_APEX_POSITION: Vec3 = Vec3::new(0.0, 1.025, 0.0);
pub const STAIRCASE_APEX_SIZE: Vec3 = Vec3::new(4.0, 2.05, 1.0);

pub(super) fn definition(map: &CurrentMap) -> Map {
    match map {
        CurrentMap::FlatFloor => Map {
            player_start: Vec3::new(0.0, 0.5, 0.0),
            enemies: vec![],
            pieces: vec![floor()],
        },
        CurrentMap::TallPillar => Map {
            player_start: Vec3::new(-6.0, 1.5, 0.0),
            enemies: vec![],
            pieces: vec![floor(), tall_pillar()],
        },
        CurrentMap::Staircase => Map {
            player_start: Vec3::new(0.0, 1.5, 12.0),
            enemies: vec![],
            pieces: staircase_pieces(),
        },
        _ => unreachable!("test map definitions only cover test maps"),
    }
}

fn floor() -> Piece {
    cuboid(Vec3::ZERO, TEST_FLOOR_SIZE, FLOOR_COLOR)
}

fn tall_pillar() -> Piece {
    cuboid(TALL_PILLAR_POSITION, TALL_PILLAR_SIZE, WALL_COLOR)
}

fn staircase_pieces() -> Vec<Piece> {
    let mut pieces = vec![floor()];
    for step_index in 0..4u32 {
        let step_center_height = 0.3 + step_index as f32 * 0.5;
        let up_z = 4.5 - step_index as f32;
        let down_z = -4.5 + step_index as f32;
        for step_z in [up_z, down_z] {
            pieces.push(cuboid(
                Vec3::new(0.0, step_center_height, step_z),
                Vec3::new(4.0, 0.5, 1.0),
                WALL_COLOR,
            ));
        }
    }
    pieces.push(cuboid(STAIRCASE_APEX_POSITION, STAIRCASE_APEX_SIZE, WALL_COLOR));
    pieces
}
