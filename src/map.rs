use avian3d::prelude::*;
use bevy::prelude::*;

use crate::enemies::{EnemyKind, Fighter, Gunner, spawn_enemy};

pub struct MapPlugin;

impl Plugin for MapPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, setup_map);
    }
}

fn setup_map(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    commands.spawn((
        PointLight {
            intensity: 2_000_000.0,
            color: Color::srgb(1.0, 0.85, 0.6),
            shadow_maps_enabled: true,
            ..default()
        },
        Transform::from_xyz(4.8, 4.8, 1.0),
    ));

    commands.spawn((
        Mesh3d(meshes.add(Cuboid::new(30.0, 0.1, 30.0))),
        MeshMaterial3d(materials.add(StandardMaterial {
            base_color: Color::srgb(0.4, 0.4, 0.4),
            ..default()
        })),
        RigidBody::Static,
        Collider::cuboid(30.0, 0.1, 30.0),
        Transform::from_xyz(0.0, 0.0, 0.0),
    ));

    let wall_mesh = meshes.add(Cuboid::new(30.0, 3.0, 0.1));
    let wall_material = materials.add(StandardMaterial {
        base_color: Color::srgb(0.6, 0.6, 0.6),
        ..default()
    });

    let wall_transforms = [
        Transform::from_xyz(0.0, 1.5, -15.0),
        Transform::from_xyz(0.0, 1.5, 15.0),
        Transform::from_xyz(-15.0, 1.5, 0.0).with_rotation(Quat::from_rotation_y(90.0_f32.to_radians())),
        Transform::from_xyz(15.0, 1.5, 0.0).with_rotation(Quat::from_rotation_y(90.0_f32.to_radians())),
    ];

    for wall_transform in wall_transforms {
        commands.spawn((
            Mesh3d(wall_mesh.clone()),
            MeshMaterial3d(wall_material.clone()),
            wall_transform,
        ));
    }

    spawn_map_enemies(&mut commands, &asset_server);
}

pub fn spawn_map_enemies(commands: &mut Commands, asset_server: &AssetServer) {
    for (translation, kind) in [
        (Vec3::new(-8.0, 0.0, -8.0), EnemyKind::Gunner(Gunner::default())),
        (Vec3::new(8.0, 0.0, -8.0), EnemyKind::Gunner(Gunner::default())),
        (Vec3::new(0.0, 0.0, -10.0), EnemyKind::Fighter(Fighter::default())),
    ] {
        spawn_enemy(
            commands,
            asset_server,
            kind,
            Transform::from_translation(translation),
            true,
        );
    }
}
