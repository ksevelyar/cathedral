use avian3d::prelude::{Collider, RigidBody};
use bevy::asset::RenderAssetUsages;
use bevy::mesh::{Indices, Mesh, Mesh3d, PrimitiveTopology};
use bevy::prelude::*;

use super::{MapObject, SpawnContext};

pub struct SandField {
    radius: f32,
    openings: Vec<SandFieldOpening>,
}

pub struct SandFieldOpening {
    center: Vec2,
    half_size: Vec2,
}

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

impl SandField {
    pub fn create(radius: f32) -> Self {
        Self {
            radius,
            openings: Vec::new(),
        }
    }

    pub fn with_opening(mut self, opening: SandFieldOpening) -> Self {
        self.openings.push(opening);
        self
    }

    pub fn calculate_height_at(horizontal_position: Vec2) -> f32 {
        terrain_height(horizontal_position)
    }

    pub fn generate_geometry(&self) -> SandFieldGeometry {
        let sand_field_mesh_data = build_mesh_data(self.radius, &self.openings);
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

impl SandFieldOpening {
    pub fn create(center: Vec2, half_size: Vec2) -> Self {
        Self { center, half_size }
    }

    fn contains_position(&self, horizontal_position: Vec2) -> bool {
        let position_offset = (horizontal_position - self.center).abs();
        position_offset.x < self.half_size.x && position_offset.y < self.half_size.y
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
            transform,
        ));
    }
}

fn build_mesh_data(radius: f32, openings: &[SandFieldOpening]) -> SandFieldMeshData {
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

    remove_triangles_in_openings(&positions, &mut triangle_indices, openings);
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

fn remove_triangles_in_openings(
    positions: &[[f32; 3]],
    triangle_indices: &mut Vec<u32>,
    openings: &[SandFieldOpening],
) {
    let remaining_triangle_indices = triangle_indices
        .as_chunks::<3>()
        .0
        .iter()
        .filter(|triangle_indices| {
            openings.iter().all(|opening| {
                triangle_indices.iter().all(|triangle_index| {
                    let vertex_index = usize::try_from(*triangle_index).expect("terrain index should fit into usize");
                    let vertex = Vec3::from_array(positions[vertex_index]);
                    !opening.contains_position(vertex.xz())
                })
            })
        })
        .flatten()
        .copied()
        .collect();
    *triangle_indices = remaining_triangle_indices;
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
