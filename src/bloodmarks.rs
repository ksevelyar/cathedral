use bevy::asset::RenderAssetUsages;
use bevy::camera::visibility::DynamicSkinnedMeshBounds;
use bevy::ecs::system::SystemParam;
use bevy::image::{ImageAddressMode, ImageSampler, ImageSamplerDescriptor};
use bevy::log::error;
use bevy::math::Mat4;
use bevy::mesh::skinning::{SkinnedMesh, SkinnedMeshInverseBindposes};
use bevy::mesh::{Indices, Mesh, Mesh3d, PrimitiveTopology, VertexAttributeValues};
use bevy::prelude::*;
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};

use std::collections::hash_map::Entry;
use std::collections::{HashMap, VecDeque};
use std::f32::consts::TAU;

use crate::enemies::{Enemy, EnemyHit};

const BLOOD_MARK_MAXIMUM_PER_CHARACTER: usize = 24;
const BLOOD_MARK_SURFACE_OFFSET: f32 = 0.001;

pub struct BloodmarksPlugin;

impl Plugin for BloodmarksPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<BloodMarkMaterial>()
            .init_resource::<BloodMarkSeed>()
            .init_resource::<PreparedBloodMarkMeshes>()
            .add_systems(PreUpdate, invalidate_prepared_blood_mark_meshes)
            .add_observer(draw_blood_mark);
    }
}

#[derive(Component, Default)]
pub(crate) struct BloodMarks(pub(crate) VecDeque<Entity>);

#[derive(Component)]
struct BloodMark;

#[derive(Resource, Default)]
struct BloodMarkSeed(u64);

#[derive(Resource, Default)]
struct PreparedBloodMarkMeshes {
    meshes: HashMap<AssetId<Mesh>, PreparedBloodMarkMesh>,
}

struct PreparedBloodMarkMesh {
    vertices: Vec<PreparedBloodMarkVertex>,
    triangles: Vec<[usize; 3]>,
    vertex_triangles: Vec<Vec<usize>>,
}

#[derive(Clone, Copy)]
struct PreparedBloodMarkVertex {
    position: Vec3,
    normal: Vec3,
    joint_indices: [u16; 4],
    joint_weights: [f32; 4],
}

#[derive(Resource)]
struct BloodMarkMaterial {
    material: Handle<StandardMaterial>,
}

impl FromWorld for BloodMarkMaterial {
    fn from_world(world: &mut World) -> Self {
        let texture_size = 64_u32;
        let entry_wound_radius = 1.5 / 4.0;
        let mut pixels = Vec::with_capacity((texture_size * texture_size * 4) as usize);
        for vertical_pixel in 0..texture_size {
            for horizontal_pixel in 0..texture_size {
                let coordinate = Vec2::new(horizontal_pixel as f32, vertical_pixel as f32) / (texture_size - 1) as f32
                    * 2.0
                    - Vec2::ONE;
                let distance = coordinate.length();
                let edge_noise = calculate_edge_noise(coordinate);
                let spray_edge = 0.72 + edge_noise * 0.18;
                let spray_alpha = if distance <= entry_wound_radius {
                    220
                } else if distance < spray_edge {
                    ((spray_edge - distance) / 0.14 * 150.0).clamp(0.0, 150.0) as u8
                } else {
                    0
                };
                let color = if distance <= entry_wound_radius {
                    [36, 1, 1]
                } else {
                    [55, 3, 2]
                };
                pixels.extend([color[0], color[1], color[2], spray_alpha]);
            }
        }
        let mut texture = Image::new(
            Extent3d {
                width: texture_size,
                height: texture_size,
                depth_or_array_layers: 1,
            },
            TextureDimension::D2,
            pixels,
            TextureFormat::Rgba8UnormSrgb,
            RenderAssetUsages::RENDER_WORLD,
        );
        texture.sampler = ImageSampler::Descriptor(ImageSamplerDescriptor {
            address_mode_u: ImageAddressMode::ClampToEdge,
            address_mode_v: ImageAddressMode::ClampToEdge,
            ..ImageSamplerDescriptor::linear()
        });
        let texture = world.resource_mut::<Assets<Image>>().add(texture);
        let material = world.resource_mut::<Assets<StandardMaterial>>().add(StandardMaterial {
            base_color_texture: Some(texture),
            alpha_mode: AlphaMode::Blend,
            unlit: true,
            perceptual_roughness: 1.0,
            cull_mode: None,
            ..default()
        });
        Self { material }
    }
}

