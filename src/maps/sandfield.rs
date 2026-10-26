use avian3d::prelude::{Collider, RigidBody};
use bevy::asset::RenderAssetUsages;
use bevy::mesh::{Indices, Mesh, Mesh3d, PrimitiveTopology};
use bevy::prelude::*;

use super::{MapObject, SpawnContext};

pub struct SandField {
    radius: f32,
    cutout: Option<TerrainCutout>,
}

pub struct TerrainCutout {
    vertices: Vec<Vec2>,
}

#[derive(Component)]
pub struct SandFieldSurface;

pub struct SandFieldGeometry {
    pub mesh: Mesh,
    pub collider: Collider,
}

struct SandFieldMeshData {
    positions: Vec<[f32; 3]>,
    normals: Vec<[f32; 3]>,
    ultraviolet_coordinates: Vec<[f32; 2]>,
    triangle_indices: Vec<u32>,
}

struct SandFieldGridVertexBuilder<'a> {
    grid_radius: i32,
    grid_vertex_width: usize,
    terrain_vertex_spacing: f32,
    sand_texture_tile_size: f32,
    grid_vertex_indices: &'a mut [Option<u32>],
    positions: &'a mut Vec<[f32; 3]>,
    ultraviolet_coordinates: &'a mut Vec<[f32; 2]>,
}

#[derive(Clone, Copy)]
struct ClippedTerrainVertex {
    position: Vec3,
    ultraviolet_coordinates: Vec2,
    original_index: Option<u32>,
}

impl SandField {
    pub fn create(radius: f32) -> Self {
        Self { radius, cutout: None }
    }

    pub fn with_cutout(mut self, cutout: TerrainCutout) -> Self {
        self.cutout = Some(cutout);
        self
    }

    pub fn calculate_height_at(horizontal_position: Vec2) -> f32 {
        terrain_height(horizontal_position)
    }

    pub fn generate_geometry(&self) -> SandFieldGeometry {
        let sand_field_mesh_data = build_mesh_data(self.radius, self.cutout.as_ref());
        let collider_vertices = sand_field_mesh_data
            .positions
            .iter()
            .map(|position| Vec3::from_array(*position))
            .collect();
        let collider_triangles = sand_field_mesh_data.triangle_indices.as_chunks::<3>().0.to_vec();
        let collider = Collider::trimesh(collider_vertices, collider_triangles);
        let mesh = build_render_mesh(sand_field_mesh_data);
        SandFieldGeometry { mesh, collider }
    }
}

impl TerrainCutout {
    pub fn new(vertices: Vec<Vec2>) -> Self {
        let minimum_convex_polygon_vertex_count = 3;
        let has_at_least_three_vertices = vertices.len() >= minimum_convex_polygon_vertex_count;
        let doubled_signed_area = if has_at_least_three_vertices {
            (0..vertices.len())
                .map(|vertex_index| {
                    let next_vertex_index = (vertex_index + 1) % vertices.len();
                    vertices[vertex_index].perp_dot(vertices[next_vertex_index])
                })
                .sum::<f32>()
        } else {
            0.0
        };
        let polygon_winding = doubled_signed_area.signum();
        let every_turn_matches_polygon_winding = has_at_least_three_vertices
            && doubled_signed_area != 0.0
            && (0..vertices.len()).all(|vertex_index| {
                let next_vertex_index = (vertex_index + 1) % vertices.len();
                let following_vertex_index = (vertex_index + 2) % vertices.len();
                let turn = (vertices[next_vertex_index] - vertices[vertex_index])
                    .perp_dot(vertices[following_vertex_index] - vertices[next_vertex_index]);
                turn * polygon_winding > 0.0
            });
        assert!(
            every_turn_matches_polygon_winding,
            "terrain cutout should be a convex polygon with at least three vertices"
        );
        Self { vertices }
    }
}

