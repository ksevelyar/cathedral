use avian3d::prelude::*;
use bevy::prelude::*;

use super::Arena;
use super::meshes::{CYLINDER_RADIAL_SEGMENTS, build_cuboid_mesh, make_cylinder_mesh};
use crate::ragdoll::{OBJECTS_GROUP, WORLD_GROUP};

pub(super) struct Piece {
    transform: Transform,
    items: Vec<Item>,
}

enum Item {
    Cuboid(Cuboid),
    Lamp(Lamp),
    Torch(Torch),
}

struct Cuboid {
    position: Vec3,
    size: Vec3,
    material: Material,
}

struct Lamp {
    wire_length: f32,
}

struct Torch {
    position: Vec3,
}

impl Item {
    fn get_position(&self) -> Vec3 {
        match self {
            Item::Cuboid(build_cuboid) => build_cuboid.position,
            Item::Lamp(_) => Vec3::ZERO,
            Item::Torch(torch) => torch.position,
        }
    }
}

#[derive(Clone, PartialEq)]
pub(super) enum Material {
    Solid {
        color: Color,
    },
    Textured {
        base_color_texture: String,
        normal_map_texture: String,
        metallic_roughness_texture: Option<String>,
        roughness: f32,
        metallic: f32,
        tile_size: f32,
    },
}

const FLOOR_GREY: Color = Color::srgb(0.4, 0.4, 0.4);
const WALL_GREY: Color = Color::srgb(0.6, 0.6, 0.6);
const CUBOID_GREY: Color = Color::srgb(0.5, 0.5, 0.5);
const LAMP_SHADE_COLOR: Color = Color::srgb(0.02, 0.02, 0.02);
const LAMP_REFLECTOR_COLOR: Color = Color::srgb(0.92, 0.92, 0.88);
const LAMP_LIGHT_COLOR: Color = Color::srgb(1.0, 0.95, 0.88);
const LAMP_CONE_INTENSITY: f32 = 80_000.0;
const LAMP_CONE_RANGE: f32 = 9.0;
const LAMP_BOUNCE_INTENSITY: f32 = 60_000.0;
const LAMP_BOUNCE_RANGE: f32 = 9.0;
const LAMP_SHADE_RADIUS: f32 = 0.3;
const LAMP_SHADE_HEIGHT: f32 = 0.1;
const LAMP_REFLECTOR_RADIUS: f32 = 0.26;
const LAMP_REFLECTOR_HEIGHT: f32 = 0.02;
const LAMP_WIRE_RADIUS: f32 = 0.02;
const LAMP_MASS: f32 = 40.0;
const LAMP_DAMPING: f32 = 2.0;
const TORCH_HANDLE_COLOR: Color = Color::srgb(0.35, 0.2, 0.1);
const TORCH_FIRE_COLOR: Color = Color::srgb(1.0, 0.6, 0.2);
const TORCH_FIRE_INTENSITY: f32 = 900_000.0;
const TORCH_FIRE_RANGE: f32 = 12.0;
const TORCH_FIRE_RADIUS: f32 = 0.15;
const TORCH_HANDLE_SIZE: Vec3 = Vec3::new(0.15, 0.4, 0.15);
const TORCH_FIRE_OFFSET: Vec3 = Vec3::new(0.0, 0.3, -0.25);
const TEXTURES: &str = "maps/industrial/textures";

pub(super) const FLOOR_MATERIAL: Material = Material::Solid { color: FLOOR_GREY };
pub(super) const WALL_MATERIAL: Material = Material::Solid { color: WALL_GREY };
pub(super) const CUBOID_MATERIAL: Material = Material::Solid { color: CUBOID_GREY };

pub(super) struct PieceSpawnContext<'c, 'w, 's> {
    commands: &'c mut Commands<'w, 's>,
    asset_server: &'c AssetServer,
    meshes: &'c mut Assets<Mesh>,
    materials: &'c mut Assets<StandardMaterial>,
    material_cache: &'c mut Vec<(Material, Handle<StandardMaterial>)>,
}

impl<'c, 'w, 's> PieceSpawnContext<'c, 'w, 's> {
    pub(super) fn new(
        commands: &'c mut Commands<'w, 's>,
        asset_server: &'c AssetServer,
        meshes: &'c mut Assets<Mesh>,
        materials: &'c mut Assets<StandardMaterial>,
        material_cache: &'c mut Vec<(Material, Handle<StandardMaterial>)>,
    ) -> Self {
        Self {
            commands,
            asset_server,
            meshes,
            materials,
            material_cache,
        }
    }

    fn find_material_handle(&mut self, material: &Material) -> Handle<StandardMaterial> {
        if let Some((_, cached_handle)) = self.material_cache.iter().find(|(cached, _)| cached == material) {
            return cached_handle.clone();
        }
        let handle = self
            .materials
            .add(make_material_from_definition(self.asset_server, material));
        self.material_cache.push((material.clone(), handle.clone()));
        handle
    }

