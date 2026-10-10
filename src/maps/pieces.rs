use avian3d::prelude::*;
use bevy::asset::RenderAssetUsages;
use bevy::mesh::{Indices, PrimitiveTopology};
use bevy::prelude::*;

use super::meshes::{CYLINDER_RADIAL_SEGMENTS, build_cuboid_mesh, make_cylinder_mesh};
use crate::ragdoll::{OBJECTS_GROUP, WORLD_GROUP};

pub(super) struct Piece {
    transform: Transform,
    items: Vec<Item>,
}

enum Item {
    Cuboid(Cuboid),
    Lamp(Lamp),
}

struct Cuboid {
    position: Vec3,
    size: Vec3,
    material: Material,
}

struct Lamp {
    wire_length: f32,
}

impl Item {
    fn get_position(&self) -> Vec3 {
        match self {
            Item::Cuboid(build_cuboid) => build_cuboid.position,
            Item::Lamp(_) => Vec3::ZERO,
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
            Mesh3d(self.meshes.add(build_cuboid_mesh(size, get_tile_size(material)))),
            MeshMaterial3d(handle),
            RigidBody::Static,
            Collider::cuboid(size.x, size.y, size.z),
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
    let lamp_shade_color = Color::srgb(0.02, 0.02, 0.02);
    let lamp_light_color = Color::srgb(1.0, 0.95, 0.88);
    let lamp_reflector_color = Color::srgb(0.92, 0.92, 0.88);
    let lamp_shade_radius = 0.3;
    let lamp_shade_height = 0.1;
    let lamp_reflector_radius = 0.26;
    let lamp_reflector_height = 0.02;
    let lamp_wire_radius = 0.02;
    let lamp_shade_mass = 16.0;
    let links_per_meter = 10.0;
    let cable_particle_radius = 0.04;
    let link_mass_per_meter = 2.5;
    let lamp_linear_damping = 0.09;
    let lamp_angular_damping = 1.0;
    let link_linear_damping = 0.3;
    let link_angular_damping = 32.0;

    let shade_material = context.find_material_handle(&Material::Solid {
        color: lamp_shade_color,
    });
    let reflector_material = context.find_material_handle(&Material::Solid {
        color: lamp_reflector_color,
    });
    let wire_material = shade_material.clone();

    let pivot_anchor = context
        .commands
        .spawn((RigidBody::Static, Transform::from_translation(pivot_position)))
        .id();
    let link_count = (wire_length * links_per_meter).round() as usize;
    let link_length = wire_length / link_count as f32;

    let mut previous_body = pivot_anchor;
    let mut link_bodies = Vec::with_capacity(link_count);
    let mut link_positions = Vec::with_capacity(link_count);
    for link_index in 0..link_count {
        let link_position = pivot_position + Vec3::Y * -(link_index as f32 * link_length + link_length * 0.5);
        let link_body = context
            .commands
            .spawn((
                Name::new(format!("Lamp cable link {link_index}")),
                RigidBody::Dynamic,
                Transform::from_translation(link_position),
                Collider::sphere(cable_particle_radius),
                CollisionLayers::new(OBJECTS_GROUP, WORLD_GROUP),
                Mass(link_length * link_mass_per_meter),
                LinearDamping(link_linear_damping),
                AngularDamping(link_angular_damping),
            ))
            .id();
        let distance_to_previous_body = if link_index == 0 {
            link_length * 0.5
        } else {
            link_length
        };
        context.commands.spawn((
            DistanceJoint::new(previous_body, link_body)
                .with_limits(distance_to_previous_body, distance_to_previous_body),
            JointCollisionDisabled,
        ));
        link_bodies.push(link_body);
        link_positions.push(link_position);
        previous_body = link_body;
    }

    let shade_distance_below_pivot = wire_length + lamp_shade_height * 0.5;
    let shade_position = pivot_position + Vec3::Y * -shade_distance_below_pivot;
    let shade_collider = Collider::cylinder(lamp_shade_radius, lamp_shade_height);
    let lamp_body = context
        .commands
        .spawn((
            Name::new("Lamp shade"),
            RigidBody::Dynamic,
            HangingLamp,
            LampShade,
            Visibility::default(),
            Transform::from_translation(shade_position),
            Mesh3d(context.meshes.add(make_cylinder_mesh(
                lamp_shade_radius,
                lamp_shade_height,
                CYLINDER_RADIAL_SEGMENTS,
                1.0,
            ))),
            MeshMaterial3d(shade_material),
            shade_collider,
            Mass(lamp_shade_mass),
            LinearDamping(lamp_linear_damping),
            AngularDamping(lamp_angular_damping),
            CollisionLayers::new(OBJECTS_GROUP, WORLD_GROUP),
        ))
        .id();
    let shade_attachment_distance = link_length * 0.5;
    context.commands.spawn((
        DistanceJoint::new(previous_body, lamp_body)
            .with_local_anchor2(Vec3::Y * lamp_shade_height * 0.5)
            .with_limits(shade_attachment_distance, shade_attachment_distance),
        JointCollisionDisabled,
    ));

    let initial_wire_points = collect_lamp_wire_points(
        pivot_position,
        &link_positions,
        shade_position + Vec3::Y * lamp_shade_height * 0.5,
    );
    let wire_mesh = context
        .meshes
        .add(build_lamp_wire_mesh(&initial_wire_points, lamp_wire_radius));
    context.commands.spawn((
        LampWire {
            mesh: wire_mesh.clone(),
            links: link_bodies,
            ceiling_anchor: pivot_anchor,
            shade: lamp_body,
            shade_attachment: Vec3::Y * lamp_shade_height * 0.5,
            radius: lamp_wire_radius,
        },
        Mesh3d(wire_mesh),
        MeshMaterial3d(wire_material),
        Transform::default(),
        Visibility::default(),
    ));

    let commands = &mut context.commands;
    let meshes = &mut *context.meshes;
    let lamp_cone_intensity = 80_000.0;
    let lamp_cone_range = 9.0;
    let lamp_bounce_intensity = 60_000.0;
    let lamp_bounce_range = 9.0;
    let reflector_center = Vec3::Y * -(lamp_shade_height * 0.5 + lamp_reflector_height * 0.5);
    let cone_position = lamp_shade_height * 0.5 + lamp_reflector_height;
    commands.entity(lamp_body).with_children(|lamp_parts| {
        lamp_parts.spawn((
            Mesh3d(meshes.add(make_cylinder_mesh(
                lamp_reflector_radius,
                lamp_reflector_height,
                CYLINDER_RADIAL_SEGMENTS,
                1.0,
            ))),
            MeshMaterial3d(reflector_material.clone()),
            Transform::from_translation(reflector_center),
        ));
        lamp_parts.spawn((
            SpotLight {
                color: lamp_light_color,
                intensity: lamp_cone_intensity,
                range: lamp_cone_range,
                shadow_maps_enabled: true,
                ..default()
            },
            Transform::from_translation(Vec3::Y * -cone_position)
                .with_rotation(Quat::from_rotation_arc(Vec3::NEG_Z, Vec3::NEG_Y)),
        ));
        lamp_parts.spawn((
            PointLight {
                color: lamp_light_color,
                intensity: lamp_bounce_intensity,
                range: lamp_bounce_range,
                radius: 0.25,
                ..default()
            },
            Transform::from_translation(Vec3::Y * -(lamp_shade_height * 0.5 + 0.25)),
        ));
    });
}

#[derive(Component)]
pub struct HangingLamp;

#[derive(Component)]
pub struct LampShade;

#[derive(Component)]
pub(crate) struct LampWire {
    mesh: Handle<Mesh>,
    links: Vec<Entity>,
    ceiling_anchor: Entity,
    shade: Entity,
    shade_attachment: Vec3,
    radius: f32,
}

pub(crate) fn update_lamp_wires(
    lamp_wires: Query<&LampWire>,
    global_transforms: Query<&GlobalTransform>,
    mut meshes: ResMut<Assets<Mesh>>,
) {
    for lamp_wire in &lamp_wires {
        let Ok(ceiling_anchor_transform) = global_transforms.get(lamp_wire.ceiling_anchor) else {
            continue;
        };
        let Ok(shade_transform) = global_transforms.get(lamp_wire.shade) else {
            continue;
        };
        let mut link_positions = Vec::with_capacity(lamp_wire.links.len());
        let mut all_links_exist = true;
        for link in &lamp_wire.links {
            let Ok(link_transform) = global_transforms.get(*link) else {
                all_links_exist = false;
                break;
            };
            link_positions.push(link_transform.translation());
        }
        if !all_links_exist {
            continue;
        }
        let shade_attachment = shade_transform.transform_point(lamp_wire.shade_attachment);
        let wire_points = collect_lamp_wire_points(
            ceiling_anchor_transform.translation(),
            &link_positions,
            shade_attachment,
        );
        let Some(mut mesh) = meshes.get_mut(&lamp_wire.mesh) else {
            continue;
        };
        update_lamp_wire_mesh(&mut mesh, &wire_points, lamp_wire.radius);
    }
}

fn collect_lamp_wire_points(ceiling_anchor: Vec3, link_positions: &[Vec3], shade_attachment: Vec3) -> Vec<Vec3> {
    let mut wire_points = Vec::with_capacity(link_positions.len() + 2);
    wire_points.push(ceiling_anchor);
    wire_points.extend_from_slice(link_positions);
    wire_points.push(shade_attachment);
    wire_points
}

struct LampWireAttributes {
    positions: Vec<[f32; 3]>,
    normals: Vec<[f32; 3]>,
    uvs: Vec<[f32; 2]>,
}

fn build_lamp_wire_mesh(points: &[Vec3], radius: f32) -> Mesh {
    let wire_attributes = calculate_lamp_wire_attributes(points, radius);
    let indices = build_lamp_wire_indices(points.len());
    let mut mesh = Mesh::new(PrimitiveTopology::TriangleList, RenderAssetUsages::default());
    mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, wire_attributes.positions);
    mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, wire_attributes.normals);
    mesh.insert_attribute(Mesh::ATTRIBUTE_UV_0, wire_attributes.uvs);
    mesh.insert_indices(Indices::U32(indices));
    mesh
}