impl MapObject for SandField {
    fn spawn(&self, context: &mut SpawnContext, transform: Transform) {
        let SandFieldGeometry { mesh, collider } = self.generate_geometry();
        let sand_material = context.materials.add(StandardMaterial {
            base_color: Color::srgb(0.52, 0.39, 0.23),
            perceptual_roughness: 0.95,
            metallic: 0.0,
            ..default()
        });

        context.commands.spawn((
            Mesh3d(context.meshes.add(mesh)),
            MeshMaterial3d(sand_material),
            RigidBody::Static,
            collider,
            SandFieldSurface,
            transform,
        ));
    }
}

fn build_mesh_data(radius: f32, cutout: Option<&TerrainCutout>) -> SandFieldMeshData {
    let terrain_vertex_spacing = 2.0_f32;
    let grid_radius = (radius / terrain_vertex_spacing).ceil() as i32;
    let grid_vertex_width = usize::try_from(grid_radius * 2 + 1).expect("terrain grid width should be positive");
    let mut grid_vertex_indices = vec![None; grid_vertex_width * grid_vertex_width];
    let sand_texture_tile_size = 4.0;
    let mut positions = Vec::new();
    let mut ultraviolet_coordinates = Vec::new();
    let mut triangle_indices = Vec::new();

    {
        let mut grid_vertex_builder = SandFieldGridVertexBuilder {
            grid_radius,
            grid_vertex_width,
            terrain_vertex_spacing,
            sand_texture_tile_size,
            grid_vertex_indices: &mut grid_vertex_indices,
            positions: &mut positions,
            ultraviolet_coordinates: &mut ultraviolet_coordinates,
        };
        for grid_x in -grid_radius..grid_radius {
            for grid_z in -grid_radius..grid_radius {
                let cell_minimum_x = grid_x as f32 * terrain_vertex_spacing;
                let cell_minimum_z = grid_z as f32 * terrain_vertex_spacing;
                let cell_maximum_x = (grid_x + 1) as f32 * terrain_vertex_spacing;
                let cell_maximum_z = (grid_z + 1) as f32 * terrain_vertex_spacing;
                let closest_x = 0.0_f32.clamp(cell_minimum_x, cell_maximum_x);
                let closest_z = 0.0_f32.clamp(cell_minimum_z, cell_maximum_z);
                if Vec2::new(closest_x, closest_z).length_squared() > radius * radius {
                    continue;
                }
                let bottom_left_vertex = grid_vertex_builder.get_or_create_vertex(grid_x, grid_z);
                let bottom_right_vertex = grid_vertex_builder.get_or_create_vertex(grid_x + 1, grid_z);
                let top_left_vertex = grid_vertex_builder.get_or_create_vertex(grid_x, grid_z + 1);
                let top_right_vertex = grid_vertex_builder.get_or_create_vertex(grid_x + 1, grid_z + 1);
                triangle_indices.extend([
                    bottom_left_vertex,
                    top_left_vertex,
                    top_right_vertex,
                    bottom_left_vertex,
                    top_right_vertex,
                    bottom_right_vertex,
                ]);
            }
        }
    }

    if let Some(cutout) = cutout {
        clip_terrain_triangles(
            &mut positions,
            &mut ultraviolet_coordinates,
            &mut triangle_indices,
            cutout,
        );
    }
    let normals = calculate_vertex_normals(&positions, &triangle_indices);
    SandFieldMeshData {
        positions,
        normals,
        ultraviolet_coordinates,
        triangle_indices,
    }
}

impl SandFieldGridVertexBuilder<'_> {
    fn get_or_create_vertex(&mut self, grid_x: i32, grid_z: i32) -> u32 {
        let grid_column =
            usize::try_from(grid_x + self.grid_radius).expect("terrain grid column should be nonnegative");
        let grid_row = usize::try_from(grid_z + self.grid_radius).expect("terrain grid row should be nonnegative");
        let grid_vertex_slot = grid_row * self.grid_vertex_width + grid_column;
        if let Some(vertex_index) = self.grid_vertex_indices[grid_vertex_slot] {
            return vertex_index;
        }

        let horizontal_position = Vec2::new(
            grid_x as f32 * self.terrain_vertex_spacing,
            grid_z as f32 * self.terrain_vertex_spacing,
        );
        let vertex_index =
            u32::try_from(self.positions.len()).expect("terrain mesh vertex count should fit into u32 indices");
        self.positions.push([
            horizontal_position.x,
            terrain_height(horizontal_position),
            horizontal_position.y,
        ]);
        self.ultraviolet_coordinates.push([
            horizontal_position.x / self.sand_texture_tile_size,
            horizontal_position.y / self.sand_texture_tile_size,
        ]);
        self.grid_vertex_indices[grid_vertex_slot] = Some(vertex_index);
        vertex_index
    }
}