fn calculate_edge_noise(coordinate: Vec2) -> f32 {
    let angle = coordinate.y.atan2(coordinate.x);
    let first_wave = (angle * 7.0).sin() * 0.5 + 0.5;
    let second_wave = (angle * 13.0 + 1.7).sin() * 0.5 + 0.5;
    first_wave * 0.65 + second_wave * 0.35
}

fn invalidate_prepared_blood_mark_meshes(
    mut mesh_asset_events: MessageReader<AssetEvent<Mesh>>,
    mut prepared_blood_mark_meshes: ResMut<PreparedBloodMarkMeshes>,
) {
    for mesh_asset_event in mesh_asset_events.read() {
        let (AssetEvent::Modified { id } | AssetEvent::Removed { id }) = mesh_asset_event else {
            continue;
        };
        prepared_blood_mark_meshes.meshes.remove(id);
    }
}

#[derive(SystemParam)]
struct TargetSurfaces<'w, 's> {
    children: Query<'w, 's, &'static Children>,
    skins: Query<'w, 's, (&'static Mesh3d, &'static SkinnedMesh), Without<BloodMark>>,
    transforms: Query<'w, 's, &'static GlobalTransform>,
    inverse_bindposes: Res<'w, Assets<SkinnedMeshInverseBindposes>>,
}

#[derive(SystemParam)]
struct BloodMarkDrawing<'w> {
    mesh_assets: ResMut<'w, Assets<Mesh>>,
    prepared_blood_mark_meshes: ResMut<'w, PreparedBloodMarkMeshes>,
    blood_mark_material: Res<'w, BloodMarkMaterial>,
    blood_mark_seed: ResMut<'w, BloodMarkSeed>,
}

struct SurfaceHit {
    mesh_entity: Entity,
    skin: SkinnedMesh,
    prepared_mesh_asset_id: AssetId<Mesh>,
    posed_positions: Vec<Vec3>,
    hit_triangle_index: usize,
    point: Vec3,
    normal: Vec3,
}

fn draw_blood_mark(
    hit: On<EnemyHit>,
    mut commands: Commands,
    mut blood_marks: Query<&mut BloodMarks, With<Enemy>>,
    target_surfaces: TargetSurfaces,
    mut blood_mark_drawing: BloodMarkDrawing,
) {
    let Ok(mut blood_marks) = blood_marks.get_mut(hit.target) else {
        return;
    };
    let Some(surface_hit) = target_surfaces.find_hit(
        &hit,
        &blood_mark_drawing.mesh_assets,
        &mut blood_mark_drawing.prepared_blood_mark_meshes,
    ) else {
        error!("enemy collider hit without an intersected posed skin: {:?}", hit.target);
        return;
    };
    let prepared_mesh = blood_mark_drawing
        .prepared_blood_mark_meshes
        .meshes
        .get(&surface_hit.prepared_mesh_asset_id)
        .expect("a surface hit should retain its prepared mesh");
    let blood_mark = spawn_blood_mark(
        &mut commands,
        &mut blood_mark_drawing.mesh_assets,
        &blood_mark_drawing.blood_mark_material,
        prepared_mesh,
        &surface_hit,
        hit.weapon_caliber,
        take_blood_mark_seed(&mut blood_mark_drawing.blood_mark_seed),
    );
    retain_blood_mark(&mut commands, &mut blood_marks, blood_mark);
}

