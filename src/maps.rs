mod map01;
mod map02;

use avian3d::prelude::*;
use bevy::prelude::*;

use crate::enemies::{Dying, Enemy, EnemyKind, spawn_enemy};
use crate::player;

pub struct MapsPlugin;

impl Plugin for MapsPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<CurrentMap>()
            .add_systems(Startup, load_current_map.before(player::setup_player))
            .add_systems(Update, advance_map);
    }
}

#[derive(Resource, Default, Clone, Copy, PartialEq)]
pub(crate) enum CurrentMap {
    #[default]
    Map01,
    Map02,
}

#[derive(Resource)]
pub struct PlayerStartPosition {
    pub position: Vec3,
}

#[derive(Component)]
struct Arena;

struct Map {
    player_start: Vec3,
    enemies: Vec<EnemySpawn>,
    geometry: Vec<Geometry>,
}

struct EnemySpawn {
    position: Vec3,
    kind: EnemyKind,
}

struct CuboidGeometry {
    position: Vec3,
    size: Vec3,
    color: Color,
}

struct StairGeometry {
    position: Vec3,
    direction: Vec3,
    width: f32,
    steps: u32,
    step_height: f32,
    step_depth: f32,
    color: Color,
}

enum Geometry {
    Cuboid(CuboidGeometry),
    Stair(StairGeometry),
}

const FLOOR_COLOR: Color = Color::srgb(0.4, 0.4, 0.4);
const WALL_COLOR: Color = Color::srgb(0.6, 0.6, 0.6);
const CUBOID_COLOR: Color = Color::srgb(0.5, 0.5, 0.5);

fn floor(position: Vec3, size: Vec3) -> Geometry {
    Geometry::Cuboid(CuboidGeometry {
        position,
        size,
        color: FLOOR_COLOR,
    })
}

fn wall(position: Vec3, size: Vec3) -> Geometry {
    Geometry::Cuboid(CuboidGeometry {
        position,
        size,
        color: WALL_COLOR,
    })
}

fn cuboid(position: Vec3, size: Vec3) -> Geometry {
    Geometry::Cuboid(CuboidGeometry {
        position,
        size,
        color: CUBOID_COLOR,
    })
}

fn stair(position: Vec3, direction: Vec3, width: f32, steps: u32, step_height: f32, step_depth: f32) -> Geometry {
    Geometry::Stair(StairGeometry {
        position,
        direction,
        width,
        steps,
        step_height,
        step_depth,
        color: CUBOID_COLOR,
    })
}

fn definition(map: &CurrentMap) -> Map {
    match map {
        CurrentMap::Map01 => map01::definition(),
        CurrentMap::Map02 => map02::definition(),
    }
}

fn load_current_map(
    current: Res<CurrentMap>,
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    spawn_map(&current, &mut commands, &asset_server, &mut meshes, &mut materials);
}

fn spawn_map(
    current: &CurrentMap,
    commands: &mut Commands,
    asset_server: &AssetServer,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
) {
    let map = definition(current);
    commands.insert_resource(PlayerStartPosition {
        position: map.player_start,
    });
    spawn_light(commands);
    spawn_geometry(commands, meshes, materials, &map.geometry);
    spawn_enemies(commands, asset_server, &map.enemies);
}

fn spawn_light(commands: &mut Commands) {
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
}

fn spawn_geometry(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
    geometry: &[Geometry],
) {
    for geometry_item in geometry {
        match geometry_item {
            Geometry::Cuboid(cuboid_geometry) => spawn_cuboid(commands, meshes, materials, cuboid_geometry),
            Geometry::Stair(stair_geometry) => {
                for step_cuboid in stair_cuboids(stair_geometry) {
                    spawn_cuboid(commands, meshes, materials, &step_cuboid);
                }
            }
        }
    }
}

fn stair_cuboids(stair_geometry: &StairGeometry) -> Vec<CuboidGeometry> {
    let direction = stair_geometry.direction.normalize();
    (0..stair_geometry.steps)
        .map(|step_index| {
            let along = step_index as f32 + 0.5;
            CuboidGeometry {
                position: stair_geometry.position
                    + direction * (stair_geometry.step_depth * along)
                    + Vec3::Y * (stair_geometry.step_height * along),
                size: Vec3::new(
                    stair_geometry.width,
                    stair_geometry.step_height,
                    stair_geometry.step_depth,
                ),
                color: stair_geometry.color,
            }
        })
        .collect()
}

fn spawn_cuboid(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
    cuboid_geometry: &CuboidGeometry,
) {
    commands.spawn((
        Arena,
        Mesh3d(meshes.add(Cuboid::new(
            cuboid_geometry.size.x,
            cuboid_geometry.size.y,
            cuboid_geometry.size.z,
        ))),
        MeshMaterial3d(materials.add(StandardMaterial {
            base_color: cuboid_geometry.color,
            ..default()
        })),
        RigidBody::Static,
        Collider::cuboid(cuboid_geometry.size.x, cuboid_geometry.size.y, cuboid_geometry.size.z),
        Transform::from_translation(cuboid_geometry.position),
    ));
}

fn spawn_enemies(commands: &mut Commands, asset_server: &AssetServer, enemies: &[EnemySpawn]) {
    for enemy_spawn in enemies {
        spawn_enemy(
            commands,
            asset_server,
            enemy_spawn.kind.clone(),
            Transform::from_translation(enemy_spawn.position),
            true,
        );
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
    spawn_map(&current, &mut commands, &asset_server, &mut meshes, &mut materials);
}

pub(crate) fn restart(
    current: &mut CurrentMap,
    player_start: &mut PlayerStartPosition,
    commands: &mut Commands,
    asset_server: &AssetServer,
) {
    *current = CurrentMap::Map01;
    let map = definition(current);
    *player_start = PlayerStartPosition {
        position: map.player_start,
    };
    spawn_enemies(commands, asset_server, &map.enemies);
}
