mod map01;
mod meshes;
pub mod pieces;
pub mod test_maps;

use bevy::light::FogVolume;
use bevy::prelude::*;
use bevy::transform::TransformSystems;

use crate::enemies::{EnemyKind, spawn_enemy};
use crate::player;
use pieces::Piece;
use pieces::PieceSpawnContext;

pub struct MapsPlugin;

impl Plugin for MapsPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<CurrentMap>()
            .add_systems(Startup, load_current_map.before(player::setup_player))
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
    FlatFloor,
    TallPillar,
    Staircase,
    ColliderInspection,
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

struct Map {
    ambient: GlobalAmbientLight,
    clear_color: ClearColor,
    fog_volume: Option<FogVolumeSettings>,
    player_position: PlayerPosition,
    enemies: Vec<EnemySpawn>,
    pieces: Vec<Piece>,
}

struct FogVolumeSettings {
    center: Vec3,
    size: Vec3,
    color: Color,
    density: f32,
    absorption: f32,
    scattering: f32,
    scattering_asymmetry: f32,
}

struct EnemySpawn {
    position: Vec3,
    kind: EnemyKind,
}

fn build_map(map: &CurrentMap) -> Map {
    match map {
        CurrentMap::Map01 => map01::build_map(),
        CurrentMap::FlatFloor | CurrentMap::TallPillar | CurrentMap::Staircase | CurrentMap::ColliderInspection => {
            test_maps::build_map(map)
        }
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
    if let Some(fog_volume) = map.fog_volume {
        commands.spawn((
            FogVolume {
                fog_color: fog_volume.color,
                density_factor: fog_volume.density,
                absorption: fog_volume.absorption,
                scattering: fog_volume.scattering,
                scattering_asymmetry: fog_volume.scattering_asymmetry,
                ..default()
            },
            Transform::from_translation(fog_volume.center).with_scale(fog_volume.size),
        ));
    }
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
