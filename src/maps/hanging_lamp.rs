use avian3d::prelude::*;
use bevy::asset::RenderAssetUsages;
use bevy::camera::visibility::RenderLayers;
use bevy::light::VolumetricLight;
use bevy::mesh::{Indices, PrimitiveTopology};
use bevy::prelude::*;

use super::meshes::{CYLINDER_RADIAL_SEGMENTS, build_cylinder_mesh};
use super::{MapObject, SpawnContext};
use crate::ragdoll::{OBJECTS_GROUP, WORLD_GROUP};

const LAMP_SHADE_HEIGHT: f32 = 0.1;

pub struct HangingLamp {
    pub wire_length: f32,
}

impl MapObject for HangingLamp {
    fn spawn(&self, context: &mut SpawnContext, pivot_transform: Transform) {
        let pivot_position = pivot_transform.translation;
        let lamp_materials = build_lamp_materials(context);
        let pivot_anchor = context
            .commands
            .spawn((RigidBody::Static, Transform::from_translation(pivot_position)))
            .id();
        let cable = build_cable(context, pivot_position, self.wire_length);
        let lamp_body = spawn_lamp_shade(
            context,
            pivot_position,
            self.wire_length,
            cable.last_link,
            cable.link_length,
            lamp_materials.shade.clone(),
        );
        spawn_lamp_wire(
            context,
            pivot_position,
            self.wire_length,
            pivot_anchor,
            &cable,
            lamp_body,
            lamp_materials.shade,
        );
        spawn_lamp_light(context, lamp_body, lamp_materials.reflector, lamp_materials.diffuser);
    }
}

struct LampMaterials {
    shade: Handle<StandardMaterial>,
    reflector: Handle<StandardMaterial>,
    diffuser: Handle<StandardMaterial>,
}

struct LampCable {
    link_bodies: Vec<Entity>,
    link_positions: Vec<Vec3>,
    link_length: f32,
    last_link: Entity,
}

fn build_lamp_materials(context: &mut SpawnContext) -> LampMaterials {
    let lamp_shade_color = Color::srgb(0.02, 0.02, 0.02);
    let lamp_light_color = Color::srgb(1.0, 0.95, 0.88);
    let lamp_reflector_color = Color::srgb(0.08, 0.08, 0.07);
    let lamp_diffuser_color = lamp_light_color;
    let lamp_diffuser_luminance = 2_000.0;
    let shade_material = context.materials.add(StandardMaterial {
        base_color: lamp_shade_color,
        perceptual_roughness: 0.9,
        ..default()
    });
    let reflector_material = context.materials.add(StandardMaterial {
        base_color: lamp_reflector_color,
        perceptual_roughness: 0.9,
        ..default()
    });
    let diffuser_material = context.materials.add(StandardMaterial {
        base_color: lamp_diffuser_color,
        emissive: lamp_diffuser_color.to_linear() * lamp_diffuser_luminance,
        emissive_exposure_weight: 0.0,
        perceptual_roughness: 0.9,
        ..default()
    });
    LampMaterials {
        shade: shade_material,
        reflector: reflector_material,
        diffuser: diffuser_material,
    }
}

fn build_cable(context: &mut SpawnContext, pivot_position: Vec3, wire_length: f32) -> LampCable {
    let links_per_meter = 10.0;
    let cable_particle_radius = 0.04;
    let link_mass_per_meter = 2.5;
    let link_linear_damping = 0.3;
    let link_angular_damping = 32.0;
    let rounded_link_count = (wire_length * links_per_meter).round();
    let link_count = usize::try_from(rounded_link_count as i64).expect("lamp cable link count should fit into usize");
    let link_length = wire_length / link_count as f32;

    let mut previous_body_result = spawn_lamp_pivot_anchor(context, pivot_position);
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
            DistanceJoint::new(previous_body_result, link_body)
                .with_limits(distance_to_previous_body, distance_to_previous_body),
            JointCollisionDisabled,
        ));
        link_bodies.push(link_body);
        link_positions.push(link_position);
        previous_body_result = link_body;
    }
    LampCable {
        link_bodies,
        link_positions,
        link_length,
        last_link: previous_body_result,
    }
}

fn spawn_lamp_pivot_anchor(context: &mut SpawnContext, pivot_position: Vec3) -> Entity {
    context
        .commands
        .spawn((RigidBody::Static, Transform::from_translation(pivot_position)))
        .id()
}