fn clip_terrain_triangles(
    positions: &mut Vec<[f32; 3]>,
    ultraviolet_coordinates: &mut Vec<[f32; 2]>,
    triangle_indices: &mut Vec<u32>,
    cutout: &TerrainCutout,
) {
    let cutout_winding = calculate_cutout_winding(&cutout.vertices);
    let original_triangle_indices = std::mem::take(triangle_indices);
    let mut clipped_triangle_indices = Vec::new();

    for triangle_vertex_indices in original_triangle_indices.as_chunks::<3>().0 {
        let terrain_triangle = triangle_vertex_indices.map(|vertex_index| {
            let vertex_array_index = usize::try_from(vertex_index).expect("terrain index should fit into usize");
            ClippedTerrainVertex {
                position: Vec3::from_array(positions[vertex_array_index]),
                ultraviolet_coordinates: Vec2::from_array(ultraviolet_coordinates[vertex_array_index]),
                original_index: Some(vertex_index),
            }
        });

        if is_triangle_outside_cutout(&terrain_triangle, &cutout.vertices, cutout_winding) {
            clipped_triangle_indices.extend(triangle_vertex_indices);
        } else if !is_triangle_inside_cutout(&terrain_triangle, &cutout.vertices, cutout_winding) {
            let mut remaining_inside_polygon = terrain_triangle.to_vec();
            for cutout_edge_index in 0..cutout.vertices.len() {
                let next_cutout_edge_index = (cutout_edge_index + 1) % cutout.vertices.len();
                let cutout_edge_start = cutout.vertices[cutout_edge_index];
                let cutout_edge_end = cutout.vertices[next_cutout_edge_index];
                let outside_fragment = clip_terrain_polygon_to_edge(
                    &remaining_inside_polygon,
                    cutout_edge_start,
                    cutout_edge_end,
                    cutout_winding,
                    false,
                );
                append_clipped_terrain_polygon(
                    &outside_fragment,
                    positions,
                    ultraviolet_coordinates,
                    &mut clipped_triangle_indices,
                );
                remaining_inside_polygon = clip_terrain_polygon_to_edge(
                    &remaining_inside_polygon,
                    cutout_edge_start,
                    cutout_edge_end,
                    cutout_winding,
                    true,
                );
                if remaining_inside_polygon.len() < 3 {
                    break;
                }
            }
        }
    }

    *triangle_indices = clipped_triangle_indices;
}

fn calculate_cutout_winding(vertices: &[Vec2]) -> f32 {
    let mut doubled_signed_area = 0.0;
    for vertex_index in 0..vertices.len() {
        let next_vertex_index = (vertex_index + 1) % vertices.len();
        doubled_signed_area += vertices[vertex_index].perp_dot(vertices[next_vertex_index]);
    }
    doubled_signed_area.signum()
}

fn is_triangle_outside_cutout(
    terrain_triangle: &[ClippedTerrainVertex; 3],
    cutout_vertices: &[Vec2],
    cutout_winding: f32,
) -> bool {
    for cutout_edge_index in 0..cutout_vertices.len() {
        let next_cutout_edge_index = (cutout_edge_index + 1) % cutout_vertices.len();
        let cutout_edge_start = cutout_vertices[cutout_edge_index];
        let cutout_edge_end = cutout_vertices[next_cutout_edge_index];
        if terrain_triangle.iter().all(|terrain_vertex| {
            calculate_cutout_edge_distance(
                terrain_vertex.position.xz(),
                cutout_edge_start,
                cutout_edge_end,
                cutout_winding,
            ) <= 0.0
        }) {
            return true;
        }
    }
    false
}

