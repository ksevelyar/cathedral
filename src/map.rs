use bevy::prelude::*;

pub struct MapPlugin;

impl Plugin for MapPlugin {
    fn build(&self, _app: &mut App) {}
}

pub fn setup_map(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    commands.spawn((
        PointLight {
            intensity: 1_000_000.0,
            color: Color::srgb(1.0, 0.85, 0.6),
            shadow_maps_enabled: true,
            ..default()
        },
        Transform::from_xyz(0.0, 2.8, 0.0),
    ));

    commands.spawn((
        Mesh3d(meshes.add(Cuboid::new(30.0, 0.1, 30.0))),
        MeshMaterial3d(materials.add(StandardMaterial {
            base_color: Color::srgb(0.4, 0.4, 0.4),
            ..default()
        })),
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
}
