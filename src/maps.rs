mod map01;
mod map02;
mod pieces;

use avian3d::prelude::*;
use bevy::prelude::*;

use crate::enemies::{Dying, Enemy, EnemyKind, spawn_enemy};
use crate::player;
use pieces::{CuboidSpec, Item, LightSpec, Piece};

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
    pieces: Vec<Piece>,
}

struct EnemySpawn {
    position: Vec3,
    kind: EnemyKind,
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
    spawn_pieces(commands, meshes, materials, &map.pieces);
    spawn_enemies(commands, asset_server, &map.enemies);
}

fn spawn_pieces(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
    map_pieces: &[Piece],
) {
    for map_piece in map_pieces {
        for item in &map_piece.items {
            let transform = map_piece.transform * Transform::from_translation(item_position(item));
            spawn_item(commands, meshes, materials, item, transform);
        }
    }
}

fn item_position(item: &Item) -> Vec3 {
    match item {
        Item::Cuboid(spec) => spec.position,
        Item::Light(spec) => spec.position,
    }
}

fn spawn_item(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
    item: &Item,
    transform: Transform,
) {
    match item {
        Item::Cuboid(CuboidSpec { size, color, .. }) => commands.spawn((
            Arena,
            Mesh3d(meshes.add(Cuboid::new(size.x, size.y, size.z))),
            MeshMaterial3d(materials.add(StandardMaterial {
                base_color: *color,
                ..default()
            })),
            RigidBody::Static,
            Collider::cuboid(size.x, size.y, size.z),
            transform,
        )),
        Item::Light(LightSpec { color, intensity, .. }) => commands.spawn((
            Arena,
            PointLight {
                intensity: *intensity,
                color: *color,
                shadow_maps_enabled: true,
                ..default()
            },
            transform,
        )),
    };
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