fn take_blood_mark_seed(blood_mark_seed: &mut BloodMarkSeed) -> u64 {
    let variation_seed = blood_mark_seed.0;
    blood_mark_seed.0 = blood_mark_seed.0.wrapping_add(1);
    variation_seed
}

fn spawn_blood_mark(
    commands: &mut Commands,
    mesh_assets: &mut Assets<Mesh>,
    blood_mark_material: &BloodMarkMaterial,
    prepared_mesh: &PreparedBloodMarkMesh,
    surface_hit: &SurfaceHit,
    caliber: f32,
    variation_seed: u64,
) -> Entity {
    let mut mesh = build_blood_mark_mesh(prepared_mesh, surface_hit, caliber, variation_seed);
    mesh.generate_skinned_mesh_bounds()
        .expect("blood mark mesh should have valid skinning attributes");
    let blood_mark = commands
        .spawn((
            Name::new("Blood mark"),
            BloodMark,
            Mesh3d(mesh_assets.add(mesh)),
            MeshMaterial3d(blood_mark_material.material.clone()),
            DynamicSkinnedMeshBounds,
            surface_hit.skin.clone(),
            Transform::default(),
        ))
        .id();
    commands.entity(surface_hit.mesh_entity).add_child(blood_mark);
    blood_mark
}

fn retain_blood_mark(commands: &mut Commands, blood_marks: &mut BloodMarks, blood_mark: Entity) {
    if blood_marks.0.len() == BLOOD_MARK_MAXIMUM_PER_CHARACTER {
        let oldest_blood_mark = blood_marks
            .0
            .pop_front()
            .expect("blood mark limit should have an oldest mark");
        commands.entity(oldest_blood_mark).despawn();
    }
    blood_marks.0.push_back(blood_mark);
}

impl TargetSurfaces<'_, '_> {
    fn find_hit(
        &self,
        hit: &EnemyHit,
        mesh_assets: &Assets<Mesh>,
        prepared_blood_mark_meshes: &mut PreparedBloodMarkMeshes,
    ) -> Option<SurfaceHit> {
        let maximum_surface_distance = hit.impact_distance + 1.0;
        let mut nearest_hit: Option<(f32, SurfaceHit)> = None;
        for mesh_entity in self.children.iter_descendants(hit.target) {
            let Ok((mesh_handle, skin)) = self.skins.get(mesh_entity) else {
                continue;
            };
            let Some(mesh) = mesh_assets.get(&mesh_handle.0) else {
                continue;
            };
            let Some(bindposes) = self.inverse_bindposes.get(&skin.inverse_bindposes) else {
                continue;
            };
            let prepared_mesh_asset_id = mesh_handle.0.id();
            let Some(prepared_mesh) =
                get_prepared_blood_mark_mesh(prepared_blood_mark_meshes, prepared_mesh_asset_id, mesh)
            else {
                continue;
            };
            let Some(joint_matrices) = self.resolve_joint_matrices(skin, bindposes) else {
                continue;
            };
            let Some(posed_positions) = pose_prepared_vertices(&prepared_mesh.vertices, &joint_matrices) else {
                continue;
            };
            let Some((distance, hit_triangle_index)) = find_nearest_mesh_hit(
                hit,
                maximum_surface_distance,
                &prepared_mesh.triangles,
                &posed_positions,
            ) else {
                continue;
            };
            if nearest_hit
                .as_ref()
                .is_some_and(|(nearest_distance, _)| distance >= *nearest_distance)
            {
                continue;
            }
            let point = hit.shot_origin + hit.shot_direction.as_vec3() * distance;
            let [first, second, third] = prepared_mesh.triangles[hit_triangle_index];
            let normal = (posed_positions[second] - posed_positions[first])
                .cross(posed_positions[third] - posed_positions[first])
                .normalize_or_zero();
            nearest_hit = Some((
                distance,
                SurfaceHit {
                    mesh_entity,
                    skin: skin.clone(),
                    prepared_mesh_asset_id,
                    posed_positions,
                    hit_triangle_index,
                    point,
                    normal,
                },
            ));
        }
        nearest_hit.map(|(_, surface_hit)| surface_hit)
    }

