use bevy::ecs::system::SystemParam;
use bevy::prelude::*;

use crate::enemies::Enemy;
use crate::maps::{self, CurrentMap, PlayerPosition};
use crate::player;

pub struct GameStatePlugin;

impl Plugin for GameStatePlugin {
    fn build(&self, app: &mut App) {
        app.init_state::<GameState>().add_systems(
            Update,
            (toggle_pause, restart_game.run_if(in_state(GameState::GameOver))),
        );
    }
}

#[derive(States, Default, Clone, PartialEq, Eq, Hash, Debug)]
pub enum GameState {
    #[default]
    Playing,
    Paused,
    GameOver,
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
            GameState::GameOver => GameState::GameOver,
        });
    }
}

#[derive(SystemParam)]
struct GameRestartContext<'w, 's> {
    commands: Commands<'w, 's>,
    enemy_query: Query<'w, 's, Entity, With<Enemy>>,
    current_map: ResMut<'w, CurrentMap>,
    player_position: ResMut<'w, PlayerPosition>,
    asset_server: Res<'w, AssetServer>,
}

fn restart_game(
    keyboard: Res<ButtonInput<KeyCode>>,
    mut next_state: ResMut<NextState<GameState>>,
    player_query: player::PlayerBodyTransformQuery,
    camera_query: player::PlayerCameraTransformQuery,
    mut restart_context: GameRestartContext,
) {
    if !keyboard.just_pressed(KeyCode::Space) {
        return;
    }

    for entity in restart_context.enemy_query.iter() {
        restart_context.commands.entity(entity).despawn();
    }

    maps::restart(
        &mut restart_context.current_map,
        &mut restart_context.player_position,
        &mut restart_context.commands,
        &restart_context.asset_server,
    );
    player::reset_player(player_query, camera_query, &restart_context.player_position);
    next_state.set(GameState::Playing);
}
