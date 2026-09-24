use bevy::prelude::*;

use super::pieces::{FLOOR_MATERIAL, Piece, WALL_MATERIAL, build_cuboid};
use super::{CurrentMap, Map};

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
            player_start: Vec3::new(0.0, 0.5, 0.0),
            enemies: vec![],
            pieces: vec![build_floor()],
        },
        CurrentMap::TallPillar => Map {
            ambient: GlobalAmbientLight::NONE,
            clear_color: ClearColor(Color::srgb(0.05, 0.05, 0.05)),
            player_start: Vec3::new(-6.0, 1.5, 0.0),
            enemies: vec![],
            pieces: vec![build_floor(), spawn_tall_pillar()],
        },
        CurrentMap::Staircase => Map {
            ambient: GlobalAmbientLight::NONE,
            clear_color: ClearColor(Color::srgb(0.05, 0.05, 0.05)),
            player_start: Vec3::new(0.0, 1.5, 12.0),
            enemies: vec![],
            pieces: spawn_staircase_pieces(),
        },
        _ => unreachable!("test map definitions only cover test maps"),
    }
}

fn build_floor() -> Piece {
    build_cuboid(Vec3::ZERO, TEST_FLOOR_SIZE, FLOOR_MATERIAL)
}

fn spawn_tall_pillar() -> Piece {
    build_cuboid(TALL_PILLAR_POSITION, TALL_PILLAR_SIZE, WALL_MATERIAL)
}

fn spawn_staircase_pieces() -> Vec<Piece> {
    let mut pieces = vec![build_floor()];
    for step_index in 0..4u32 {
        let step_center_height = 0.3 + step_index as f32 * 0.5;
        let up_z = 4.5 - step_index as f32;
        let down_z = -4.5 + step_index as f32;
        for step_z in [up_z, down_z] {
            pieces.push(build_cuboid(
                Vec3::new(0.0, step_center_height, step_z),
                Vec3::new(4.0, 0.5, 1.0),
                WALL_MATERIAL,
            ));
        }
    }
    pieces.push(build_cuboid(
        STAIRCASE_APEX_POSITION,
        STAIRCASE_APEX_SIZE,
        WALL_MATERIAL,
    ));
    pieces
}
