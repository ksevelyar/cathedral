use bevy::input::ButtonState;
use bevy::input::keyboard::{Key, KeyboardInput, NativeKey};
use bevy::prelude::*;
use terminal::app::build_test_app;
use terminal::maps::{ActiveViewpointIndex, MapBuilder, PlayerPosition};
use terminal::player::{CameraState, Player, PlayerCamera, compute_look_angles};
use terminal::state::GameState;

fn create_test_app(map_name: &'static str) -> App {
    let viewpoints = vec![
        PlayerPosition {
            label: "First viewpoint",
            position: Vec3::new(1.0, 3.0, 2.0),
            look_at: Vec3::new(1.0, 3.0, 1.0),
        },
        PlayerPosition {
            label: "Second viewpoint",
            position: Vec3::new(5.0, 3.0, 2.0),
            look_at: Vec3::new(6.0, 4.0, 2.0),
        },
        PlayerPosition {
            label: "Third viewpoint",
            position: Vec3::new(-3.0, 3.0, 4.0),
            look_at: Vec3::new(-3.0, 4.0, 3.0),
        },
    ];
    let floor_size = Vec3::new(80.0, 0.1, 80.0);
    let map = MapBuilder::new(map_name)
        .with_viewpoints(viewpoints)
        .with_floor(floor_size)
        .build();
    build_test_app(map)
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

fn press_key(app: &mut App, key_code: KeyCode) {
    send_key(app, key_code, ButtonState::Pressed);
    app.update();
    send_key(app, key_code, ButtonState::Released);
    app.update();
}

fn assert_player_at_viewpoint(app: &mut App, viewpoint: PlayerPosition) {
    let (player_entity, player_position) = app
        .world_mut()
        .query_filtered::<(Entity, &Transform), With<Player>>()
        .single(app.world())
        .expect("one player should exist");
    assert!(
        player_position.translation.distance(viewpoint.position) < 0.1,
        "player should reset to the selected viewpoint position"
    );
    let (expected_yaw, expected_pitch) = compute_look_angles(viewpoint.position, viewpoint.look_at);
    let player_camera_state = app.world().get::<CameraState>(player_entity).unwrap();
    assert!((player_camera_state.yaw - expected_yaw).abs() < 0.001);
    assert!((player_camera_state.pitch - expected_pitch).abs() < 0.001);

    let camera_transform = app
        .world_mut()
        .query_filtered::<&Transform, With<PlayerCamera>>()
        .single(app.world())
        .expect("one player camera should exist");
    let expected_camera_rotation = Quat::from_euler(EulerRot::YXZ, 0.0, expected_pitch, 0.0);
    assert!(camera_transform.rotation.angle_between(expected_camera_rotation) < 0.001);
}

#[test]
fn comma_selects_the_previous_viewpoint_and_period_selects_the_next() {
    let mut app = create_test_app("viewpoints");

    press_key(&mut app, KeyCode::Period);
    assert_eq!(app.world().resource::<ActiveViewpointIndex>().0, 1);

    press_key(&mut app, KeyCode::Comma);
    assert_eq!(app.world().resource::<ActiveViewpointIndex>().0, 0);
}

#[test]
fn viewpoint_selection_wraps_in_both_directions() {
    let mut app = create_test_app("viewpoints");

    press_key(&mut app, KeyCode::Comma);
    assert_eq!(app.world().resource::<ActiveViewpointIndex>().0, 2);

    press_key(&mut app, KeyCode::Period);
    assert_eq!(app.world().resource::<ActiveViewpointIndex>().0, 0);
}

#[test]
fn switching_viewpoints_resets_player_position_and_look_direction() {
    let mut app = create_test_app("viewpoints");

    press_key(&mut app, KeyCode::Period);

    assert_player_at_viewpoint(
        &mut app,
        PlayerPosition {
            label: "Second viewpoint",
            position: Vec3::new(5.0, 3.0, 2.0),
            look_at: Vec3::new(6.0, 4.0, 2.0),
        },
    );
}

#[test]
fn restarting_selects_the_first_viewpoint() {
    let mut app = create_test_app("map01");

    press_key(&mut app, KeyCode::Period);
    assert_eq!(app.world().resource::<ActiveViewpointIndex>().0, 1);

    app.world_mut()
        .resource_mut::<NextState<GameState>>()
        .set(GameState::GameOver);
    app.update();
    press_key(&mut app, KeyCode::Space);

    assert_eq!(app.world().resource::<ActiveViewpointIndex>().0, 0);
    assert_player_at_viewpoint(
        &mut app,
        PlayerPosition {
            label: "control-room-entrance",
            position: Vec3::new(10.0, 1.9, 7.0),
            look_at: Vec3::new(-6.0, 2.0, -2.0),
        },
    );
}