    fn resolve_joint_matrices(&self, skin: &SkinnedMesh, bindposes: &SkinnedMeshInverseBindposes) -> Option<Vec<Mat4>> {
        if skin.joints.len() != bindposes.len() {
            return None;
        }
        skin.joints
            .iter()
            .copied()
            .zip(bindposes.iter().copied())
            .map(|(joint, bindpose)| Some(self.transforms.get(joint).ok()?.affine() * bindpose))
            .collect()
    }
}

fn get_prepared_blood_mark_mesh<'a>(
    prepared_blood_mark_meshes: &'a mut PreparedBloodMarkMeshes,
    mesh_asset_id: AssetId<Mesh>,
    mesh: &Mesh,
) -> Option<&'a PreparedBloodMarkMesh> {
    if let Entry::Vacant(vacant_mesh_entry) = prepared_blood_mark_meshes.meshes.entry(mesh_asset_id) {
        let prepared_mesh = prepare_blood_mark_mesh(mesh)?;
        vacant_mesh_entry.insert(prepared_mesh);
    }
    prepared_blood_mark_meshes.meshes.get(&mesh_asset_id)
}

fn prepare_blood_mark_mesh(mesh: &Mesh) -> Option<PreparedBloodMarkMesh> {
    let VertexAttributeValues::Float32x3(positions) = mesh.try_attribute(Mesh::ATTRIBUTE_POSITION).ok()? else {
        return None;
    };
    let VertexAttributeValues::Float32x3(normals) = mesh.try_attribute(Mesh::ATTRIBUTE_NORMAL).ok()? else {
        return None;
    };
    let VertexAttributeValues::Uint16x4(joint_indices) = mesh.try_attribute(Mesh::ATTRIBUTE_JOINT_INDEX).ok()? else {
        return None;
    };
    let VertexAttributeValues::Float32x4(joint_weights) = mesh.try_attribute(Mesh::ATTRIBUTE_JOINT_WEIGHT).ok()? else {
        return None;
    };
    let vertex_count = positions.len();
    if normals.len() != vertex_count || joint_indices.len() != vertex_count || joint_weights.len() != vertex_count {
        return None;
    }
    let triangles = prepare_triangles(mesh.indices(), vertex_count)?;
    let vertices = positions
        .iter()
        .copied()
        .zip(normals.iter().copied())
        .zip(joint_indices.iter().copied())
        .zip(joint_weights.iter().copied())
        .map(
            |(((position, normal), joint_indices), joint_weights)| PreparedBloodMarkVertex {
                position: position.into(),
                normal: normal.into(),
                joint_indices,
                joint_weights,
            },
        )
        .collect();
    let mut vertex_triangles = vec![Vec::new(); vertex_count];
    for (triangle_index, triangle) in triangles.iter().enumerate() {
        for vertex_index in triangle {
            vertex_triangles[*vertex_index].push(triangle_index);
        }
    }
    Some(PreparedBloodMarkMesh {
        vertices,
        triangles,
        vertex_triangles,
    })
}

fn prepare_triangles(indices: Option<&Indices>, vertex_count: usize) -> Option<Vec<[usize; 3]>> {
    let triangles: Vec<[usize; 3]> = match indices {
        Some(Indices::U16(indices)) => indices
            .as_chunks::<3>()
            .0
            .iter()
            .map(|triangle| triangle.map(usize::from))
            .collect(),
        Some(Indices::U32(indices)) => indices
            .as_chunks::<3>()
            .0
            .iter()
            .map(|triangle| triangle.map(|index| index as usize))
            .collect(),
        None if vertex_count.is_multiple_of(3) => (0..vertex_count)
            .step_by(3)
            .map(|first| [first, first + 1, first + 2])
            .collect(),
        None => return None,
    };
    triangles
        .iter()
        .all(|triangle: &[usize; 3]| triangle.iter().all(|vertex_index| *vertex_index < vertex_count))
        .then_some(triangles)
}