    fn spawn_mesh_cuboid(&mut self, transform: Transform, size: Vec3, material: &Material) {
        let handle = self.find_material_handle(material);
        self.commands.spawn((
            Arena,
            Mesh3d(self.meshes.add(build_cuboid_mesh(size, get_tile_size(material)))),
            MeshMaterial3d(handle),
            RigidBody::Static,
            Collider::cuboid(size.x, size.y, size.z),
            transform,
        ));
    }

    fn spawn_point_light(&mut self, transform: Transform, color: Color, intensity: f32, range: f32, radius: f32) {
        self.commands.spawn((
            Arena,
            PointLight {
                intensity,
                color,
                range,
                radius,
                shadow_maps_enabled: true,
                ..default()
            },
            transform,
        ));
    }
}

pub(super) fn spawn_piece(piece: &Piece, context: &mut PieceSpawnContext) {
    for item in &piece.items {
        let transform = piece.transform * Transform::from_translation(item.get_position());
        match item {
            Item::Cuboid(build_cuboid) => {
                context.spawn_mesh_cuboid(transform, build_cuboid.size, &build_cuboid.material)
            }
            Item::Lamp(lamp) => spawn_lamp(context, piece.transform, lamp.wire_length),
            Item::Torch(_) => spawn_torch(context, transform),
        }
    }
}

fn get_tile_size(material: &Material) -> f32 {
    match material {
        Material::Solid { .. } => 1.0,
        Material::Textured { tile_size, .. } => *tile_size,
    }
}

fn make_material_from_definition(asset_server: &AssetServer, material: &Material) -> StandardMaterial {
    match material {
        Material::Solid { color } => StandardMaterial {
            base_color: *color,
            perceptual_roughness: 0.9,
            ..default()
        },
        Material::Textured {
            base_color_texture,
            normal_map_texture,
            metallic_roughness_texture,
            roughness,
            metallic,
            ..
        } => StandardMaterial {
            base_color: Color::WHITE,
            base_color_texture: Some(asset_server.load(base_color_texture)),
            normal_map_texture: Some(asset_server.load(normal_map_texture)),
            metallic_roughness_texture: metallic_roughness_texture.as_ref().map(|path| asset_server.load(path)),
            perceptual_roughness: *roughness,
            metallic: *metallic,
            ..default()
        },
    }
}

fn spawn_lamp(context: &mut PieceSpawnContext, pivot_transform: Transform, wire_length: f32) {
    let pivot_position = pivot_transform.translation;
    let shade_material = context.find_material_handle(&Material::Solid {
        color: LAMP_SHADE_COLOR,
    });
    let reflector_material = context.find_material_handle(&Material::Solid {
        color: LAMP_REFLECTOR_COLOR,
    });
    let wire_material = shade_material.clone();

    let pivot_anchor = context
        .commands
        .spawn((RigidBody::Static, Transform::from_translation(pivot_position)))
        .id();
    let lamp_body = context
        .commands
        .spawn((
            RigidBody::Dynamic,
            Mass(LAMP_MASS),
            LinearDamping(LAMP_DAMPING),
            AngularDamping(LAMP_DAMPING),
            Visibility::default(),
            Transform::from_translation(pivot_position),
        ))
        .id();
    context.commands.spawn(SphericalJoint::new(pivot_anchor, lamp_body));

    let commands = &mut context.commands;
    let meshes = &mut *context.meshes;
    let shade_center = Vec3::Y * -(wire_length + LAMP_SHADE_HEIGHT * 0.5);
    let reflector_center = Vec3::Y * -(wire_length + LAMP_SHADE_HEIGHT + LAMP_REFLECTOR_HEIGHT * 0.5);
    let cone_position = wire_length + LAMP_SHADE_HEIGHT + LAMP_REFLECTOR_HEIGHT;
    commands.entity(lamp_body).with_children(|lamp_parts| {
        lamp_parts.spawn((
            Mesh3d(meshes.add(make_cylinder_mesh(
                LAMP_WIRE_RADIUS,
                wire_length,
                CYLINDER_RADIAL_SEGMENTS,
                1.0,
            ))),
            MeshMaterial3d(wire_material.clone()),
            Collider::cylinder(LAMP_WIRE_RADIUS, wire_length),
            CollisionLayers::new(OBJECTS_GROUP, WORLD_GROUP),
            Transform::from_translation(Vec3::Y * -(wire_length * 0.5)),
        ));
        lamp_parts.spawn((
            Mesh3d(meshes.add(make_cylinder_mesh(
                LAMP_SHADE_RADIUS,
                LAMP_SHADE_HEIGHT,
                CYLINDER_RADIAL_SEGMENTS,
                1.0,
            ))),
            MeshMaterial3d(shade_material.clone()),
            Collider::cylinder(LAMP_SHADE_RADIUS, LAMP_SHADE_HEIGHT),
            CollisionLayers::new(OBJECTS_GROUP, WORLD_GROUP),
            Transform::from_translation(shade_center),
        ));
        lamp_parts.spawn((
            Mesh3d(meshes.add(make_cylinder_mesh(
                LAMP_REFLECTOR_RADIUS,
                LAMP_REFLECTOR_HEIGHT,
                CYLINDER_RADIAL_SEGMENTS,
                1.0,
            ))),
            MeshMaterial3d(reflector_material.clone()),
            Transform::from_translation(reflector_center),
        ));
        lamp_parts.spawn((
            SpotLight {
                color: LAMP_LIGHT_COLOR,
                intensity: LAMP_CONE_INTENSITY,
                range: LAMP_CONE_RANGE,
                shadow_maps_enabled: true,
                ..default()
            },
            Transform::from_translation(Vec3::Y * -cone_position)
                .with_rotation(Quat::from_rotation_arc(Vec3::NEG_Z, Vec3::NEG_Y)),
        ));
    });
    context.spawn_point_light(
        Transform::from_translation(pivot_position + Vec3::Y * -(wire_length + 0.25)),
        LAMP_LIGHT_COLOR,
        LAMP_BOUNCE_INTENSITY,
        LAMP_BOUNCE_RANGE,
        0.25,
    );
}

