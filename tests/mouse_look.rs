use bevy::input::ButtonState;
use bevy::input::keyboard::{Key, KeyboardInput, NativeKey};
use bevy::input::mouse::{AccumulatedMouseMotion, MouseMotion};
use bevy::prelude::*;
use cathedral::app::build_headless_app;
use cathedral::maps::CurrentMap;
use cathedral::player::{CameraState, Player};

fn create_test_app() -> App {
    build_headless_app(CurrentMap::FlatFloor, None)
}

fn player_entity(app: &mut App) -> Entity {
    app.world_mut()
        .query_filtered::<Entity, With<Player>>()
        .single(app.world_mut())
        .expect("one player should spawn")
}

fn send_mouse_motion(app: &mut App, delta: Vec2) {
    app.world_mut()
        .resource_mut::<Messages<MouseMotion>>()
        .write(MouseMotion { delta });
}

fn send_key(app: &mut App, key_code: KeyCode, state: ButtonState) {
    app.world_mut()
        .resource_mut::<Messages<KeyboardInput>>()
        .write(KeyboardInput {
            key_code,
            logical_key: Key::Unidentified(NativeKey::Unidentified),
            state,
            text: None,
            repeat: false,
            window: Entity::PLACEHOLDER,
        });
}

#[test]
fn starting_the_game_spawns_the_player_camera() {
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

    let player = player_entity(&mut app);

    send_mouse_motion(&mut app, Vec2::new(100.0, 0.0));

    app.update();

    let state = app.world().get::<CameraState>(player).unwrap();
    assert!(state.yaw < 0.0, "moving mouse right should decrease yaw");
}

#[test]
fn mouse_motion_rotates_pitch() {
    let mut app = create_test_app();
    app.update();

    let player = player_entity(&mut app);

    send_mouse_motion(&mut app, Vec2::new(0.0, 50.0));

    app.update();

    let state = app.world().get::<CameraState>(player).unwrap();
    assert!(state.pitch < 0.0, "moving mouse up should decrease pitch");
}

#[test]
fn pitch_is_clamped() {
    let mut app = create_test_app();
    app.update();

    let player = player_entity(&mut app);

    for _ in 0..100 {
        send_mouse_motion(&mut app, Vec2::new(0.0, 1000.0));
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

    let player = player_entity(&mut app);

    let initial_z = app.world().get::<Transform>(player).unwrap().translation.z;

    send_key(&mut app, KeyCode::KeyW, ButtonState::Pressed);
    app.update();

    let z = app.world().get::<Transform>(player).unwrap().translation.z;
    assert!(z < initial_z, "W should move forward (-Z in default orientation)");
}

#[test]
fn movement_is_on_xz_plane() {
    let mut app = create_test_app();
    app.update();

    let player = player_entity(&mut app);

    let initial_y = app.world().get::<Transform>(player).unwrap().translation.y;

    send_key(&mut app, KeyCode::KeyW, ButtonState::Pressed);
    send_key(&mut app, KeyCode::KeyD, ButtonState::Pressed);
    app.update();
    app.update();

    let y = app.world().get::<Transform>(player).unwrap().translation.y;
    assert!(
        (y - initial_y).abs() < 0.001,
        "movement should stay on XZ plane, Y should not change"
    );
}