fn pose_prepared_vertices(prepared_vertices: &[PreparedBloodMarkVertex], joint_matrices: &[Mat4]) -> Option<Vec<Vec3>> {
    prepared_vertices
        .iter()
        .map(|prepared_vertex| {
            prepared_vertex
                .joint_indices
                .into_iter()
                .zip(prepared_vertex.joint_weights)
                .try_fold(Vec3::ZERO, |posed_position, (joint_index, joint_weight)| {
                    if joint_weight == 0.0 {
                        return Some(posed_position);
                    }
                    let joint_matrix = joint_matrices.get(joint_index as usize)?;
                    Some(posed_position + joint_matrix.transform_point3(prepared_vertex.position) * joint_weight)
                })
        })
        .collect()
}

fn find_nearest_mesh_hit(
    hit: &EnemyHit,
    maximum_surface_distance: f32,
    triangles: &[[usize; 3]],
    posed_positions: &[Vec3],
) -> Option<(f32, usize)> {
    let mut nearest_mesh_hit: Option<(f32, usize)> = None;
    for (triangle_index, [first, second, third]) in triangles.iter().copied().enumerate() {
        let Some(distance) = intersect_ray_triangle(
            hit.shot_origin,
            hit.shot_direction.as_vec3(),
            posed_positions[first],
            posed_positions[second],
            posed_positions[third],
        ) else {
            continue;
        };
        if distance > maximum_surface_distance
            || nearest_mesh_hit
                .as_ref()
                .is_some_and(|(nearest_distance, _)| distance >= *nearest_distance)
        {
            continue;
        }
        nearest_mesh_hit = Some((distance, triangle_index));
    }
    nearest_mesh_hit
}

fn collect_patch_triangles(
    prepared_mesh: &PreparedBloodMarkMesh,
    hit: &SurfaceHit,
    outer_spray_radius: f32,
) -> Vec<usize> {
    let mut triangle_indices = vec![hit.hit_triangle_index];
    let mut included_triangles = vec![false; prepared_mesh.triangles.len()];
    included_triangles[hit.hit_triangle_index] = true;
    let mut triangle_frontier = VecDeque::from([hit.hit_triangle_index]);
    while let Some(triangle_index) = triangle_frontier.pop_front() {
        for vertex_index in prepared_mesh.triangles[triangle_index] {
            for adjacent_triangle_index in &prepared_mesh.vertex_triangles[vertex_index] {
                if included_triangles[*adjacent_triangle_index]
                    || !triangle_reaches_spray(
                        prepared_mesh.triangles[*adjacent_triangle_index],
                        &hit.posed_positions,
                        hit.point,
                        outer_spray_radius,
                    )
                {
                    continue;
                }
                included_triangles[*adjacent_triangle_index] = true;
                triangle_indices.push(*adjacent_triangle_index);
                triangle_frontier.push_back(*adjacent_triangle_index);
            }
        }
    }
    triangle_indices
}

fn triangle_reaches_spray(
    triangle: [usize; 3],
    posed_positions: &[Vec3],
    point: Vec3,
    outer_spray_radius: f32,
) -> bool {
    triangle
        .into_iter()
        .any(|vertex_index| posed_positions[vertex_index].distance(point) <= outer_spray_radius)
}

