mod map01;
mod map02;
mod meshes;
pub mod pieces;
pub mod test_maps;

use bevy::prelude::*;
use bevy::transform::TransformSystems;

use crate::enemies::{Dying, Enemy, EnemyKind, spawn_enemy};
use crate::player;
use pieces::Piece;
use pieces::PieceSpawnContext;

pub struct MapsPlugin;

impl Plugin for MapsPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<CurrentMap>()
            .add_systems(Startup, load_current_map.before(player::setup_player))
            .add_systems(Update, advance_map)
            .add_systems(
                FixedPostUpdate,
                pieces::update_lamp_wires.after(TransformSystems::Propagate),
            );
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

#[derive(Resource, Clone, Copy)]
pub struct PlayerPosition {
    pub position: Vec3,
    pub look_at: Vec3,
}

impl PlayerPosition {
    pub fn new(position: Vec3) -> Self {
        Self {
            position,
            look_at: position + Vec3::NEG_Z,
        }
    }
}

#[derive(Component)]
pub(super) struct Arena;

struct Map {
    ambient: GlobalAmbientLight,
    clear_color: ClearColor,
    player_position: PlayerPosition,
    enemies: Vec<EnemySpawn>,
    pieces: Vec<Piece>,
}

struct EnemySpawn {
    position: Vec3,
    kind: EnemyKind,
}

fn build_map(map: &CurrentMap) -> Map {
    match map {
        CurrentMap::Map01 => map01::build_map(),
        CurrentMap::Map02 => map02::build_map(),
        CurrentMap::FlatFloor | CurrentMap::TallPillar | CurrentMap::Staircase => test_maps::build_map(map),
    }
}

fn load_current_map(
    current: Res<CurrentMap>,
    player_position: Option<Res<PlayerPosition>>,
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let override_position = player_position.map(|player_position| *player_position);
    spawn_map(
        &current,
        override_position,
        &mut commands,
        &asset_server,
        &mut meshes,
        &mut materials,
    );
}

fn spawn_map(
    current: &CurrentMap,
    override_position: Option<PlayerPosition>,
    commands: &mut Commands,
    asset_server: &AssetServer,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
) {
    let map = build_map(current);
    commands.insert_resource(override_position.unwrap_or(map.player_position));
    commands.insert_resource(map.ambient);
    commands.insert_resource(map.clear_color);
    let mut material_cache = Vec::new();
    let mut piece_context = PieceSpawnContext::new(commands, asset_server, meshes, materials, &mut material_cache);
    for map_piece in &map.pieces {
        pieces::spawn_piece(map_piece, &mut piece_context);
    }
    spawn_enemies(commands, asset_server, &map.enemies);
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
    player_position: &mut PlayerPosition,
    commands: &mut Commands,
    asset_server: &AssetServer,
) {
    *current = CurrentMap::Map01;
    let map = build_map(current);
    *player_position = map.player_position;
    spawn_enemies(commands, asset_server, &map.enemies);
}
