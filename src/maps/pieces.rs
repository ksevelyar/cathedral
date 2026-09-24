use bevy::prelude::*;

pub(super) struct Piece {
    pub(super) transform: Transform,
    pub(super) items: Vec<Item>,
}

pub(super) enum Item {
    Cuboid(CuboidSpec),
    Light(LightSpec),
}

pub(super) struct CuboidSpec {
    pub(super) position: Vec3,
    pub(super) size: Vec3,
    pub(super) color: Color,
}

pub(super) struct LightSpec {
    pub(super) position: Vec3,
    pub(super) color: Color,
    pub(super) intensity: f32,
}

pub(super) const FLOOR_COLOR: Color = Color::srgb(0.4, 0.4, 0.4);
pub(super) const WALL_COLOR: Color = Color::srgb(0.6, 0.6, 0.6);
const CUBOID_COLOR: Color = Color::srgb(0.5, 0.5, 0.5);

fn piece(transform: Transform, items: Vec<Item>) -> Piece {
    Piece { transform, items }
}

pub(super) fn cuboid(position: Vec3, size: Vec3, color: Color) -> Piece {
    piece(
        Transform::from_translation(position),
        vec![Item::Cuboid(CuboidSpec {
            position: Vec3::ZERO,
            size,
            color,
        })],
    )
}

pub(super) fn stair(transform: Transform, width: f32, steps: u32, step_height: f32, step_depth: f32) -> Piece {
    let items = (0..steps)
        .map(|step_index| {
            let along = step_index as f32 + 0.5;
            Item::Cuboid(CuboidSpec {
                position: Vec3::NEG_Z * (step_depth * along) + Vec3::Y * (step_height * along),
                size: Vec3::new(width, step_height, step_depth),
                color: CUBOID_COLOR,
            })
        })
        .collect();
    piece(transform, items)
}

pub(super) fn door(transform: Transform, width: f32, height: f32, thickness: f32) -> Piece {
    let post_thickness = 0.5;
    let lintel_thickness = 0.5;
    let post_offset = (width + post_thickness) * 0.5;
    piece(
        transform,
        vec![
            Item::Cuboid(CuboidSpec {
                position: Vec3::new(-post_offset, height * 0.5, 0.0),
                size: Vec3::new(post_thickness, height, thickness),
                color: WALL_COLOR,
            }),
            Item::Cuboid(CuboidSpec {
                position: Vec3::new(post_offset, height * 0.5, 0.0),
                size: Vec3::new(post_thickness, height, thickness),
                color: WALL_COLOR,
            }),
            Item::Cuboid(CuboidSpec {
                position: Vec3::new(0.0, height + lintel_thickness * 0.5, 0.0),
                size: Vec3::new(width + post_thickness * 2.0, lintel_thickness, thickness),
                color: WALL_COLOR,
            }),
        ],
    )
}

pub(super) fn wall_torch(transform: Transform) -> Piece {
    piece(
        transform,
        vec![
            Item::Cuboid(CuboidSpec {
                position: Vec3::new(0.0, 2.2, -0.15),
                size: Vec3::new(0.15, 0.4, 0.15),
                color: Color::srgb(0.35, 0.2, 0.1),
            }),
            Item::Light(LightSpec {
                position: Vec3::new(0.0, 2.5, -0.4),
                color: Color::srgb(1.0, 0.6, 0.2),
                intensity: 900_000.0,
            }),
        ],
    )
}
