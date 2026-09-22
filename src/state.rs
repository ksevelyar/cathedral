use avian3d::prelude::*;
use bevy::ecs::system::SystemParam;
use bevy::prelude::*;

use crate::enemies::Enemy;
use crate::maps::{self, CurrentMap, PlayerStartPosition};
use crate::player::{self, CameraState, Player};

pub struct GameStatePlugin;

impl Plugin for GameStatePlugin {
    fn build(&self, app: &mut App) {
        app.init_state::<GameState>().add_systems(
            Update,
            (
                toggle_pause,
                toggle_physics_pause,
                restart_game.run_if(in_state(GameState::GameOver)),
            ),
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

fn toggle_physics_pause(
    keyboard: Res<ButtonInput<KeyCode>>,
    mut time: ResMut<Time<Physics>>,
    mut gizmo_configs: ResMut<GizmoConfigStore>,
) {
    if keyboard.just_pressed(KeyCode::KeyP) {
        let pause = !time.is_paused();
        if pause {
            time.pause();
        } else {
            time.unpause();
        }
        gizmo_configs.config_mut::<PhysicsGizmos>().0.enabled = pause;
    }
}

#[derive(SystemParam)]
struct GameRestartContext<'w, 's> {
    commands: Commands<'w, 's>,
    enemy_query: Query<'w, 's, Entity, With<Enemy>>,
    current_map: ResMut<'w, CurrentMap>,
    player_start: ResMut<'w, PlayerStartPosition>,
    asset_server: Res<'w, AssetServer>,
}

fn restart_game(
    keyboard: Res<ButtonInput<KeyCode>>,
    mut next_state: ResMut<NextState<GameState>>,
    player_query: Query<(&mut Transform, &mut CameraState), With<Player>>,
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
        &mut restart_context.player_start,
        &mut restart_context.commands,
        &restart_context.asset_server,
    );
    player::reset_player(player_query, &restart_context.player_start);
    next_state.set(GameState::Playing);
}