fn is_triangle_inside_cutout(
    terrain_triangle: &[ClippedTerrainVertex; 3],
    cutout_vertices: &[Vec2],
    cutout_winding: f32,
) -> bool {
    cutout_vertices
        .iter()
        .enumerate()
        .all(|(cutout_edge_index, cutout_edge_start)| {
            let next_cutout_edge_index = (cutout_edge_index + 1) % cutout_vertices.len();
            let cutout_edge_end = cutout_vertices[next_cutout_edge_index];
            terrain_triangle.iter().all(|terrain_vertex| {
                calculate_cutout_edge_distance(
                    terrain_vertex.position.xz(),
                    *cutout_edge_start,
                    cutout_edge_end,
                    cutout_winding,
                ) >= 0.0
            })
        })
}

fn calculate_cutout_edge_distance(
    horizontal_position: Vec2,
    cutout_edge_start: Vec2,
    cutout_edge_end: Vec2,
    cutout_winding: f32,
) -> f32 {
    (cutout_edge_end - cutout_edge_start).perp_dot(horizontal_position - cutout_edge_start) * cutout_winding
}

fn clip_terrain_polygon_to_edge(
    terrain_polygon: &[ClippedTerrainVertex],
    cutout_edge_start: Vec2,
    cutout_edge_end: Vec2,
    cutout_winding: f32,
    retain_inside: bool,
) -> Vec<ClippedTerrainVertex> {
    let mut clipped_polygon = Vec::with_capacity(terrain_polygon.len() + 1);
    let mut previous_vertex = *terrain_polygon
        .last()
        .expect("terrain polygon should contain vertices before clipping");
    let mut previous_distance = calculate_cutout_edge_distance(
        previous_vertex.position.xz(),
        cutout_edge_start,
        cutout_edge_end,
        cutout_winding,
    );
    let mut previous_vertex_is_inside = previous_distance >= 0.0;

    for current_vertex in terrain_polygon {
        let current_distance = calculate_cutout_edge_distance(
            current_vertex.position.xz(),
            cutout_edge_start,
            cutout_edge_end,
            cutout_winding,
        );
        let current_vertex_is_inside = current_distance >= 0.0;
        let previous_vertex_is_retained = previous_vertex_is_inside == retain_inside;
        let current_vertex_is_retained = current_vertex_is_inside == retain_inside;

        if current_vertex_is_retained {
            if !previous_vertex_is_retained {
                clipped_polygon.push(calculate_terrain_edge_intersection(
                    previous_vertex,
                    *current_vertex,
                    previous_distance,
                    current_distance,
                ));
            }
            clipped_polygon.push(*current_vertex);
        } else if previous_vertex_is_retained {
            clipped_polygon.push(calculate_terrain_edge_intersection(
                previous_vertex,
                *current_vertex,
                previous_distance,
                current_distance,
            ));
        }

        previous_vertex = *current_vertex;
        previous_distance = current_distance;
        previous_vertex_is_inside = current_vertex_is_inside;
    }

    clipped_polygon
}

fn calculate_terrain_edge_intersection(
    previous_vertex: ClippedTerrainVertex,
    current_vertex: ClippedTerrainVertex,
    previous_distance: f32,
    current_distance: f32,
) -> ClippedTerrainVertex {
    let interpolation_factor = previous_distance / (previous_distance - current_distance);
    ClippedTerrainVertex {
        position: previous_vertex
            .position
            .lerp(current_vertex.position, interpolation_factor),
        ultraviolet_coordinates: previous_vertex
            .ultraviolet_coordinates
            .lerp(current_vertex.ultraviolet_coordinates, interpolation_factor),
        original_index: None,
    }
}

