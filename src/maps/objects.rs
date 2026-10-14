use avian3d::prelude::*;
use bevy::asset::RenderAssetUsages;
use bevy::gltf::GltfAssetLabel;
use bevy::light::VolumetricLight;
use bevy::mesh::{Indices, PrimitiveTopology};
use bevy::prelude::*;

use super::meshes::{CYLINDER_RADIAL_SEGMENTS, build_cuboid_mesh, make_cylinder_mesh};
use crate::ragdoll::{OBJECTS_GROUP, WORLD_GROUP};

#[derive(Clone)]
pub enum Object {
    Desk(Transform),
    Laptop(Transform),
    Stool(Transform),
    StorageRack(Transform),
    CardboardBox(Transform),
    Jerrycan(Transform),
    PlasticBottleGallon(Transform),
    PowerBox(Transform),
    Floor { transform: Transform, size: Vec3 },
    Ceiling { transform: Transform, size: Vec3 },
    Wall { transform: Transform, size: Vec3 },
    HangingLamp { transform: Transform, wire_length: f32 },
}

pub(super) struct ObjectSpawnContext<'commands, 'world, 'state> {
    commands: &'commands mut Commands<'world, 'state>,
    asset_server: &'commands AssetServer,
    meshes: &'commands mut Assets<Mesh>,
    materials: &'commands mut Assets<StandardMaterial>,
}

impl<'commands, 'world, 'state> ObjectSpawnContext<'commands, 'world, 'state> {
    pub(super) fn new(
        commands: &'commands mut Commands<'world, 'state>,
        asset_server: &'commands AssetServer,
        meshes: &'commands mut Assets<Mesh>,
        materials: &'commands mut Assets<StandardMaterial>,
    ) -> Self {
        Self {
            commands,
            asset_server,
            meshes,
            materials,
        }
    }
}

impl Object {
    pub(super) fn spawn(self, context: &mut ObjectSpawnContext) {
        match self {
            Self::Desk(transform) => spawn_desk(context, transform),
            Self::Laptop(transform) => spawn_laptop(context, transform),
            Self::Stool(transform) => spawn_stool(context, transform),
            Self::StorageRack(transform) => spawn_storage_rack(context, transform),
            Self::CardboardBox(transform) => spawn_cardboard_box(context, transform),
            Self::Jerrycan(transform) => spawn_jerrycan(context, transform),
            Self::PlasticBottleGallon(transform) => spawn_plastic_bottle_gallon(context, transform),
            Self::PowerBox(transform) => spawn_power_box(context, transform),
            Self::Floor { transform, size } => spawn_floor(context, transform, size),
            Self::Ceiling { transform, size } => spawn_ceiling(context, transform, size),
            Self::Wall { transform, size } => spawn_wall(context, transform, size),
            Self::HangingLamp { transform, wire_length } => spawn_hanging_lamp(context, transform, wire_length),
        }
    }
}

fn spawn_desk(context: &mut ObjectSpawnContext, transform: Transform) {
    spawn_dynamic_gltf_object(
        context,
        transform,
        "desk",
        "objects/table/metal_office_desk_2k.gltf",
        Collider::cuboid(2.0, 0.788, 0.947),
        Transform::from_translation(Vec3::new(0.0, 0.394, 0.0)),
        ColliderDensity(300.0),
    );
}

fn spawn_laptop(context: &mut ObjectSpawnContext, transform: Transform) {
    spawn_dynamic_gltf_object(
        context,
        transform,
        "laptop",
        "objects/classic-laptop/classic_laptop_2k.gltf",
        Collider::cuboid(0.66, 0.46, 0.49),
        Transform::from_translation(Vec3::new(0.0, 0.23, 0.0)),
        ColliderDensity(1000.0),
    );
}

fn spawn_stool(context: &mut ObjectSpawnContext, transform: Transform) {
    spawn_dynamic_gltf_object(
        context,
        transform,
        "stool",
        "objects/metal-stool/metal_stool_03_2k.gltf",
        Collider::cuboid(0.46, 1.024, 0.47),
        Transform::from_translation(Vec3::new(0.0, 0.512, 0.0)),
        ColliderDensity(500.0),
    );
}

fn spawn_storage_rack(context: &mut ObjectSpawnContext, transform: Transform) {
    spawn_dynamic_gltf_object(
        context,
        transform,
        "storage rack",
        "objects/worn-metal-rack/worn_metal_rack_2k.gltf",
        Collider::cuboid(0.92, 1.9, 0.6),
        Transform::from_translation(Vec3::new(0.0, 0.95, 0.0)),
        ColliderDensity(300.0),
    );
}

fn spawn_cardboard_box(context: &mut ObjectSpawnContext, transform: Transform) {
    spawn_dynamic_gltf_object(
        context,
        transform,
        "cardboard box",
        "objects/cardboard-box/cardboard_box_01_2k.gltf",
        Collider::cuboid(0.39, 0.34, 0.52),
        Transform::from_translation(Vec3::new(0.0, 0.17, 0.03)),
        ColliderDensity(100.0),
    );
}

fn spawn_jerrycan(context: &mut ObjectSpawnContext, transform: Transform) {
    spawn_dynamic_gltf_object(
        context,
        transform,
        "jerrycan",
        "objects/metal-jerrycan/metal_jerrycan_2k.gltf",
        Collider::cuboid(0.35, 0.455, 0.172),
        Transform::from_translation(Vec3::new(0.005, 0.228, -0.003)),
        ColliderDensity(500.0),
    );
}

