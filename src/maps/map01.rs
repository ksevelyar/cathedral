use bevy::prelude::*;

use super::Map;
use super::pieces::{Piece, build_cuboid, build_hanging_lamp, make_concrete_material};

pub(super) fn build_map() -> Map {
    Map {
        ambient: GlobalAmbientLight::NONE,
        clear_color: ClearColor(Color::srgb(0.05, 0.05, 0.05)),
        player_start: Vec3::new(0.0, 1.85, 6.0),
        enemies: vec![],
        pieces: spawn_control_room_pieces(),
    }
}

fn spawn_control_room_pieces() -> Vec<Piece> {
    let mut pieces = vec![build_floor(), build_ceiling()];
    pieces.extend(build_walls());
    pieces.extend(spawn_ceiling_beams());
    pieces.extend(spawn_control_room_lights());
    pieces
}

fn build_floor() -> Piece {
    build_cuboid(
        Vec3::new(0.0, 0.05, 0.0),
        Vec3::new(25.8, 0.1, 19.8),
        make_concrete_material(2.0),
    )
}

fn build_ceiling() -> Piece {
    build_cuboid(
        Vec3::new(0.0, 5.95, 0.0),
        Vec3::new(25.8, 0.1, 19.8),
        make_concrete_material(3.0),
    )
}

fn build_walls() -> Vec<Piece> {
    vec![
        build_cuboid(
            Vec3::new(0.0, 3.0, 9.6),
            Vec3::new(25.4, 6.0, 0.2),
            make_concrete_material(3.0),
        ),
        build_cuboid(
            Vec3::new(12.6, 3.0, 0.0),
            Vec3::new(0.2, 6.0, 19.4),
            make_concrete_material(3.0),
        ),
        build_cuboid(
            Vec3::new(-12.6, 3.0, 0.0),
            Vec3::new(0.2, 6.0, 19.4),
            make_concrete_material(3.0),
        ),
        build_cuboid(
            Vec3::new(-6.75, 3.0, -9.6),
            Vec3::new(11.9, 6.0, 0.2),
            make_concrete_material(3.0),
        ),
        build_cuboid(
            Vec3::new(6.75, 3.0, -9.6),
            Vec3::new(11.9, 6.0, 0.2),
            make_concrete_material(3.0),
        ),
        build_cuboid(
            Vec3::new(0.0, 4.1, -9.6),
            Vec3::new(1.6, 3.8, 0.2),
            make_concrete_material(3.0),
        ),
    ]
}

fn spawn_ceiling_beams() -> Vec<Piece> {
    vec![
        build_cuboid(
            Vec3::new(0.0, 5.75, 9.25),
            Vec3::new(25.0, 0.3, 0.3),
            make_concrete_material(3.0),
        ),
        build_cuboid(
            Vec3::new(0.0, 5.75, -9.25),
            Vec3::new(25.0, 0.3, 0.3),
            make_concrete_material(3.0),
        ),
        build_cuboid(
            Vec3::new(12.35, 5.75, 0.0),
            Vec3::new(0.3, 0.3, 18.5),
            make_concrete_material(3.0),
        ),
        build_cuboid(
            Vec3::new(-12.35, 5.75, 0.0),
            Vec3::new(0.3, 0.3, 18.5),
            make_concrete_material(3.0),
        ),
    ]
}

fn spawn_control_room_lights() -> Vec<Piece> {
    const LAMP_PIVOT_HEIGHT: f32 = 5.85;
    const LAMP_WIRE_LENGTH: f32 = 2.2;
    [(-6.0, -4.5), (6.0, -4.5), (-6.0, 4.5), (6.0, 4.5)]
        .into_iter()
        .map(|(x_position, z_position)| {
            build_hanging_lamp(Vec3::new(x_position, LAMP_PIVOT_HEIGHT, z_position), LAMP_WIRE_LENGTH)
        })
        .collect()
}