fn append_clipped_terrain_polygon(
    terrain_polygon: &[ClippedTerrainVertex],
    positions: &mut Vec<[f32; 3]>,
    ultraviolet_coordinates: &mut Vec<[f32; 2]>,
    triangle_indices: &mut Vec<u32>,
) {
    if terrain_polygon.len() < 3 {
        return;
    }

    let terrain_polygon_vertex_indices = terrain_polygon
        .iter()
        .map(|terrain_vertex| {
            if let Some(original_index) = terrain_vertex.original_index {
                original_index
            } else {
                let vertex_index =
                    u32::try_from(positions.len()).expect("terrain mesh vertex count should fit into u32 indices");
                positions.push(terrain_vertex.position.to_array());
                ultraviolet_coordinates.push(terrain_vertex.ultraviolet_coordinates.to_array());
                vertex_index
            }
        })
        .collect::<Vec<_>>();
    for polygon_vertex_index in 1..terrain_polygon_vertex_indices.len() - 1 {
        triangle_indices.extend([
            terrain_polygon_vertex_indices[0],
            terrain_polygon_vertex_indices[polygon_vertex_index],
            terrain_polygon_vertex_indices[polygon_vertex_index + 1],
        ]);
    }
}

fn terrain_height(horizontal_position: Vec2) -> f32 {
    let main_dune_wavelength = 45.0;
    let main_dune_height = 1.5;
    let secondary_dune_wavelength = 18.0;
    let secondary_dune_height = 0.3;
    let irregularity_height = 0.15;
    let wind_direction = Vec2::new(0.8, 0.6).normalize();
    let crossing_wave_direction = Vec2::new(-wind_direction.y, wind_direction.x);
    let irregularity_direction = Vec2::new(0.42, 0.91).normalize();
    let crossing_irregularity_direction = Vec2::new(-irregularity_direction.y, irregularity_direction.x);
    let irregularity_wavelength = 73.0;
    let crossing_irregularity_wavelength = 61.0;
    let main_dune_wave = (horizontal_position.dot(wind_direction) * std::f32::consts::TAU / main_dune_wavelength).sin()
        * main_dune_height;
    let crossing_dune_wave =
        (horizontal_position.dot(crossing_wave_direction) * std::f32::consts::TAU / secondary_dune_wavelength).sin()
            * secondary_dune_height;
    let low_frequency_variation =
        (horizontal_position.dot(irregularity_direction) * std::f32::consts::TAU / irregularity_wavelength).sin()
            * (horizontal_position.dot(crossing_irregularity_direction) * std::f32::consts::TAU
                / crossing_irregularity_wavelength)
                .cos()
            * irregularity_height;

    main_dune_wave + crossing_dune_wave + low_frequency_variation
}

fn calculate_vertex_normals(positions: &[[f32; 3]], triangle_indices: &[u32]) -> Vec<[f32; 3]> {
    let mut accumulated_normals = vec![Vec3::ZERO; positions.len()];
    for [first_triangle_index, second_triangle_index, third_triangle_index] in triangle_indices.as_chunks::<3>().0 {
        let first_vertex_index = usize::try_from(*first_triangle_index).expect("terrain index should fit into usize");
        let second_vertex_index = usize::try_from(*second_triangle_index).expect("terrain index should fit into usize");
        let third_vertex_index = usize::try_from(*third_triangle_index).expect("terrain index should fit into usize");
        let first_vertex = Vec3::from_array(positions[first_vertex_index]);
        let second_vertex = Vec3::from_array(positions[second_vertex_index]);
        let third_vertex = Vec3::from_array(positions[third_vertex_index]);
        let triangle_normal = (second_vertex - first_vertex).cross(third_vertex - first_vertex);
        accumulated_normals[first_vertex_index] += triangle_normal;
        accumulated_normals[second_vertex_index] += triangle_normal;
        accumulated_normals[third_vertex_index] += triangle_normal;
    }

    accumulated_normals
        .into_iter()
        .map(|normal| normal.normalize_or_zero().to_array())
        .collect()
}

fn build_render_mesh(mesh_data: SandFieldMeshData) -> Mesh {
    let mut mesh = Mesh::new(PrimitiveTopology::TriangleList, RenderAssetUsages::default());
    mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, mesh_data.positions);
    mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, mesh_data.normals);
    mesh.insert_attribute(Mesh::ATTRIBUTE_UV_0, mesh_data.ultraviolet_coordinates);
    mesh.insert_indices(Indices::U32(mesh_data.triangle_indices));
    mesh
}
