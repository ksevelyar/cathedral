use std::time::Instant;

use bevy::ecs::system::SystemParam;
use bevy::prelude::*;
use bevy::world_serialization::{WorldAssetRoot, WorldInstanceReady};

use crate::enemies::Enemy;
use crate::maps::{self, CurrentMap, PlayerPosition};
use crate::player;
use crate::player::Player;
use crate::ragdoll::CharacterPhysicsEntities;
use crate::shooting;

pub struct GameStatePlugin;

impl Plugin for GameStatePlugin {
    fn build(&self, app: &mut App) {
        app.init_state::<GameState>()
            .init_resource::<ColliderInspection>()
            .init_resource::<LoadingStarted>()
            .add_observer(mark_world_asset_ready)
            .add_systems(
                Update,
                (
                    finish_asset_loading.run_if(in_state(GameState::LoadingAssets)),
                    finish_level_loading.run_if(in_state(GameState::LoadingLevel)),
                    toggle_pause,
                    restart_game.run_if(in_state(GameState::GameOver)),
                ),
            );
    }
}

#[derive(States, Default, Clone, PartialEq, Eq, Hash, Debug)]
pub enum GameState {
    #[default]
    LoadingAssets,
    LoadingLevel,
    Playing,
    Paused,
    GameOver,
}

#[derive(Resource, Default)]
pub struct ColliderInspection(pub bool);

#[derive(Resource)]
struct LoadingStarted(Instant);

impl Default for LoadingStarted {
    fn default() -> Self {
        Self(Instant::now())
    }
}

#[derive(Component)]
struct WorldAssetReady;

fn mark_world_asset_ready(world_instance_ready: On<WorldInstanceReady>, mut commands: Commands) {
    commands.entity(world_instance_ready.entity).insert(WorldAssetReady);
}

fn finish_asset_loading(
    asset_server: Res<AssetServer>,
    world_assets: Query<&WorldAssetRoot>,
    mut next_state: ResMut<NextState<GameState>>,
    loading_started: Res<LoadingStarted>,
) {
    if world_assets
        .iter()
        .all(|world_asset| asset_server.is_loaded_with_dependencies(world_asset.0.id()))
    {
        let elapsed_seconds = loading_started.0.elapsed().as_secs_f32();
        info!("assets loaded in {elapsed_seconds:.2} seconds");
        next_state.set(GameState::LoadingLevel);
    }
}

fn finish_level_loading(
    world_assets: Query<(), (With<WorldAssetRoot>, Without<WorldAssetReady>)>,
    mut next_state: ResMut<NextState<GameState>>,
    loading_started: Res<LoadingStarted>,
) {
    if world_assets.is_empty() {
        let elapsed_seconds = loading_started.0.elapsed().as_secs_f32();
        info!("level is rendered in {elapsed_seconds:.2} seconds");
        next_state.set(GameState::Playing);
    }
}

pub fn toggle_pause(
    keyboard: Res<ButtonInput<KeyCode>>,
    state: Res<State<GameState>>,
    mut next_state: ResMut<NextState<GameState>>,
) {
    if keyboard.just_pressed(KeyCode::Escape) {
        next_state.set(match state.get() {
            GameState::Playing => GameState::Paused,
            GameState::Paused => GameState::Playing,
            GameState::LoadingAssets | GameState::LoadingLevel | GameState::GameOver => state.get().clone(),
        });
    }
}

#[derive(SystemParam)]
struct GameRestartContext<'w, 's> {
    commands: Commands<'w, 's>,
    enemy_query: Query<'w, 's, Entity, With<Enemy>>,
    player_bodies: Query<'w, 's, &'static CharacterPhysicsEntities, With<Player>>,
    dead_players: Query<'w, 's, Entity, With<Player>>,
    current_map: ResMut<'w, CurrentMap>,
    player_position: ResMut<'w, PlayerPosition>,
    asset_server: Res<'w, AssetServer>,
}

fn restart_game(
    keyboard: Res<ButtonInput<KeyCode>>,
    mut next_state: ResMut<NextState<GameState>>,
    mut restart_context: GameRestartContext,
) {
    if !keyboard.just_pressed(KeyCode::Space) {
        return;
    }

    for entity in restart_context.enemy_query.iter() {
        restart_context.commands.entity(entity).despawn();
    }
    for body_entities in restart_context.player_bodies.iter() {
        for body_entity in body_entities.list_bodies().iter() {
            restart_context.commands.entity(*body_entity).despawn();
        }
    }
    for player_entity in restart_context.dead_players.iter() {
        restart_context.commands.entity(player_entity).despawn();
    }

    maps::restart(
        &mut restart_context.current_map,
        &mut restart_context.player_position,
        &mut restart_context.commands,
        &restart_context.asset_server,
    );
    let (_, player_camera_entity) = player::spawn_player(
        &mut restart_context.commands,
        &restart_context.asset_server,
        *restart_context.player_position,
    );
    shooting::spawn_player_guns(
        &mut restart_context.commands,
        &restart_context.asset_server,
        player_camera_entity,
    );
    next_state.set(GameState::Playing);
}