fn build_blood_mark_mesh(
    prepared_mesh: &PreparedBloodMarkMesh,
    hit: &SurfaceHit,
    caliber: f32,
    variation_seed: u64,
) -> Mesh {
    let outer_spray_radius = caliber * 4.0 * 0.5;
    let source_triangle_indices = collect_patch_triangles(prepared_mesh, hit, outer_spray_radius);
    let reference_axis = if hit.normal.abs_diff_eq(Vec3::Y, 0.001) || hit.normal.abs_diff_eq(Vec3::NEG_Y, 0.001) {
        Vec3::X
    } else {
        Vec3::Y
    };
    let tangent = hit.normal.cross(reference_axis).normalize_or_zero();
    let bitangent = hit.normal.cross(tangent).normalize_or_zero();
    let mixed_seed = variation_seed.wrapping_mul(0x9E37_79B9_7F4A_7C15);
    let rotation_angle = ((mixed_seed ^ (mixed_seed >> 29)) >> 40) as f32 / ((1_u32 << 24) - 1) as f32 * TAU;
    let (rotation_sine, rotation_cosine) = rotation_angle.sin_cos();
    let mut copied_vertices = HashMap::new();
    let mut positions = Vec::new();
    let mut normals = Vec::new();
    let mut ultraviolet_coordinates = Vec::new();
    let mut joint_indices = Vec::new();
    let mut joint_weights = Vec::new();
    let mut indices = Vec::new();
    for source_triangle_index in source_triangle_indices {
        for source_vertex_index in prepared_mesh.triangles[source_triangle_index] {
            let copied_vertex_index = if let Some(copied_vertex_index) = copied_vertices.get(&source_vertex_index) {
                *copied_vertex_index
            } else {
                let source_vertex = prepared_mesh.vertices[source_vertex_index];
                let projected_offset = hit.posed_positions[source_vertex_index] - hit.point;
                let projected_coordinate = Vec2::new(projected_offset.dot(tangent), projected_offset.dot(bitangent));
                let rotated_coordinate = Vec2::new(
                    projected_coordinate.x * rotation_cosine - projected_coordinate.y * rotation_sine,
                    projected_coordinate.x * rotation_sine + projected_coordinate.y * rotation_cosine,
                );
                let ultraviolet_coordinate = rotated_coordinate / (outer_spray_radius * 2.0) + Vec2::splat(0.5);
                let copied_vertex_index = positions.len() as u32;
                copied_vertices.insert(source_vertex_index, copied_vertex_index);
                positions.push((source_vertex.position + source_vertex.normal * BLOOD_MARK_SURFACE_OFFSET).to_array());
                normals.push(source_vertex.normal.to_array());
                ultraviolet_coordinates.push(ultraviolet_coordinate.to_array());
                joint_indices.push(source_vertex.joint_indices);
                joint_weights.push(source_vertex.joint_weights);
                copied_vertex_index
            };
            indices.push(copied_vertex_index);
        }
    }
    let mut mesh = Mesh::new(PrimitiveTopology::TriangleList, RenderAssetUsages::RENDER_WORLD);
    mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, positions);
    mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, normals);
    mesh.insert_attribute(Mesh::ATTRIBUTE_UV_0, ultraviolet_coordinates);
    mesh.insert_attribute(
        Mesh::ATTRIBUTE_JOINT_INDEX,
        VertexAttributeValues::Uint16x4(joint_indices),
    );
    mesh.insert_attribute(
        Mesh::ATTRIBUTE_JOINT_WEIGHT,
        VertexAttributeValues::Float32x4(joint_weights),
    );
    mesh.insert_indices(Indices::U32(indices));
    mesh
}

fn intersect_ray_triangle(origin: Vec3, direction: Vec3, first: Vec3, second: Vec3, third: Vec3) -> Option<f32> {
    let first_edge = second - first;
    let second_edge = third - first;
    let perpendicular = direction.cross(second_edge);
    let determinant = first_edge.dot(perpendicular);
    if determinant.abs() < f32::EPSILON {
        return None;
    }
    let inverse_determinant = determinant.recip();
    let origin_offset = origin - first;
    let second_weight = origin_offset.dot(perpendicular) * inverse_determinant;
    if !(0.0..=1.0).contains(&second_weight) {
        return None;
    }
    let perpendicular_offset = origin_offset.cross(first_edge);
    let third_weight = direction.dot(perpendicular_offset) * inverse_determinant;
    if third_weight < 0.0 || second_weight + third_weight > 1.0 {
        return None;
    }
    let distance = second_edge.dot(perpendicular_offset) * inverse_determinant;
    (distance >= 0.0).then_some(distance)
}
