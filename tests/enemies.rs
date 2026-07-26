use bevy::input::ButtonState;
use bevy::input::mouse::MouseButtonInput;
use bevy::prelude::*;
use cathedral::enemies::{Dying, Enemy, move_enemies_and_check_reach, ENEMY_KILL_DISTANCE};
use cathedral::player::{CameraState, Player, PLAYER_START_POSITION};
use cathedral::shooting::shoot;
use cathedral::state::GameState;
use std::time::Duration;

fn create_enemy_test_app() -> App {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins);
    app.add_plugins(bevy::state::app::StatesPlugin);
    app.add_plugins(bevy::input::InputPlugin);
    app.init_state::<GameState>();
    app.insert_resource(bevy::time::TimeUpdateStrategy::ManualDuration(
        Duration::from_millis(16),
    ));
    app.insert_resource(Assets::<Mesh>::default());
    app.insert_resource(Assets::<StandardMaterial>::default());
    app.add_systems(
        Update,
        move_enemies_and_check_reach.run_if(in_state(GameState::Playing)),
    );
    app
}

fn create_shooting_test_app() -> App {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins);
    app.add_plugins(bevy::state::app::StatesPlugin);
    app.add_plugins(bevy::input::InputPlugin);
    app.init_state::<GameState>();
    app.insert_resource(bevy::time::TimeUpdateStrategy::ManualDuration(
        Duration::from_millis(16),
    ));
    app.insert_resource(Assets::<Mesh>::default());
    app.insert_resource(Assets::<StandardMaterial>::default());
    app.add_systems(
        Update,
        shoot.run_if(in_state(GameState::Playing)),
    );
    app
}

fn send_mouse_click(app: &mut App) {
    app.world_mut()
        .resource_mut::<Messages<MouseButtonInput>>()
        .write(MouseButtonInput {
            button: MouseButton::Left,
            state: ButtonState::Pressed,
            window: Entity::PLACEHOLDER,
        });
}

fn current_state(app: &App) -> GameState {
    app.world().resource::<State<GameState>>().get().clone()
}

#[test]
fn enemy_reaching_player_triggers_game_over() {
    let mut app = create_enemy_test_app();
    app.update();

    app.world_mut().spawn((
        Transform::from_translation(PLAYER_START_POSITION),
        Player,
    ));

    let enemy_position = PLAYER_START_POSITION + Vec3::new(0.0, 0.0, ENEMY_KILL_DISTANCE * 0.5);

    app.world_mut().spawn((
        Enemy,
        Transform::from_translation(enemy_position),
    ));

    app.update();
    app.update();

    assert_eq!(current_state(&app), GameState::GameOver);
}

#[test]
fn shooting_enemy_marks_it_dying() {
    let mut app = create_shooting_test_app();
    app.update();

    let _player_entity = app
        .world_mut()
        .spawn((
            Camera3d::default(),
            Transform::from_xyz(0.0, 1.5, 5.0),
            CameraState::default(),
            Player,
        ))
        .id();

    let enemy_entity = app
        .world_mut()
        .spawn((
            Enemy,
            Transform::from_xyz(0.0, 1.5, 3.0),
        ))
        .id();

    send_mouse_click(&mut app);
    app.update();

    assert!(
        app.world().entity(enemy_entity).get::<Dying>().is_some(),
        "enemy should have Dying component after shooting"
    );
}
