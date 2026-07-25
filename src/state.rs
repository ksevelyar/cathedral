use bevy::prelude::*;

use crate::enemies::{self, Enemy};
use crate::player::{CameraState, Player, PLAYER_START_POSITION};

pub struct GameStatePlugin;

impl Plugin for GameStatePlugin {
    fn build(&self, app: &mut App) {
        app.init_state::<GameState>()
            .add_systems(
                Update,
                (
                    toggle_pause,
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

pub fn restart_game(
    keyboard: Res<ButtonInput<KeyCode>>,
    mut next_state: ResMut<NextState<GameState>>,
    mut commands: Commands,
    enemy_query: Query<Entity, With<Enemy>>,
    mut player_query: Query<(&mut Transform, &mut CameraState), With<Player>>,
    meshes: ResMut<Assets<Mesh>>,
    materials: ResMut<Assets<StandardMaterial>>,
) {
    if !keyboard.just_pressed(KeyCode::Space) {
        return;
    }

    for entity in enemy_query.iter() {
        commands.entity(entity).despawn();
    }

    if let Ok((mut transform, mut camera_state)) = player_query.single_mut() {
        transform.translation = PLAYER_START_POSITION;
        camera_state.yaw = 0.0;
        camera_state.pitch = 0.0;
        transform.rotation = Quat::IDENTITY;
    }

    enemies::spawn_enemies(commands, meshes, materials);
    next_state.set(GameState::Playing);
}
