use bevy::input::mouse::AccumulatedMouseMotion;
use bevy::prelude::*;
use cathedral::player::{CameraState, Player, mouse_look, move_player, setup_player};
use cathedral::state::GameState;
use std::time::Duration;

fn create_test_app() -> App {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins);
    app.add_plugins(bevy::state::app::StatesPlugin);
    app.init_state::<GameState>();
    app.insert_resource(ButtonInput::<KeyCode>::default());
    app.insert_resource(AccumulatedMouseMotion::default());
    app.insert_resource(bevy::time::TimeUpdateStrategy::ManualDuration(Duration::from_millis(
        16,
    )));
    app.insert_resource(Assets::<Mesh>::default());
    app.insert_resource(Assets::<StandardMaterial>::default());
    app.add_systems(Startup, setup_player);
    app.add_systems(Update, (mouse_look, move_player).run_if(in_state(GameState::Playing)));
    app
}

#[test]
fn setup_spawns_player_camera() {
    let mut app = create_test_app();
    app.update();
    app.update();

    let mut query = app
        .world_mut()
        .query_filtered::<Entity, (With<Camera3d>, With<Player>)>();
    assert_eq!(
        query.iter(app.world()).count(),
        1,
        "should spawn exactly one player camera"
    );
}

#[test]
fn mouse_motion_rotates_yaw() {
    let mut app = create_test_app();
    app.update();

    let player = app
        .world_mut()
        .query_filtered::<Entity, With<Player>>()
        .single(app.world())
        .unwrap();

    app.world_mut().resource_mut::<AccumulatedMouseMotion>().delta = Vec2::new(100.0, 0.0);

    app.update();

    let state = app.world().get::<CameraState>(player).unwrap();
    assert!(state.yaw < 0.0, "moving mouse right should decrease yaw");
}

#[test]
fn mouse_motion_rotates_pitch() {
    let mut app = create_test_app();
    app.update();

    let player = app
        .world_mut()
        .query_filtered::<Entity, With<Player>>()
        .single(app.world())
        .unwrap();

    app.world_mut().resource_mut::<AccumulatedMouseMotion>().delta = Vec2::new(0.0, 50.0);

    app.update();

    let state = app.world().get::<CameraState>(player).unwrap();
    assert!(state.pitch < 0.0, "moving mouse up should decrease pitch");
}

#[test]
fn pitch_is_clamped() {
    let mut app = create_test_app();
    app.update();

    let player = app
        .world_mut()
        .query_filtered::<Entity, With<Player>>()
        .single(app.world())
        .unwrap();

    for _ in 0..100 {
        app.world_mut().resource_mut::<AccumulatedMouseMotion>().delta = Vec2::new(0.0, 1000.0);
        app.update();
    }

    let state = app.world().get::<CameraState>(player).unwrap();
    assert!(
        state.pitch >= -std::f32::consts::FRAC_PI_2,
        "pitch should not go below -PI/2"
    );
    assert!(
        state.pitch <= std::f32::consts::FRAC_PI_2,
        "pitch should not go above PI/2"
    );
}

#[test]
fn w_moves_forward() {
    let mut app = create_test_app();
    app.update();

    let player = app
        .world_mut()
        .query_filtered::<Entity, With<Player>>()
        .single(app.world())
        .unwrap();

    let initial_z = app.world().get::<Transform>(player).unwrap().translation.z;

    app.world_mut()
        .resource_mut::<ButtonInput<KeyCode>>()
        .press(KeyCode::KeyW);
    app.update();

    let z = app.world().get::<Transform>(player).unwrap().translation.z;
    assert!(z < initial_z, "W should move forward (-Z in default orientation)");
}

#[test]
fn movement_is_on_xz_plane() {
    let mut app = create_test_app();
    app.update();

    let player = app
        .world_mut()
        .query_filtered::<Entity, With<Player>>()
        .single(app.world())
        .unwrap();

    let initial_y = app.world().get::<Transform>(player).unwrap().translation.y;

    app.world_mut()
        .resource_mut::<ButtonInput<KeyCode>>()
        .press(KeyCode::KeyW);
    app.world_mut()
        .resource_mut::<ButtonInput<KeyCode>>()
        .press(KeyCode::KeyD);
    app.update();

    let y = app.world().get::<Transform>(player).unwrap().translation.y;
    assert!(
        (y - initial_y).abs() < 0.001,
        "movement should stay on XZ plane, Y should not change"
    );
}