fn spawn_lamp_shade(
    context: &mut SpawnContext,
    pivot_position: Vec3,
    wire_length: f32,
    previous_body: Entity,
    link_length: f32,
    shade_material: Handle<StandardMaterial>,
) -> Entity {
    let lamp_shade_radius = 0.3;
    let lamp_shade_mass = 16.0;
    let lamp_linear_damping = 0.09;
    let lamp_angular_damping = 1.0;
    let shade_distance_below_pivot = wire_length + LAMP_SHADE_HEIGHT * 0.5;
    let shade_position = pivot_position + Vec3::Y * -shade_distance_below_pivot;
    let shade_collider = Collider::cylinder(lamp_shade_radius, LAMP_SHADE_HEIGHT);
    let lamp_body = context
        .commands
        .spawn((
            Name::new("Lamp shade"),
            RigidBody::Dynamic,
            LampShade,
            Visibility::default(),
            Transform::from_translation(shade_position),
            Mesh3d(context.meshes.add(build_cylinder_mesh(
                lamp_shade_radius,
                LAMP_SHADE_HEIGHT,
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
            .with_local_anchor2(Vec3::Y * LAMP_SHADE_HEIGHT * 0.5)
            .with_limits(shade_attachment_distance, shade_attachment_distance),
        JointCollisionDisabled,
    ));
    lamp_body
}

fn spawn_lamp_wire(
    context: &mut SpawnContext,
    pivot_position: Vec3,
    wire_length: f32,
    pivot_anchor: Entity,
    cable: &LampCable,
    lamp_body: Entity,
    wire_material: Handle<StandardMaterial>,
) {
    let lamp_wire_radius = 0.02;
    let shade_distance_below_pivot = wire_length + LAMP_SHADE_HEIGHT * 0.5;
    let shade_position = pivot_position + Vec3::Y * -shade_distance_below_pivot;
    let shade_attachment_offset = Vec3::Y * LAMP_SHADE_HEIGHT * 0.5;
    let initial_wire_points = collect_lamp_wire_points(
        pivot_position,
        &cable.link_positions,
        shade_position + shade_attachment_offset,
    );
    let wire_mesh = context
        .meshes
        .add(build_lamp_wire_mesh(&initial_wire_points, lamp_wire_radius));
    context.commands.spawn((
        LampWire {
            mesh: wire_mesh.clone(),
            links: cable.link_bodies.clone(),
            ceiling_anchor: pivot_anchor,
            shade: lamp_body,
            shade_attachment: shade_attachment_offset,
            radius: lamp_wire_radius,
        },
        Mesh3d(wire_mesh),
        MeshMaterial3d(wire_material),
        Transform::default(),
        Visibility::default(),
    ));
}

fn spawn_lamp_light(
    context: &mut SpawnContext,
    lamp_body: Entity,
    reflector_material: Handle<StandardMaterial>,
    diffuser_material: Handle<StandardMaterial>,
) {
    let lamp_reflector_radius = 0.26;
    let lamp_reflector_height = 0.02;
    let lamp_diffuser_radius = 0.22;
    let lamp_diffuser_height = 0.01;
    let lamp_cone_intensity = 65_000.0;
    let lamp_cone_range = 7.0;
    let lamp_cone_inner_angle = 25.0_f32.to_radians();
    let lamp_cone_outer_angle = 55.0_f32.to_radians();
    let reflector_center = Vec3::Y * -(LAMP_SHADE_HEIGHT * 0.5 + lamp_reflector_height * 0.5);
    let diffuser_center = Vec3::Y * -(LAMP_SHADE_HEIGHT * 0.5 + lamp_reflector_height + lamp_diffuser_height * 0.5);
    let meshes = &mut *context.meshes;
    let commands = &mut *context.commands;
    commands.entity(lamp_body).with_children(|lamp_parts| {
        lamp_parts.spawn((
            Mesh3d(meshes.add(build_cylinder_mesh(
                lamp_reflector_radius,
                lamp_reflector_height,
                CYLINDER_RADIAL_SEGMENTS,
                1.0,
            ))),
            MeshMaterial3d(reflector_material.clone()),
            Transform::from_translation(reflector_center),
        ));
        lamp_parts.spawn((
            Mesh3d(meshes.add(build_cylinder_mesh(
                lamp_diffuser_radius,
                lamp_diffuser_height,
                CYLINDER_RADIAL_SEGMENTS,
                1.0,
            ))),
            MeshMaterial3d(diffuser_material),
            Transform::from_translation(diffuser_center),
        ));
        lamp_parts.spawn((
            SpotLight {
                color: Color::srgb(1.0, 0.95, 0.88),
                intensity: lamp_cone_intensity,
                range: lamp_cone_range,
                inner_angle: lamp_cone_inner_angle,
                outer_angle: lamp_cone_outer_angle,
                shadow_maps_enabled: true,
                ..default()
            },
            VolumetricLight,
            RenderLayers::layer(0).with(1),
            Transform::from_translation(diffuser_center)
                .with_rotation(Quat::from_rotation_arc(Vec3::NEG_Z, Vec3::NEG_Y)),
        ));
    });
}

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

pub(super) fn update_lamp_wires(
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
        let current_ring_start =
            u32::try_from(point_index).expect("lamp wire point count exceeds u32 index capacity") * vertices_per_ring;
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
