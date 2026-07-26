use bevy::input::ButtonState;
use bevy::input::keyboard::{Key, KeyboardInput, NativeKey};
use bevy::prelude::*;
use cathedral::player::setup_player;
use cathedral::state::{GameState, toggle_pause};
use cathedral::ui::setup_pause_menu;
use std::time::Duration;

fn create_test_app() -> App {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins);
    app.add_plugins(bevy::state::app::StatesPlugin);
    app.add_plugins(bevy::input::InputPlugin);
    app.init_state::<GameState>();
    app.insert_resource(bevy::time::TimeUpdateStrategy::ManualDuration(Duration::from_millis(
        16,
    )));
    app.insert_resource(Assets::<Mesh>::default());
    app.insert_resource(Assets::<StandardMaterial>::default());
    app.add_systems(Startup, setup_player);
    app.add_systems(Update, toggle_pause);
    app.add_systems(OnEnter(GameState::Paused), setup_pause_menu);
    app
}

fn current_state(app: &App) -> GameState {
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
fn starts_in_playing_state() {
    let app = create_test_app();
    assert_eq!(current_state(&app), GameState::Playing);
}

#[test]
fn esc_toggles_to_paused() {
    let mut app = create_test_app();
    app.update();

    press_escape(&mut app);

    assert_eq!(current_state(&app), GameState::Paused);
}

#[test]
fn esc_toggles_back_to_playing() {
    let mut app = create_test_app();
    app.update();

    press_escape(&mut app);
    assert_eq!(current_state(&app), GameState::Paused);

    press_escape(&mut app);
    assert_eq!(current_state(&app), GameState::Playing);
}

#[test]
fn pause_menu_spawns_on_paused() {
    let mut app = create_test_app();
    app.update();

    press_escape(&mut app);

    let menus = app
        .world_mut()
        .query_filtered::<Entity, With<Node>>()
        .iter(app.world())
        .count();
    assert!(menus >= 1, "pause menu node should exist");
}

#[test]
fn pause_menu_despawns_on_resume() {
    let mut app = create_test_app();
    app.update();

    press_escape(&mut app);
    assert_eq!(current_state(&app), GameState::Paused);

    press_escape(&mut app);
    assert_eq!(current_state(&app), GameState::Playing);

    let menus = app
        .world_mut()
        .query_filtered::<Entity, With<Node>>()
        .iter(app.world())
        .count();
    assert_eq!(menus, 0, "pause menu should be despawned");
}
