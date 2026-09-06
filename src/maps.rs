mod map01;
mod map02;

use avian3d::prelude::*;
use bevy::prelude::*;

use crate::enemies::{Dying, Enemy};

#[derive(Resource, Default, Clone, Copy, PartialEq)]
pub(crate) enum CurrentMap {
    #[default]
    Map01,
    Map02,
}

#[derive(Component)]
struct Arena;

pub struct MapsPlugin;

impl Plugin for MapsPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<CurrentMap>()
            .add_systems(Startup, load_current_map)
            .add_systems(Update, advance_map);
    }
}

fn load_current_map(
    current: Res<CurrentMap>,
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    load(&current, &mut commands, &asset_server, &mut meshes, &mut materials);
}

fn load(
    map: &CurrentMap,
    commands: &mut Commands,
    asset_server: &AssetServer,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
) {
    match map {
        CurrentMap::Map01 => map01::load(commands, asset_server, meshes, materials),
        CurrentMap::Map02 => map02::load(commands, asset_server, meshes, materials),
    }
}

fn advance_map(
    mut current: ResMut<CurrentMap>,
    enemies: Query<(Entity, Has<Dying>), With<Enemy>>,
    arenas: Query<Entity, With<Arena>>,
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let all_dead = !enemies.is_empty() && enemies.iter().all(|(_, dying)| dying);
    if *current == CurrentMap::Map02 || !all_dead {
        return;
    }

    for entity in &arenas {
        commands.entity(entity).despawn();
    }
    for (entity, _) in &enemies {
        commands.entity(entity).despawn();
    }
    *current = CurrentMap::Map02;
    load(&current, &mut commands, &asset_server, &mut meshes, &mut materials);
}

pub(crate) fn restart(current: &mut CurrentMap, commands: &mut Commands, asset_server: &AssetServer) {
    *current = CurrentMap::Map01;
    map01::spawn_enemies(commands, asset_server);
}

fn arena(commands: &mut Commands, meshes: &mut Assets<Mesh>, materials: &mut Assets<StandardMaterial>) {
    commands.spawn((
        Arena,
        PointLight {
            intensity: 2_000_000.0,
            color: Color::srgb(1.0, 0.85, 0.6),
            shadow_maps_enabled: true,
            ..default()
        },
        Transform::from_xyz(4.8, 4.8, 1.0),
    ));

    commands.spawn((
        Arena,
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
            Arena,
            Mesh3d(wall_mesh.clone()),
            MeshMaterial3d(wall_material.clone()),
            wall_transform,
        ));
    }
}