fn spawn_torch(context: &mut PieceSpawnContext, transform: Transform) {
    context.spawn_mesh_cuboid(
        transform,
        TORCH_HANDLE_SIZE,
        &Material::Solid {
            color: TORCH_HANDLE_COLOR,
        },
    );
    context.spawn_point_light(
        transform * Transform::from_translation(TORCH_FIRE_OFFSET),
        TORCH_FIRE_COLOR,
        TORCH_FIRE_INTENSITY,
        TORCH_FIRE_RANGE,
        TORCH_FIRE_RADIUS,
    );
}

fn build_piece(transform: Transform, items: Vec<Item>) -> Piece {
    Piece { transform, items }
}

pub(super) fn build_cuboid(position: Vec3, size: Vec3, material: Material) -> Piece {
    build_piece(
        Transform::from_translation(position),
        vec![Item::Cuboid(Cuboid {
            position: Vec3::ZERO,
            size,
            material,
        })],
    )
}

pub(super) fn make_concrete_material(tile_size: f32) -> Material {
    Material::Textured {
        base_color_texture: format!("{TEXTURES}/concrete_floor/base_color.jpg"),
        normal_map_texture: format!("{TEXTURES}/concrete_floor/normal.png"),
        metallic_roughness_texture: None,
        roughness: 0.85,
        metallic: 0.0,
        tile_size,
    }
}

pub(super) fn build_stair(transform: Transform, width: f32, steps: u32, step_height: f32, step_depth: f32) -> Piece {
    let items = (0..steps)
        .map(|step_index| {
            let along = step_index as f32 + 0.5;
            Item::Cuboid(Cuboid {
                position: Vec3::NEG_Z * (step_depth * along) + Vec3::Y * (step_height * along),
                size: Vec3::new(width, step_height, step_depth),
                material: CUBOID_MATERIAL,
            })
        })
        .collect();
    build_piece(transform, items)
}

pub(super) fn build_door(transform: Transform, width: f32, height: f32, thickness: f32) -> Piece {
    let post_thickness = 0.5;
    let lintel_thickness = 0.5;
    let post_offset = (width + post_thickness) * 0.5;
    build_piece(
        transform,
        vec![
            Item::Cuboid(Cuboid {
                position: Vec3::new(-post_offset, height * 0.5, 0.0),
                size: Vec3::new(post_thickness, height, thickness),
                material: WALL_MATERIAL,
            }),
            Item::Cuboid(Cuboid {
                position: Vec3::new(post_offset, height * 0.5, 0.0),
                size: Vec3::new(post_thickness, height, thickness),
                material: WALL_MATERIAL,
            }),
            Item::Cuboid(Cuboid {
                position: Vec3::new(0.0, height + lintel_thickness * 0.5, 0.0),
                size: Vec3::new(width + post_thickness * 2.0, lintel_thickness, thickness),
                material: WALL_MATERIAL,
            }),
        ],
    )
}

pub(super) fn build_hanging_lamp(pivot_position: Vec3, wire_length: f32) -> Piece {
    build_piece(
        Transform::from_translation(pivot_position),
        vec![Item::Lamp(Lamp { wire_length })],
    )
}

pub(super) fn build_wall_torch(transform: Transform) -> Piece {
    build_piece(
        transform,
        vec![Item::Torch(Torch {
            position: Vec3::new(0.0, 2.2, -0.15),
        })],
    )
}
