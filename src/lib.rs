pub mod enemies;
pub mod map;
pub mod player;
pub mod ragdoll;
pub mod shooting;
pub mod state;
pub mod ui;

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

use avian3d::prelude::*;
use bevy::prelude::*;

pub struct GamePlugin;

impl Plugin for GamePlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins((
            PhysicsPlugins::default(),
            PhysicsDebugPlugin,
            state::GameStatePlugin,
            map::MapPlugin,
            player::PlayerPlugin,
            enemies::EnemiesPlugin,
            ragdoll::RagdollPlugin,
            shooting::ShootingPlugin,
            ui::UiPlugin,
        ))
        .add_systems(Startup, player::setup_player)
        .insert_gizmo_config(
            PhysicsGizmos::default(),
            GizmoConfig {
                enabled: false,
                ..default()
            },
        )
        .add_systems(Update, toggle_physics_pause);
    }
}
