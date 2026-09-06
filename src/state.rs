use avian3d::prelude::*;
use bevy::prelude::*;

use crate::enemies::Enemy;
use crate::map;
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

fn restart_game(
    keyboard: Res<ButtonInput<KeyCode>>,
    mut next_state: ResMut<NextState<GameState>>,
    mut commands: Commands,
    enemy_query: Query<Entity, With<Enemy>>,
    player_query: Query<(&mut Transform, &mut CameraState), With<Player>>,
    asset_server: Res<AssetServer>,
) {
    if !keyboard.just_pressed(KeyCode::Space) {
        return;
    }

    for entity in enemy_query.iter() {
        commands.entity(entity).despawn();
    }

    player::reset_player(player_query);
    map::spawn_map_enemies(&mut commands, &asset_server);
    next_state.set(GameState::Playing);
}
