use std::f32::consts::FRAC_PI_2;

use bevy::prelude::*;

use super::pieces::{
    Piece, concrete_material, cylinder, emissive_material, hanging_lamp, oriented_cylinder, painted_metal_material,
    rust_material, textured_cuboid,
};
use super::{EnemySpawn, Map};
use crate::enemies::{EnemyKind, Fighter, Gunner};

pub(super) fn definition() -> Map {
    Map {
        player_start: Vec3::new(0.0, 1.85, 6.0),
        enemies: vec![
            EnemySpawn {
                position: Vec3::new(-8.0, 0.0, -6.0),
                kind: EnemyKind::Gunner(Gunner::default()),
            },
            EnemySpawn {
                position: Vec3::new(8.0, 0.0, -6.0),
                kind: EnemyKind::Gunner(Gunner::default()),
            },
            EnemySpawn {
                position: Vec3::new(0.0, 0.0, -6.5),
                kind: EnemyKind::Fighter(Fighter::default()),
            },
        ],
        pieces: control_room_pieces(),
    }
}

fn control_room_pieces() -> Vec<Piece> {
    let mut pieces = vec![floor(), ceiling(), duct(), conduit(), vertical_pipe_drop()];
    pieces.extend(walls());
    pieces.extend(ceiling_beams());
    pieces.extend(doorway());
    pieces.extend(west_wall_pipes());
    pieces.extend(west_pipe_brackets());
    pieces.extend(led_strips());
    pieces.extend(control_room_lights());
    pieces
}

fn floor() -> Piece {
    textured_cuboid(
        Vec3::new(0.0, 0.05, 0.0),
        Vec3::new(25.8, 0.1, 19.8),
        concrete_material(2.0),
    )
}

fn ceiling() -> Piece {
    textured_cuboid(
        Vec3::new(0.0, 5.95, 0.0),
        Vec3::new(25.8, 0.1, 19.8),
        concrete_material(3.0),
    )
}

fn walls() -> Vec<Piece> {
    vec![
        textured_cuboid(
            Vec3::new(0.0, 3.0, 9.6),
            Vec3::new(25.4, 6.0, 0.2),
            concrete_material(3.0),
        ),
        textured_cuboid(
            Vec3::new(12.6, 3.0, 0.0),
            Vec3::new(0.2, 6.0, 19.4),
            concrete_material(3.0),
        ),
        textured_cuboid(
            Vec3::new(-12.6, 3.0, 0.0),
            Vec3::new(0.2, 6.0, 19.4),
            concrete_material(3.0),
        ),
        textured_cuboid(
            Vec3::new(-6.75, 3.0, -9.6),
            Vec3::new(11.9, 6.0, 0.2),
            concrete_material(3.0),
        ),
        textured_cuboid(
            Vec3::new(6.75, 3.0, -9.6),
            Vec3::new(11.9, 6.0, 0.2),
            concrete_material(3.0),
        ),
        textured_cuboid(
            Vec3::new(0.0, 4.1, -9.6),
            Vec3::new(1.6, 3.8, 0.2),
            concrete_material(3.0),
        ),
    ]
}

fn ceiling_beams() -> Vec<Piece> {
    vec![
        textured_cuboid(
            Vec3::new(0.0, 5.75, 9.25),
            Vec3::new(25.0, 0.3, 0.3),
            concrete_material(3.0),
        ),
        textured_cuboid(
            Vec3::new(0.0, 5.75, -9.25),
            Vec3::new(25.0, 0.3, 0.3),
            concrete_material(3.0),
        ),
        textured_cuboid(
            Vec3::new(12.35, 5.75, 0.0),
            Vec3::new(0.3, 0.3, 18.5),
            concrete_material(3.0),
        ),
        textured_cuboid(
            Vec3::new(-12.35, 5.75, 0.0),
            Vec3::new(0.3, 0.3, 18.5),
            concrete_material(3.0),
        ),
    ]
}

fn doorway() -> Vec<Piece> {
    vec![
        textured_cuboid(
            Vec3::new(-0.86, 1.15, -9.4),
            Vec3::new(0.12, 2.3, 0.12),
            painted_metal_material(),
        ),
        textured_cuboid(
            Vec3::new(0.86, 1.15, -9.4),
            Vec3::new(0.12, 2.3, 0.12),
            painted_metal_material(),
        ),
        textured_cuboid(
            Vec3::new(0.0, 2.26, -9.4),
            Vec3::new(1.84, 0.12, 0.12),
            painted_metal_material(),
        ),
        textured_cuboid(
            Vec3::new(0.0, 1.1, -9.45),
            Vec3::new(1.6, 2.2, 0.08),
            painted_metal_material(),
        ),
    ]
}

fn west_wall_pipes() -> Vec<Piece> {
    vec![
        oriented_cylinder(
            Transform::from_xyz(-12.3, 5.0, 0.0).with_rotation(Quat::from_rotation_x(FRAC_PI_2)),
            0.08,
            18.5,
            rust_material(),
        ),
        oriented_cylinder(
            Transform::from_xyz(-12.3, 5.15, 0.0).with_rotation(Quat::from_rotation_x(FRAC_PI_2)),
            0.08,
            18.5,
            rust_material(),
        ),
    ]
}

fn west_pipe_brackets() -> Vec<Piece> {
    let mut brackets = Vec::new();
    for z_position in [-7.0, -3.5, 0.0, 3.5, 7.0] {
        for y_position in [5.0, 5.15] {
            brackets.push(oriented_cylinder(
                Transform::from_xyz(-12.4, y_position, z_position).with_rotation(Quat::from_rotation_z(FRAC_PI_2)),
                0.025,
                0.2,
                rust_material(),
            ));
        }
    }
    brackets
}

fn vertical_pipe_drop() -> Piece {
    cylinder(Vec3::new(-12.3, 2.5, -9.0), 0.08, 5.0, rust_material())
}

fn duct() -> Piece {
    textured_cuboid(
        Vec3::new(12.2, 5.0, 0.0),
        Vec3::new(0.6, 0.4, 17.5),
        painted_metal_material(),
    )
}

fn conduit() -> Piece {
    oriented_cylinder(
        Transform::from_xyz(0.0, 0.15, 9.4).with_rotation(Quat::from_rotation_z(FRAC_PI_2)),
        0.02,
        24.8,
        painted_metal_material(),
    )
}

fn led_strips() -> Vec<Piece> {
    vec![
        textured_cuboid(
            Vec3::new(0.0, 5.88, -7.5),
            Vec3::new(20.0, 0.04, 0.15),
            emissive_material(Color::srgb(0.55, 0.7, 1.0), 0.6),
        ),
        textured_cuboid(
            Vec3::new(0.0, 5.88, 7.5),
            Vec3::new(20.0, 0.04, 0.15),
            emissive_material(Color::srgb(0.55, 0.7, 1.0), 0.6),
        ),
    ]
}

fn control_room_lights() -> Vec<Piece> {
    [(-6.0, -4.5), (6.0, -4.5), (-6.0, 4.5), (6.0, 4.5)]
        .into_iter()
        .map(|(x_position, z_position)| hanging_lamp(Vec3::new(x_position, 0.0, z_position)))
        .collect()
}
