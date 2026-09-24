use bevy::input::ButtonState;
use bevy::input::keyboard::{Key, KeyboardInput, NativeKey};
use bevy::prelude::*;
use cathedral::app::build_headless_app;
use cathedral::maps::CurrentMap;
use cathedral::state::GameState;
use cathedral::ui::setup_pause_menu;

fn create_test_app() -> App {
    let mut app = build_headless_app(CurrentMap::FlatFloor, None);
    app.add_systems(OnEnter(GameState::Paused), setup_pause_menu);
    app
}

fn get_current_state(app: &App) -> GameState {
    app.world().resource::<State<GameState>>().get().clone()
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

fn press_escape(app: &mut App) {
    send_key(app, KeyCode::Escape, ButtonState::Pressed);
    app.update();
    send_key(app, KeyCode::Escape, ButtonState::Released);
    app.update();
}

#[test]
fn the_game_starts_in_playing_state() {
    let app = create_test_app();
    assert_eq!(get_current_state(&app), GameState::Playing);
}

#[test]
fn pressing_escape_pauses_the_game() {
    let mut app = create_test_app();
    app.update();

    press_escape(&mut app);

    assert_eq!(get_current_state(&app), GameState::Paused);
}

#[test]
fn pressing_escape_twice_resumes_the_game() {
    let mut app = create_test_app();
    app.update();

    press_escape(&mut app);
    assert_eq!(get_current_state(&app), GameState::Paused);

    press_escape(&mut app);
    assert_eq!(get_current_state(&app), GameState::Playing);
}

#[test]
fn pausing_spawns_the_pause_menu() {
    let mut app = create_test_app();
    app.update();

    press_escape(&mut app);

    let menus = app
        .world_mut()
        .query_filtered::<Entity, With<DespawnOnExit<GameState>>>()
        .iter(app.world())
        .count();
    assert!(menus >= 1, "pause menu node should exist");
}

#[test]
fn resuming_despawns_the_pause_menu() {
    let mut app = create_test_app();
    app.update();

    press_escape(&mut app);
    assert_eq!(get_current_state(&app), GameState::Paused);

    press_escape(&mut app);
    assert_eq!(get_current_state(&app), GameState::Playing);

    let menus = app
        .world_mut()
        .query_filtered::<Entity, With<DespawnOnExit<GameState>>>()
        .iter(app.world())
        .count();
    assert_eq!(menus, 0, "pause menu should be despawned");
}
