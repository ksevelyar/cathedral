pub mod enemies;
pub mod map;
pub mod player;
pub mod shooting;
pub mod state;
pub mod ui;

use bevy::prelude::*;

pub struct GamePlugin;

impl Plugin for GamePlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins((
            state::GameStatePlugin,
            map::MapPlugin,
            player::PlayerPlugin,
            enemies::EnemiesPlugin,
            shooting::ShootingPlugin,
            ui::UiPlugin,
        ))
        .add_systems(
            Startup,
            (
                player::setup_player,
                map::setup_map,
            )
                .chain(),
        );
    }
}
