mod map01;
mod map02;
mod pieces;
pub mod test_maps;

use avian3d::prelude::*;
use bevy::prelude::*;

use crate::enemies::{Dying, Enemy, EnemyKind, spawn_enemy};
use crate::player;
use pieces::{CuboidSpec, CylinderSpec, Item, LightSpec, MaterialSpec, Piece};

mod meshes;

pub struct MapsPlugin;

impl Plugin for MapsPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<CurrentMap>()
            .add_systems(Startup, load_current_map.before(player::setup_player))
            .add_systems(Update, advance_map);
    }
}

#[derive(Resource, Default, Clone, Copy, PartialEq, Debug)]
pub enum CurrentMap {
    #[default]
    Map01,
    Map02,
    FlatFloor,
    TallPillar,
    Staircase,
}

#[derive(Resource)]
pub(crate) struct PlayerStartOverride(pub Vec3);

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
        CurrentMap::FlatFloor | CurrentMap::TallPillar | CurrentMap::Staircase => test_maps::definition(map),
    }
}

fn load_current_map(
    current: Res<CurrentMap>,
    player_start_override: Option<Res<PlayerStartOverride>>,
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let player_start_override = player_start_override.map(|override_position| override_position.0);
    spawn_map(
        &current,
        player_start_override,
        &mut commands,
        &asset_server,
        &mut meshes,
        &mut materials,
    );
}

fn spawn_map(
    current: &CurrentMap,
    player_start_override: Option<Vec3>,
    commands: &mut Commands,
    asset_server: &AssetServer,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
) {
    let mut map = definition(current);
    if let Some(player_start) = player_start_override {
        map.player_start = player_start;
    }
    commands.insert_resource(PlayerStartPosition {
        position: map.player_start,
    });
    spawn_pieces(commands, asset_server, meshes, materials, &map.pieces);
    spawn_enemies(commands, asset_server, &map.enemies);
}

fn spawn_pieces(
    commands: &mut Commands,
    asset_server: &AssetServer,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
    map_pieces: &[Piece],
) {
    let mut material_cache = Vec::new();
    for map_piece in map_pieces {
        for item in &map_piece.items {
            let transform = map_piece.transform * Transform::from_translation(item_position(item));
            spawn_item(
                commands,
                asset_server,
                meshes,
                materials,
                &mut material_cache,
                item,
                transform,
            );
        }
    }
}

fn item_position(item: &Item) -> Vec3 {
    match item {
        Item::Cuboid(spec) => spec.position,
        Item::Cylinder(spec) => spec.position,
        Item::Light(spec) => spec.position,
    }
}

fn spawn_item(
    commands: &mut Commands,
    asset_server: &AssetServer,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
    material_cache: &mut Vec<(MaterialSpec, Handle<StandardMaterial>)>,
    item: &Item,
    transform: Transform,
) {
    match item {
        Item::Cuboid(CuboidSpec { size, material, .. }) => commands.spawn((
            Arena,
            Mesh3d(meshes.add(meshes::cuboid_mesh(*size, material.tile_size))),
            MeshMaterial3d(material_handle(material_cache, materials, asset_server, material)),
            RigidBody::Static,
            Collider::cuboid(size.x, size.y, size.z),
            transform,
        )),
        Item::Cylinder(CylinderSpec {
            radius,
            height,
            material,
            ..
        }) => commands.spawn((
            Arena,
            Mesh3d(meshes.add(meshes::cylinder_mesh(
                *radius,
                *height,
                meshes::CYLINDER_RADIAL_SEGMENTS,
                material.tile_size,
            ))),
            MeshMaterial3d(material_handle(material_cache, materials, asset_server, material)),
            RigidBody::Static,
            Collider::cylinder(*radius, *height),
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

fn material_handle(
    material_cache: &mut Vec<(MaterialSpec, Handle<StandardMaterial>)>,
    materials: &mut Assets<StandardMaterial>,
    asset_server: &AssetServer,
    spec: &MaterialSpec,
) -> Handle<StandardMaterial> {
    if let Some((_, cached_handle)) = material_cache.iter().find(|(cached, _)| cached == spec) {
        return cached_handle.clone();
    }
    let handle = materials.add(material_from_spec(asset_server, spec));
    material_cache.push((spec.clone(), handle.clone()));
    handle
}

fn material_from_spec(asset_server: &AssetServer, spec: &MaterialSpec) -> StandardMaterial {
    StandardMaterial {
        base_color: spec.base_color,
        base_color_texture: spec
            .base_color_texture
            .as_ref()
            .map(|path| asset_server.load(path.clone())),
        normal_map_texture: spec.normal_texture.as_ref().map(|path| asset_server.load(path.clone())),
        metallic_roughness_texture: spec
            .metallic_roughness_texture
            .as_ref()
            .map(|path| asset_server.load(path.clone())),
        perceptual_roughness: spec.roughness_factor,
        metallic: spec.metallic_factor,
        emissive: spec.emissive.unwrap_or(LinearRgba::BLACK),
        ..default()
    }
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
    if *current != CurrentMap::Map01 || !all_dead {
        return;
    }

    for entity in &arenas {
        commands.entity(entity).despawn();
    }
    for (entity, _) in &enemies {
        commands.entity(entity).despawn();
    }
    *current = CurrentMap::Map02;
    spawn_map(
        &current,
        None,
        &mut commands,
        &asset_server,
        &mut meshes,
        &mut materials,
    );
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