fn update_lamp_wire_mesh(mesh: &mut Mesh, points: &[Vec3], radius: f32) {
    let wire_attributes = calculate_lamp_wire_attributes(points, radius);
    mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, wire_attributes.positions);
    mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, wire_attributes.normals);
    mesh.insert_attribute(Mesh::ATTRIBUTE_UV_0, wire_attributes.uvs);
}

fn calculate_lamp_wire_attributes(points: &[Vec3], radius: f32) -> LampWireAttributes {
    let radial_segments = CYLINDER_RADIAL_SEGMENTS;
    let vertices_per_ring = radial_segments + 1;
    let vertex_count = points.len() * vertices_per_ring as usize;
    let mut positions = Vec::with_capacity(vertex_count);
    let mut normals = Vec::with_capacity(vertex_count);
    let mut uvs = Vec::with_capacity(vertex_count);
    let mut distance_along_wire = 0.0;
    let mut previous_radial_axis = None;

    for (point_index, point) in points.iter().enumerate() {
        if point_index > 0 {
            distance_along_wire += point.distance(points[point_index - 1]);
        }
        let tangent = calculate_lamp_wire_tangent(points, point_index);
        let radial_axis = calculate_lamp_wire_radial_axis(tangent, previous_radial_axis);
        let perpendicular_axis = tangent.cross(radial_axis).normalize_or_zero();
        previous_radial_axis = Some(radial_axis);
        for segment_index in 0..=radial_segments {
            let segment_fraction = segment_index as f32 / radial_segments as f32;
            let angle = std::f32::consts::TAU * segment_fraction;
            let (sine, cosine) = angle.sin_cos();
            let normal = radial_axis * cosine + perpendicular_axis * sine;
            positions.push((*point + normal * radius).into());
            normals.push(normal.into());
            uvs.push([distance_along_wire, segment_fraction]);
        }
    }

    LampWireAttributes {
        positions,
        normals,
        uvs,
    }
}