fn spawn_plastic_bottle_gallon(context: &mut ObjectSpawnContext, transform: Transform) {
    spawn_dynamic_gltf_object(
        context,
        transform,
        "plastic bottle gallon",
        "objects/plastic-bottle-gallon/plastic_bottle_gallon_2k.gltf",
        Collider::cuboid(0.166, 0.275, 0.126),
        Transform::from_translation(Vec3::new(-0.005, 0.137, -0.002)),
        ColliderDensity(100.0),
    );
}

fn spawn_power_box(context: &mut ObjectSpawnContext, transform: Transform) {
    let asset_path = "objects/power-box/power_box_01_2k.gltf";
    let scene = context
        .asset_server
        .load(GltfAssetLabel::Scene(0).from_asset(asset_path));
    let collider = Collider::cuboid(0.35, 0.51, 0.12);
    let density = ColliderDensity(500.0);
    context
        .commands
        .spawn((
            Name::new("power box"),
            transform,
            WorldAssetRoot(scene),
            RigidBody::Static,
        ))
        .with_children(|object_parent| {
            object_parent.spawn((collider, density, CollisionLayers::new(WORLD_GROUP, OBJECTS_GROUP)));
        });
}

fn spawn_dynamic_gltf_object(
    context: &mut ObjectSpawnContext,
    transform: Transform,
    name: &'static str,
    asset_path: &'static str,
    collider: Collider,
    collider_transform: Transform,
    density: ColliderDensity,
) {
    let scene = context
        .asset_server
        .load(GltfAssetLabel::Scene(0).from_asset(asset_path));
    let object_linear_damping = 0.5;
    let object_angular_damping = 8.0;
    context
        .commands
        .spawn((
            Name::new(name),
            transform,
            WorldAssetRoot(scene),
            RigidBody::Dynamic,
            LinearDamping(object_linear_damping),
            AngularDamping(object_angular_damping),
        ))
        .with_children(|object_parent| {
            object_parent.spawn((
                collider,
                collider_transform,
                density,
                CollisionLayers::new(OBJECTS_GROUP, WORLD_GROUP | OBJECTS_GROUP),
            ));
        });
}

fn spawn_floor(context: &mut ObjectSpawnContext, transform: Transform, size: Vec3) {
    let concrete_tile_size = 2.0;
    spawn_concrete_cuboid(context, transform, size, concrete_tile_size);
}

fn spawn_ceiling(context: &mut ObjectSpawnContext, transform: Transform, size: Vec3) {
    let concrete_tile_size = 3.0;
    spawn_concrete_cuboid(context, transform, size, concrete_tile_size);
}

fn spawn_wall(context: &mut ObjectSpawnContext, transform: Transform, size: Vec3) {
    let concrete_tile_size = 3.0;
    spawn_concrete_cuboid(context, transform, size, concrete_tile_size);
}

fn spawn_concrete_cuboid(context: &mut ObjectSpawnContext, transform: Transform, size: Vec3, tile_size: f32) {
    let texture_directory = "textures/concrete-floor";
    let material = context.materials.add(StandardMaterial {
        base_color: Color::WHITE,
        base_color_texture: Some(context.asset_server.load(format!("{texture_directory}/base-color.jpg"))),
        normal_map_texture: Some(context.asset_server.load(format!("{texture_directory}/normal.png"))),
        perceptual_roughness: 0.85,
        ..default()
    });
    context.commands.spawn((
        Mesh3d(context.meshes.add(build_cuboid_mesh(size, tile_size))),
        MeshMaterial3d(material),
        RigidBody::Static,
        Collider::cuboid(size.x, size.y, size.z),
        transform,
    ));
}

fn spawn_hanging_lamp(context: &mut ObjectSpawnContext, pivot_transform: Transform, wire_length: f32) {
    let pivot_position = pivot_transform.translation;
    let lamp_shade_color = Color::srgb(0.02, 0.02, 0.02);
    let lamp_light_color = Color::srgb(1.0, 0.95, 0.88);
    let lamp_reflector_color = Color::srgb(0.08, 0.08, 0.07);
    let lamp_diffuser_color = lamp_light_color;
    let lamp_diffuser_luminance = 2_000.0;
    let lamp_shade_radius = 0.3;
    let lamp_shade_height = 0.1;
    let lamp_reflector_radius = 0.26;
    let lamp_reflector_height = 0.02;
    let lamp_diffuser_radius = 0.22;
    let lamp_diffuser_height = 0.01;
    let lamp_wire_radius = 0.02;
    let lamp_shade_mass = 16.0;
    let links_per_meter = 10.0;
    let cable_particle_radius = 0.04;
    let link_mass_per_meter = 2.5;
    let lamp_linear_damping = 0.09;
    let lamp_angular_damping = 1.0;
    let link_linear_damping = 0.3;
    let link_angular_damping = 32.0;

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
    let lamp_cone_intensity = 65_000.0;
    let lamp_cone_range = 7.0;
    let lamp_cone_inner_angle = 25.0_f32.to_radians();
    let lamp_cone_outer_angle = 55.0_f32.to_radians();
    let reflector_center = Vec3::Y * -(lamp_shade_height * 0.5 + lamp_reflector_height * 0.5);
    let diffuser_center = Vec3::Y * -(lamp_shade_height * 0.5 + lamp_reflector_height + lamp_diffuser_height * 0.5);
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
            Mesh3d(meshes.add(make_cylinder_mesh(
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
                color: lamp_light_color,
                intensity: lamp_cone_intensity,
                range: lamp_cone_range,
                inner_angle: lamp_cone_inner_angle,
                outer_angle: lamp_cone_outer_angle,
                shadow_maps_enabled: true,
                ..default()
            },
            VolumetricLight,
            Transform::from_translation(diffuser_center)
                .with_rotation(Quat::from_rotation_arc(Vec3::NEG_Z, Vec3::NEG_Y)),
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
