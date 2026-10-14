use bevy::asset::RenderAssetUsages;
use bevy::camera::visibility::DynamicSkinnedMeshBounds;
use bevy::ecs::system::SystemParam;
use bevy::mesh::skinning::{SkinnedMesh, SkinnedMeshInverseBindposes};
use bevy::mesh::{Indices, Mesh, Mesh3d, MeshVertexAttribute, PrimitiveTopology, VertexAttributeValues};
use bevy::prelude::*;
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};

use crate::enemies::Enemy;

const BLOOD_MARK_MAXIMUM_PER_CHARACTER: usize = 16;
const BLOOD_MARK_SURFACE_OFFSET: f32 = 0.001;

pub struct BloodPlugin;

impl Plugin for BloodPlugin {
    fn build(&self, app: &mut App) {
        app.add_message::<BulletHitCharacter>()
            .init_resource::<BloodMarkMaterial>()
            .add_systems(Update, draw_blood_marks.after(crate::shooting::shoot));
    }
}

#[derive(Message, Clone, Copy)]
pub(crate) struct BulletHitCharacter {
    pub(crate) character: Entity,
    pub(crate) origin: Vec3,
    pub(crate) direction: Dir3,
    pub(crate) maximum_distance: f32,
    pub(crate) collider_point: Vec3,
}

#[derive(Component, Default)]
pub(crate) struct BloodMarkCount(pub(crate) usize);

#[derive(Component)]
struct BloodMark;

#[derive(Resource)]
struct BloodMarkMaterial {
    material: Handle<StandardMaterial>,
}

