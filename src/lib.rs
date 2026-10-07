#![recursion_limit = "512"]

use avian3d::prelude::*;
use bevy::prelude::*;

pub mod app;
pub mod enemies;
pub mod maps;
pub(crate) mod movement;
pub mod player;
pub mod ragdoll;
pub mod shooting;
pub mod state;
pub mod ui;

pub struct GamePlugin;

impl Plugin for GamePlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins((
            PhysicsPlugins::default(),
            PhysicsDebugPlugin,
            state::GameStatePlugin,
            maps::MapsPlugin,
            player::PlayerPlugin,
            enemies::EnemiesPlugin,
            ragdoll::RagdollPlugin,
            shooting::ShootingPlugin,
            ui::UiPlugin,
        ))
        .insert_resource(SubstepCount(30))
        .add_systems(Startup, player::setup_player)
        .insert_gizmo_config(
            PhysicsGizmos::default(),
            GizmoConfig {
                enabled: false,
                ..default()
            },
        );
    }
}
