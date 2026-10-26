use avian3d::prelude::{Collider, RigidBody};
use bevy::prelude::*;
use cathedral::app::build_test_app;
use cathedral::maps::{MapLoadingDependency, SandField, SandFieldSurface, build_intro};
use cathedral::player::Player;
use cathedral::state::GameState;

#[test]
fn terrain_samples_remain_finite_and_smooth_at_two_meter_intervals() {
    for grid_x in (-300..=300).step_by(10) {
        for grid_z in (-300..=300).step_by(10) {
            let horizontal_position = Vec2::new(grid_x as f32, grid_z as f32);
            let terrain_height = SandField::calculate_height_at(horizontal_position);
            let eastward_terrain_height = SandField::calculate_height_at(horizontal_position + Vec2::X * 2.0);
            let northward_terrain_height = SandField::calculate_height_at(horizontal_position + Vec2::Y * 2.0);
            assert!(terrain_height.is_finite());
            assert!((eastward_terrain_height - terrain_height).abs() < 1.5);
            assert!((northward_terrain_height - terrain_height).abs() < 1.5);
        }
    }
}

#[test]
fn intro_map_enters_playing_with_sandfield_and_player_above_terrain() {
    let mut app = build_test_app(build_intro());

    assert_eq!(app.world().resource::<State<GameState>>().get(), &GameState::Playing);

    let remaining_map_loading_dependencies = app
        .world_mut()
        .query_filtered::<Entity, With<MapLoadingDependency>>()
        .iter(app.world())
        .count();
    assert_eq!(remaining_map_loading_dependencies, 0);

    let static_sandfield_count = app
        .world_mut()
        .query_filtered::<&RigidBody, (With<SandFieldSurface>, With<Collider>)>()
        .iter(app.world())
        .filter(|rigid_body| **rigid_body == RigidBody::Static)
        .count();
    assert_eq!(static_sandfield_count, 1);

    let player_position = app
        .world_mut()
        .query_filtered::<&Transform, With<Player>>()
        .single(app.world())
        .expect("intro should spawn one player")
        .translation;
    let terrain_height = SandField::calculate_height_at(player_position.xz());
    assert!(
        player_position.y > terrain_height,
        "player should start above the terrain at y={}, terrain y={terrain_height}",
        player_position.y
    );
}