impl FromWorld for BloodMarkMaterial {
    fn from_world(world: &mut World) -> Self {
        let texture_size = 32_u32;
        let mut pixels = Vec::with_capacity((texture_size * texture_size * 4) as usize);
        for vertical_pixel in 0..texture_size {
            for horizontal_pixel in 0..texture_size {
                let coordinate = Vec2::new(horizontal_pixel as f32, vertical_pixel as f32) / (texture_size - 1) as f32
                    * 2.0
                    - Vec2::ONE;
                let distance = coordinate.length();
                let alpha = if distance < 0.62 {
                    255
                } else if distance < 0.88 {
                    ((0.88 - distance) / 0.26 * 255.0) as u8
                } else {
                    0
                };
                pixels.extend([74, 0, 0, alpha]);
            }
        }
        let texture = Image::new(
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
        let texture = world.resource_mut::<Assets<Image>>().add(texture);
        let material = world.resource_mut::<Assets<StandardMaterial>>().add(StandardMaterial {
            base_color_texture: Some(texture),
            alpha_mode: AlphaMode::Blend,
            perceptual_roughness: 1.0,
            cull_mode: None,
            ..default()
        });
        Self { material }
    }
}

type SkinQuery<'world, 'state> =
    Query<'world, 'state, (Entity, &'static Mesh3d, &'static SkinnedMesh, &'static GlobalTransform)>;
type BloodMeshAssets<'w, 's> = ParamSet<'w, 's, (Res<'w, Assets<Mesh>>, ResMut<'w, Assets<Mesh>>)>;

#[derive(SystemParam)]
struct BloodDrawingContext<'w, 's> {
    blood_mark_counts: Query<'w, 's, &'static mut BloodMarkCount>,
    enemies: Query<'w, 's, (), With<Enemy>>,
    children: Query<'w, 's, &'static Children>,
    skins: SkinQuery<'w, 's>,
    transforms: Query<'w, 's, &'static GlobalTransform>,
    mesh_assets: BloodMeshAssets<'w, 's>,
    inverse_bindposes: Res<'w, Assets<SkinnedMeshInverseBindposes>>,
    blood_mark_material: Res<'w, BloodMarkMaterial>,
}

struct SkinSurfaceContext<'world, 'state> {
    children: &'world Query<'world, 'state, &'static Children>,
    skins: &'world SkinQuery<'world, 'state>,
    transforms: &'world Query<'world, 'state, &'static GlobalTransform>,
    mesh_assets: &'world Assets<Mesh>,
    inverse_bindposes: &'world Assets<SkinnedMeshInverseBindposes>,
}

fn draw_blood_marks(
    mut commands: Commands,
    mut hits: MessageReader<BulletHitCharacter>,
    mut context: BloodDrawingContext,
) {
    for hit in hits.read() {
        if !context.enemies.contains(hit.character) {
            continue;
        }
        let Ok(mut count) = context.blood_mark_counts.get_mut(hit.character) else {
            continue;
        };
        if count.0 >= BLOOD_MARK_MAXIMUM_PER_CHARACTER {
            continue;
        }
        let surface_context = SkinSurfaceContext {
            children: &context.children,
            skins: &context.skins,
            transforms: &context.transforms,
            mesh_assets: &context.mesh_assets.p0(),
            inverse_bindposes: &context.inverse_bindposes,
        };
        let Some(surface_hit) = find_surface_hit(*hit, surface_context) else {
            continue;
        };
        let mesh_entity = surface_hit.mesh_entity;
        let skin = surface_hit.skin.clone();
        let mut mesh = build_blood_mark_mesh(surface_hit);
        mesh.generate_skinned_mesh_bounds()
            .expect("blood mark mesh should have valid skinning attributes");
        let blood_mark = commands
            .spawn((
                Name::new("Blood mark"),
                BloodMark,
                Mesh3d(context.mesh_assets.p1().add(mesh)),
                MeshMaterial3d(context.blood_mark_material.material.clone()),
                DynamicSkinnedMeshBounds,
                skin,
                Transform::default(),
            ))
            .id();
        commands.entity(mesh_entity).add_child(blood_mark);
        count.0 += 1;
    }
}

struct SurfaceHit {
    mesh_entity: Entity,
    skin: SkinnedMesh,
    positions: [Vec3; 3],
    normals: [Vec3; 3],
    joint_indices: [[u16; 4]; 3],
    joint_weights: [[f32; 4]; 3],
    barycentric: Vec3,
}

fn find_surface_hit(hit: BulletHitCharacter, context: SkinSurfaceContext) -> Option<SurfaceHit> {
    let character = hit.character;
    let origin = hit.origin;
    let direction = hit.direction.as_vec3();
    let maximum_distance = hit.maximum_distance;
    let collider_point = hit.collider_point;
    let children = context.children;
    let skins = context.skins;
    let transforms = context.transforms;
    let mesh_assets = context.mesh_assets;
    let inverse_bindposes = context.inverse_bindposes;
    let mut nearest_hit: Option<(f32, SurfaceHit)> = None;
    let mut nearest_collider_surface: Option<(f32, SurfaceHit)> = None;
    for (mesh_entity, mesh_handle, skin, _) in skins {
        if !is_descendant_of(mesh_entity, character, children) {
            continue;
        }
        let Some(mesh) = mesh_assets.get(&mesh_handle.0) else {
            continue;
        };
        let Some(bindposes) = inverse_bindposes.get(&skin.inverse_bindposes) else {
            continue;
        };
        let Some(positions) = attribute_vec3(mesh, Mesh::ATTRIBUTE_POSITION) else {
            continue;
        };
        let Some(normals) = attribute_vec3(mesh, Mesh::ATTRIBUTE_NORMAL) else {
            continue;
        };
        let Some(joint_indices) = attribute_joint_indices(mesh) else {
            continue;
        };
        let Some(joint_weights) = attribute_joint_weights(mesh) else {
            continue;
        };
        let Some(indices) = triangle_indices(mesh) else {
            continue;
        };
        for triangle in indices.as_chunks::<3>().0 {
            let [first, second, third] = [triangle[0] as usize, triangle[1] as usize, triangle[2] as usize];
            let Some([first_position, second_position, third_position]) = skin_triangle_positions(
                [
                    positions[first].into(),
                    positions[second].into(),
                    positions[third].into(),
                ],
                [joint_indices[first], joint_indices[second], joint_indices[third]],
                [joint_weights[first], joint_weights[second], joint_weights[third]],
                skin,
                bindposes,
                transforms,
            ) else {
                continue;
            };
            let surface_hit = SurfaceHit {
                mesh_entity,
                skin: skin.clone(),
                positions: [
                    positions[first].into(),
                    positions[second].into(),
                    positions[third].into(),
                ],
                normals: [normals[first].into(), normals[second].into(), normals[third].into()],
                joint_indices: [joint_indices[first], joint_indices[second], joint_indices[third]],
                joint_weights: [joint_weights[first], joint_weights[second], joint_weights[third]],
                barycentric: Vec3::splat(1.0 / 3.0),
            };
            if let Some((distance, barycentric)) =
                intersect_ray_triangle(origin, direction, first_position, second_position, third_position)
                && distance <= maximum_distance
                && nearest_hit
                    .as_ref()
                    .is_none_or(|(nearest_distance, _)| distance < *nearest_distance)
            {
                nearest_hit = Some((
                    distance,
                    SurfaceHit {
                        barycentric,
                        ..surface_hit
                    },
                ));
                continue;
            }
            let triangle_center = (first_position + second_position + third_position) / 3.0;
            let collider_distance_squared = triangle_center.distance_squared(collider_point);
            if nearest_collider_surface
                .as_ref()
                .is_none_or(|(nearest_distance_squared, _)| collider_distance_squared < *nearest_distance_squared)
            {
                nearest_collider_surface = Some((collider_distance_squared, surface_hit));
            }
        }
    }
    nearest_hit.or(nearest_collider_surface).map(|(_, hit)| hit)
}

fn is_descendant_of(entity: Entity, ancestor: Entity, children: &Query<&Children>) -> bool {
    children.iter_descendants(ancestor).any(|child| child == entity)
}

fn skin_triangle_positions(
    positions: [Vec3; 3],
    joint_indices: [[u16; 4]; 3],
    joint_weights: [[f32; 4]; 3],
    skin: &SkinnedMesh,
    bindposes: &SkinnedMeshInverseBindposes,
    transforms: &Query<&GlobalTransform>,
) -> Option<[Vec3; 3]> {
    positions
        .into_iter()
        .zip(joint_indices)
        .zip(joint_weights)
        .map(|((position, indices), weights)| skin_position(position, indices, weights, skin, bindposes, transforms))
        .collect::<Option<Vec<_>>>()
        .and_then(|positions| positions.try_into().ok())
}

fn skin_position(
    position: Vec3,
    joint_indices: [u16; 4],
    joint_weights: [f32; 4],
    skin: &SkinnedMesh,
    bindposes: &SkinnedMeshInverseBindposes,
    transforms: &Query<&GlobalTransform>,
) -> Option<Vec3> {
    let mut skinned_position = Vec3::ZERO;
    for (joint_index, weight) in joint_indices.into_iter().zip(joint_weights) {
        if weight == 0.0 {
            continue;
        }
        let joint = *skin.joints.get(joint_index as usize)?;
        let joint_transform = transforms.get(joint).ok()?.affine();
        let bindpose = *bindposes.get(joint_index as usize)?;
        skinned_position += (joint_transform * bindpose).transform_point3(position) * weight;
    }
    Some(skinned_position)
}

fn intersect_ray_triangle(
    origin: Vec3,
    direction: Vec3,
    first: Vec3,
    second: Vec3,
    third: Vec3,
) -> Option<(f32, Vec3)> {
    let first_edge = second - first;
    let second_edge = third - first;
    let determinant = direction.dot(first_edge.cross(second_edge));
    if determinant.abs() < f32::EPSILON {
        return None;
    }
    let inverse_determinant = determinant.recip();
    let origin_offset = origin - first;
    let second_weight = origin_offset.dot(direction.cross(second_edge)) * inverse_determinant;
    if !(0.0..=1.0).contains(&second_weight) {
        return None;
    }
    let third_weight = direction.dot(origin_offset.cross(first_edge)) * inverse_determinant;
    if third_weight < 0.0 || second_weight + third_weight > 1.0 {
        return None;
    }
    let distance = second_edge.dot(origin_offset.cross(first_edge)) * inverse_determinant;
    (distance >= 0.0).then_some((
        distance,
        Vec3::new(1.0 - second_weight - third_weight, second_weight, third_weight),
    ))
}

fn build_blood_mark_mesh(hit: SurfaceHit) -> Mesh {
    let center = hit.positions[0] * hit.barycentric.x
        + hit.positions[1] * hit.barycentric.y
        + hit.positions[2] * hit.barycentric.z;
    let center_normal =
        (hit.normals[0] * hit.barycentric.x + hit.normals[1] * hit.barycentric.y + hit.normals[2] * hit.barycentric.z)
            .normalize_or_zero();
    let first_edge_length = hit.positions[0].distance(hit.positions[1]);
    let second_edge_length = hit.positions[1].distance(hit.positions[2]);
    let third_edge_length = hit.positions[2].distance(hit.positions[0]);
    let longest_triangle_edge_length = first_edge_length.max(second_edge_length).max(third_edge_length);
    let bullet_wound_diameter = 0.08;
    let blood_mark_scale = bullet_wound_diameter / longest_triangle_edge_length.max(f32::EPSILON);
    let positions: Vec<[f32; 3]> = hit
        .positions
        .into_iter()
        .map(|position| center.lerp(position, blood_mark_scale) + center_normal * BLOOD_MARK_SURFACE_OFFSET)
        .map(|position| position.to_array())
        .collect();
    let normals: Vec<[f32; 3]> = hit.normals.into_iter().map(|normal| normal.to_array()).collect();
    let mut mesh = Mesh::new(PrimitiveTopology::TriangleList, RenderAssetUsages::RENDER_WORLD);
    mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, positions);
    mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, normals);
    mesh.insert_attribute(Mesh::ATTRIBUTE_UV_0, vec![[0.5, 0.0], [0.0, 1.0], [1.0, 1.0]]);
    mesh.insert_attribute(
        Mesh::ATTRIBUTE_JOINT_INDEX,
        VertexAttributeValues::Uint16x4(hit.joint_indices.to_vec()),
    );
    mesh.insert_attribute(
        Mesh::ATTRIBUTE_JOINT_WEIGHT,
        VertexAttributeValues::Float32x4(hit.joint_weights.to_vec()),
    );
    mesh.insert_indices(Indices::U32(vec![0, 1, 2]));
    mesh
}

fn attribute_vec3(mesh: &Mesh, attribute: MeshVertexAttribute) -> Option<&Vec<[f32; 3]>> {
    match mesh.try_attribute(attribute).ok()? {
        VertexAttributeValues::Float32x3(values) => Some(values),
        _ => None,
    }
}

fn attribute_joint_indices(mesh: &Mesh) -> Option<&Vec<[u16; 4]>> {
    match mesh.try_attribute(Mesh::ATTRIBUTE_JOINT_INDEX).ok()? {
        VertexAttributeValues::Uint16x4(values) => Some(values),
        _ => None,
    }
}

fn attribute_joint_weights(mesh: &Mesh) -> Option<&Vec<[f32; 4]>> {
    match mesh.try_attribute(Mesh::ATTRIBUTE_JOINT_WEIGHT).ok()? {
        VertexAttributeValues::Float32x4(values) => Some(values),
        _ => None,
    }
}

fn triangle_indices(mesh: &Mesh) -> Option<Vec<u32>> {
    match mesh.indices()? {
        Indices::U16(indices) => Some(indices.iter().map(|&index| index as u32).collect()),
        Indices::U32(indices) => Some(indices.clone()),
    }
}