fn calculate_lamp_wire_tangent(points: &[Vec3], point_index: usize) -> Vec3 {
    let previous_point = points[point_index.saturating_sub(1)];
    let next_point = points[(point_index + 1).min(points.len() - 1)];
    let tangent = (next_point - previous_point).normalize_or_zero();
    if tangent == Vec3::ZERO { Vec3::Y } else { tangent }
}

fn calculate_lamp_wire_radial_axis(tangent: Vec3, previous_radial_axis: Option<Vec3>) -> Vec3 {
    if let Some(previous_radial_axis) = previous_radial_axis {
        let projected_radial_axis = previous_radial_axis - tangent * previous_radial_axis.dot(tangent);
        if projected_radial_axis.length_squared() > f32::EPSILON {
            return projected_radial_axis.normalize();
        }
    }
    let reference_axis = if tangent.dot(Vec3::Y).abs() > 0.99 {
        Vec3::X
    } else {
        Vec3::Y
    };
    tangent.cross(reference_axis).normalize()
}

fn build_lamp_wire_indices(point_count: usize) -> Vec<u32> {
    let radial_segments = CYLINDER_RADIAL_SEGMENTS;
    let vertices_per_ring = radial_segments + 1;
    let face_count = (point_count - 1) * radial_segments as usize;
    let mut indices = Vec::with_capacity(face_count * 6);
    for point_index in 0..point_count - 1 {
        let current_ring_start = point_index as u32 * vertices_per_ring;
        let next_ring_start = current_ring_start + vertices_per_ring;
        for segment_index in 0..radial_segments {
            indices.extend([
                current_ring_start + segment_index,
                next_ring_start + segment_index,
                next_ring_start + segment_index + 1,
                current_ring_start + segment_index,
                next_ring_start + segment_index + 1,
                current_ring_start + segment_index + 1,
            ]);
        }
    }
    indices
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

pub(super) fn build_default_cuboid(position: Vec3, size: Vec3) -> Piece {
    let default_cuboid_color = Color::srgb(0.5, 0.5, 0.5);
    let default_cuboid_material = Material::Solid {
        color: default_cuboid_color,
    };
    build_cuboid(position, size, default_cuboid_material)
}

pub(super) fn make_concrete_material(tile_size: f32) -> Material {
    let textures = "textures";

    Material::Textured {
        base_color_texture: format!("{textures}/concrete-floor/base-color.jpg"),
        normal_map_texture: format!("{textures}/concrete-floor/normal.png"),
        metallic_roughness_texture: None,
        roughness: 0.85,
        metallic: 0.0,
        tile_size,
    }
}

pub(super) fn build_hanging_lamp(pivot_position: Vec3, wire_length: f32) -> Piece {
    build_piece(
        Transform::from_translation(pivot_position),
        vec![Item::Lamp(Lamp { wire_length })],
    )
}
