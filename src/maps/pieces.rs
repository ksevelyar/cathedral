use bevy::prelude::*;

pub(super) struct Piece {
    pub(super) transform: Transform,
    pub(super) items: Vec<Item>,
}

pub(super) enum Item {
    Cuboid(CuboidSpec),
    Cylinder(CylinderSpec),
    Light(LightSpec),
}

pub(super) struct CuboidSpec {
    pub(super) position: Vec3,
    pub(super) size: Vec3,
    pub(super) material: MaterialSpec,
}

pub(super) struct CylinderSpec {
    pub(super) position: Vec3,
    pub(super) radius: f32,
    pub(super) height: f32,
    pub(super) material: MaterialSpec,
}

pub(super) struct LightSpec {
    pub(super) position: Vec3,
    pub(super) color: Color,
    pub(super) intensity: f32,
}

#[derive(Clone, PartialEq)]
pub(super) struct MaterialSpec {
    pub(super) base_color: Color,
    pub(super) base_color_texture: Option<String>,
    pub(super) normal_texture: Option<String>,
    pub(super) metallic_roughness_texture: Option<String>,
    pub(super) roughness_factor: f32,
    pub(super) metallic_factor: f32,
    pub(super) tile_size: f32,
    pub(super) emissive: Option<LinearRgba>,
}

pub(super) const FLOOR_COLOR: Color = Color::srgb(0.4, 0.4, 0.4);
pub(super) const WALL_COLOR: Color = Color::srgb(0.6, 0.6, 0.6);
const CUBOID_COLOR: Color = Color::srgb(0.5, 0.5, 0.5);
const TEXTURES: &str = "maps/industrial/textures";

fn piece(transform: Transform, items: Vec<Item>) -> Piece {
    Piece { transform, items }
}

pub(super) fn cuboid(position: Vec3, size: Vec3, color: Color) -> Piece {
    textured_cuboid(position, size, solid_material(color))
}

pub(super) fn textured_cuboid(position: Vec3, size: Vec3, material: MaterialSpec) -> Piece {
    piece(
        Transform::from_translation(position),
        vec![Item::Cuboid(CuboidSpec {
            position: Vec3::ZERO,
            size,
            material,
        })],
    )
}

pub(super) fn cylinder(position: Vec3, radius: f32, height: f32, material: MaterialSpec) -> Piece {
    oriented_cylinder(Transform::from_translation(position), radius, height, material)
}

pub(super) fn oriented_cylinder(transform: Transform, radius: f32, height: f32, material: MaterialSpec) -> Piece {
    piece(
        transform,
        vec![Item::Cylinder(CylinderSpec {
            position: Vec3::ZERO,
            radius,
            height,
            material,
        })],
    )
}

pub(super) fn solid_material(base_color: Color) -> MaterialSpec {
    MaterialSpec {
        base_color,
        base_color_texture: None,
        normal_texture: None,
        metallic_roughness_texture: None,
        roughness_factor: 0.9,
        metallic_factor: 0.0,
        tile_size: 1.0,
        emissive: None,
    }
}

fn textured_material(
    base_color_texture: String,
    normal_texture: String,
    metallic_roughness_texture: String,
    roughness_factor: f32,
    metallic_factor: f32,
    tile_size: f32,
) -> MaterialSpec {
    MaterialSpec {
        base_color: Color::WHITE,
        base_color_texture: Some(base_color_texture),
        normal_texture: Some(normal_texture),
        metallic_roughness_texture: Some(metallic_roughness_texture),
        roughness_factor,
        metallic_factor,
        tile_size,
        emissive: None,
    }
}

pub(super) fn concrete_material(tile_size: f32) -> MaterialSpec {
    MaterialSpec {
        base_color: Color::WHITE,
        base_color_texture: Some(format!("{TEXTURES}/concrete_floor/base_color.jpg")),
        normal_texture: Some(format!("{TEXTURES}/concrete_floor/normal.png")),
        metallic_roughness_texture: None,
        roughness_factor: 0.85,
        metallic_factor: 0.0,
        tile_size,
        emissive: None,
    }
}

pub(super) fn painted_metal_material() -> MaterialSpec {
    textured_material(
        format!("{TEXTURES}/painted_metal/base_color.png"),
        format!("{TEXTURES}/painted_metal/normal.png"),
        format!("{TEXTURES}/painted_metal/metallic_roughness.png"),
        1.0,
        1.0,
        1.0,
    )
}

pub(super) fn rust_material() -> MaterialSpec {
    textured_material(
        format!("{TEXTURES}/rust/base_color.jpg"),
        format!("{TEXTURES}/rust/normal.png"),
        format!("{TEXTURES}/rust/metallic_roughness.png"),
        1.0,
        0.9,
        1.0,
    )
}

pub(super) fn emissive_material(color: Color, intensity: f32) -> MaterialSpec {
    MaterialSpec {
        base_color: Color::BLACK,
        base_color_texture: None,
        normal_texture: None,
        metallic_roughness_texture: None,
        roughness_factor: 1.0,
        metallic_factor: 0.0,
        tile_size: 1.0,
        emissive: Some(LinearRgba::from(color) * intensity),
    }
}

pub(super) fn stair(transform: Transform, width: f32, steps: u32, step_height: f32, step_depth: f32) -> Piece {
    let items = (0..steps)
        .map(|step_index| {
            let along = step_index as f32 + 0.5;
            Item::Cuboid(CuboidSpec {
                position: Vec3::NEG_Z * (step_depth * along) + Vec3::Y * (step_height * along),
                size: Vec3::new(width, step_height, step_depth),
                material: solid_material(CUBOID_COLOR),
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
                material: solid_material(WALL_COLOR),
            }),
            Item::Cuboid(CuboidSpec {
                position: Vec3::new(post_offset, height * 0.5, 0.0),
                size: Vec3::new(post_thickness, height, thickness),
                material: solid_material(WALL_COLOR),
            }),
            Item::Cuboid(CuboidSpec {
                position: Vec3::new(0.0, height + lintel_thickness * 0.5, 0.0),
                size: Vec3::new(width + post_thickness * 2.0, lintel_thickness, thickness),
                material: solid_material(WALL_COLOR),
            }),
        ],
    )
}

pub(super) fn hanging_lamp(position: Vec3) -> Piece {
    piece(
        Transform::from_translation(position),
        vec![
            Item::Cylinder(CylinderSpec {
                position: Vec3::new(0.0, 5.72, 0.0),
                radius: 0.02,
                height: 0.36,
                material: rust_material(),
            }),
            Item::Cylinder(CylinderSpec {
                position: Vec3::new(0.0, 5.5, 0.0),
                radius: 0.3,
                height: 0.1,
                material: painted_metal_material(),
            }),
            Item::Cylinder(CylinderSpec {
                position: Vec3::new(0.0, 5.44, 0.0),
                radius: 0.26,
                height: 0.02,
                material: emissive_material(Color::srgb(1.0, 0.95, 0.88), 4.0),
            }),
            Item::Light(LightSpec {
                position: Vec3::new(0.0, 5.2, 0.0),
                color: Color::srgb(1.0, 0.95, 0.88),
                intensity: 120_000.0,
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
                material: solid_material(Color::srgb(0.35, 0.2, 0.1)),
            }),
            Item::Light(LightSpec {
                position: Vec3::new(0.0, 2.5, -0.4),
                color: Color::srgb(1.0, 0.6, 0.2),
                intensity: 900_000.0,
            }),
        ],
    )
}
