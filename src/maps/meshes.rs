use bevy::asset::RenderAssetUsages;
use bevy::mesh::{Indices, Mesh, PrimitiveTopology};
use bevy::prelude::*;

pub(super) const CYLINDER_RADIAL_SEGMENTS: u32 = 48;

pub(super) fn build_cuboid_mesh(size: Vec3, tile_size: f32) -> Mesh {
    let half = size * 0.5;
    let mut positions = Vec::new();
    let mut normals = Vec::new();
    let mut uvs = Vec::new();
    let mut indices = Vec::new();
    for (normal, tangent, bitangent) in build_cuboid_faces() {
        let face_center = normal * half.dot(normal.abs());
        let half_extent_along_tangent = size.dot(tangent.abs()) * 0.5;
        let half_extent_along_bitangent = size.dot(bitangent.abs()) * 0.5;
        let extent_along_tangent = half_extent_along_tangent * 2.0 / tile_size;
        let extent_along_bitangent = half_extent_along_bitangent * 2.0 / tile_size;
        let first_vertex = positions.len() as u32;
        for (tangent_sign, bitangent_sign) in [(-1.0, -1.0), (1.0, -1.0), (1.0, 1.0), (-1.0, 1.0)] {
            positions.push(
                (face_center
                    + tangent * half_extent_along_tangent * tangent_sign
                    + bitangent * half_extent_along_bitangent * bitangent_sign)
                    .into(),
            );
            normals.push(normal.into());
            uvs.push([
                extent_along_tangent * (tangent_sign * 0.5 + 0.5),
                extent_along_bitangent * (bitangent_sign * 0.5 + 0.5),
            ]);
        }
        indices.extend([
            first_vertex,
            first_vertex + 1,
            first_vertex + 2,
            first_vertex,
            first_vertex + 2,
            first_vertex + 3,
        ]);
    }
    make_triangle_list_mesh(positions, normals, uvs, indices)
}

fn build_cuboid_faces() -> [(Vec3, Vec3, Vec3); 6] {
    [
        (Vec3::X, -Vec3::Z, Vec3::Y),
        (-Vec3::X, Vec3::Z, Vec3::Y),
        (Vec3::Y, Vec3::X, -Vec3::Z),
        (-Vec3::Y, Vec3::X, Vec3::Z),
        (Vec3::Z, Vec3::X, Vec3::Y),
        (-Vec3::Z, -Vec3::X, Vec3::Y),
    ]
}

pub(super) fn make_cylinder_mesh(radius: f32, height: f32, radial_segments: u32, tile_size: f32) -> Mesh {
    let half_height = height * 0.5;
    let side_extent = 2.0 * std::f32::consts::PI * radius / tile_size;
    let cap_uv_radius = radius / tile_size;
    let mut positions = Vec::new();
    let mut normals = Vec::new();
    let mut uvs = Vec::new();
    let mut indices = Vec::new();

    for row in 0..=1u32 {
        let y = if row == 0 { -half_height } else { half_height };
        let v = if row == 0 { 0.0 } else { height / tile_size };
        for segment in 0..=radial_segments {
            let angle = 2.0 * std::f32::consts::PI * segment as f32 / radial_segments as f32;
            let (sine, cosine) = angle.sin_cos();
            positions.push([radius * cosine, y, radius * sine]);
            normals.push([cosine, 0.0, sine]);
            uvs.push([side_extent * segment as f32 / radial_segments as f32, v]);
        }
    }
    for segment in 0..radial_segments {
        let bottom_start = segment;
        let top_start = radial_segments + 1 + segment;
        indices.extend([
            bottom_start,
            top_start,
            top_start + 1,
            bottom_start,
            top_start + 1,
            bottom_start + 1,
        ]);
    }

    for (normal, center_y) in [(Vec3::Y, half_height), (-Vec3::Y, -half_height)] {
        let cap_center_vertex = positions.len() as u32;
        positions.push([0.0, center_y, 0.0]);
        normals.push(normal.into());
        uvs.push([0.5, 0.5]);
        let ring_base = positions.len() as u32;
        for segment in 0..=radial_segments {
            let angle = 2.0 * std::f32::consts::PI * segment as f32 / radial_segments as f32;
            let (sine, cosine) = angle.sin_cos();
            positions.push([radius * cosine, center_y, radius * sine]);
            normals.push(normal.into());
            uvs.push([0.5 + cosine * cap_uv_radius, 0.5 + sine * cap_uv_radius]);
        }
        for segment in 0..radial_segments {
            if normal == Vec3::Y {
                indices.extend([cap_center_vertex, ring_base + segment + 1, ring_base + segment]);
            } else {
                indices.extend([cap_center_vertex, ring_base + segment, ring_base + segment + 1]);
            }
        }
    }

    make_triangle_list_mesh(positions, normals, uvs, indices)
}

fn make_triangle_list_mesh(
    positions: Vec<[f32; 3]>,
    normals: Vec<[f32; 3]>,
    uvs: Vec<[f32; 2]>,
    indices: Vec<u32>,
) -> Mesh {
    let mut mesh = Mesh::new(PrimitiveTopology::TriangleList, RenderAssetUsages::default());
    mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, positions);
    mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, normals);
    mesh.insert_attribute(Mesh::ATTRIBUTE_UV_0, uvs);
    mesh.insert_indices(Indices::U32(indices));
    mesh
}
