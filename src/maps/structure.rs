use avian3d::prelude::*;
use bevy::prelude::*;

use super::meshes::build_cuboid_mesh;
use super::{MapObject, SpawnContext};

pub struct Floor {
    pub size: Vec3,
}

pub struct Ceiling {
    pub size: Vec3,
}

pub struct Wall {
    pub size: Vec3,
}

impl MapObject for Floor {
    fn spawn(&self, context: &mut SpawnContext, transform: Transform) {
        let floor_tile_size = 2.0;
        spawn_concrete_cuboid(context, transform, self.size, floor_tile_size);
    }
}

impl MapObject for Ceiling {
    fn spawn(&self, context: &mut SpawnContext, transform: Transform) {
        let ceiling_tile_size = 3.0;
        spawn_concrete_cuboid(context, transform, self.size, ceiling_tile_size);
    }
}

impl MapObject for Wall {
    fn spawn(&self, context: &mut SpawnContext, transform: Transform) {
        let wall_tile_size = 3.0;
        spawn_concrete_cuboid(context, transform, self.size, wall_tile_size);
    }
}

fn spawn_concrete_cuboid(context: &mut SpawnContext, transform: Transform, size: Vec3, tile_size: f32) {
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
