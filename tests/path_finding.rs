use bevy::ecs::system::RunSystemOnce;
use bevy::input::ButtonState;
use bevy::input::keyboard::{Key, KeyboardInput, NativeKey};
use bevy::prelude::*;
use cathedral::app::build_headless_app;
use cathedral::enemies::{AnimationState, EnemyActivity, EnemyKind, Fighter, spawn_enemy};
use cathedral::maps::test_maps::{TALL_PILLAR_POSITION, TALL_PILLAR_SIZE};
use cathedral::maps::{CurrentMap, PlayerPosition};
use cathedral::player::{CameraState, Player};
use std::time::Duration;

const FIXED_TIMESTEP_SECONDS: f64 = 1.0 / 64.0;
const PATH_FINDING_UPDATES: usize = 2048;
const PLAYER_BLOCKER_UPDATES: usize = 1024;
const FIGHTER_START_POSITION: Vec3 = Vec3::new(-6.0, 0.5, 0.0);
const PLAYER_START_POSITION: Vec3 = Vec3::new(6.0, 0.5, 0.0);
const BLOCKED_PLAYER_START_POSITION: Vec3 = Vec3::new(-6.0, 1.5, 0.0);
const SOUTH_FACE_PLAYER_START_POSITION: Vec3 = Vec3::new(0.0, 1.5, 2.0);
const STAIR_PLAYER_START_POSITION: Vec3 = Vec3::new(0.0, 1.5, 12.0);
const STAIR_PLAYER_WAITING_POSITION: Vec3 = Vec3::new(0.0, 0.5, -8.0);
const STAIR_FIGHTER_START_POSITION: Vec3 = Vec3::new(0.0, 0.5, 2.5);
const FIGHTER_REACH: f32 = 2.5;
const REACH_MARGIN: f32 = 0.5;
const MINIMUM_PROGRESS: f32 = 0.5;
const STAIR_CROSSING_POSITION: f32 = -1.0;

fn create_test_app(map: CurrentMap, player_position: Option<PlayerPosition>) -> App {
    let fixed_timestep = Duration::from_secs_f64(FIXED_TIMESTEP_SECONDS);
    let mut app = build_headless_app(map, player_position);
    app.insert_resource(Time::<Fixed>::from_duration(fixed_timestep));
    app.update();
    app
}

fn spawn_fighter(mut commands: Commands, asset_server: Res<AssetServer>) -> Entity {
    spawn_enemy(
        &mut commands,
        &asset_server,
        EnemyKind::Fighter(Fighter::default()),
        Transform::from_translation(FIGHTER_START_POSITION),
        true,
    )
}

fn spawn_fighter_on_staircase(mut commands: Commands, asset_server: Res<AssetServer>) -> Entity {
    spawn_enemy(
        &mut commands,
        &asset_server,
        EnemyKind::Fighter(Fighter::default()),
        Transform::from_translation(STAIR_FIGHTER_START_POSITION),
        true,
    )
}

fn find_player_entity(app: &mut App) -> Entity {
    app.world_mut()
        .query_filtered::<Entity, With<Player>>()
        .single(app.world_mut())
        .expect("one player should spawn")
}

fn read_player_translation(app: &mut App) -> Vec3 {
    let player = find_player_entity(app);
    app.world().get::<Transform>(player).unwrap().translation
}

fn aim_player_yaw(app: &mut App, yaw: f32) {
    let player = find_player_entity(app);
    app.world_mut().get_mut::<CameraState>(player).unwrap().yaw = yaw;
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

fn press_forward(app: &mut App) {
    send_key(app, KeyCode::KeyW, ButtonState::Pressed);
}

#[test]
fn player_pressing_w_is_blocked_by_pillar() {
    let mut app = create_test_app(
        CurrentMap::TallPillar,
        Some(PlayerPosition::new(BLOCKED_PLAYER_START_POSITION)),
    );
    aim_player_yaw(&mut app, -std::f32::consts::FRAC_PI_2);
    press_forward(&mut app);

    for _ in 0..PLAYER_BLOCKER_UPDATES {
        app.update();
    }

    let translation = read_player_translation(&mut app);
    assert!(
        translation.x - BLOCKED_PLAYER_START_POSITION.x > MINIMUM_PROGRESS,
        "player never moved while pressing W, ended at {translation:?}"
    );
    assert!(
        translation.x < TALL_PILLAR_POSITION.x,
        "player went through the pillar, ended at {translation:?}"
    );
}

#[test]
fn player_pressing_w_is_stopped_by_the_pillar_south_face() {
    let mut app = create_test_app(
        CurrentMap::TallPillar,
        Some(PlayerPosition::new(SOUTH_FACE_PLAYER_START_POSITION)),
    );
    press_forward(&mut app);

    for _ in 0..PLAYER_BLOCKER_UPDATES {
        app.update();
    }

    let translation = read_player_translation(&mut app);
    let pillar_south_face = TALL_PILLAR_POSITION.z - TALL_PILLAR_SIZE.z / 2.0;
    assert!(
        translation.z > pillar_south_face,
        "player went through the pillar south face, ended at {translation:?}"
    );
}

#[test]
fn player_walks_over_the_staircase() {
    let mut app = create_test_app(
        CurrentMap::Staircase,
        Some(PlayerPosition::new(STAIR_PLAYER_START_POSITION)),
    );
    press_forward(&mut app);

    let player = find_player_entity(&mut app);
    let mut highest_camera = f32::MIN;
    for _ in 0..PLAYER_BLOCKER_UPDATES {
        app.update();
        let camera_height = app.world().get::<Transform>(player).unwrap().translation.y;
        highest_camera = highest_camera.max(camera_height);
    }

    let translation = read_player_translation(&mut app);
    assert!(
        translation.z < -5.0,
        "player never crossed the staircase, ended at {translation:?}"
    );
    assert!(
        highest_camera > 3.0,
        "player never climbed onto the apex, highest camera height {highest_camera}"
    );
}

#[test]
fn fighter_walks_over_the_staircase_to_player() {
    let mut app = create_test_app(
        CurrentMap::Staircase,
        Some(PlayerPosition::new(STAIR_PLAYER_WAITING_POSITION)),
    );
    let enemy = app
        .world_mut()
        .run_system_once(spawn_fighter_on_staircase)
        .expect("fighter should spawn");
    app.update();

    let mut activities = app.world_mut().query::<&EnemyActivity>();
    let mut attacking_observed = false;
    for _ in 0..PATH_FINDING_UPDATES {
        app.update();
        if let Ok(activity) = activities.get(app.world(), enemy)
            && activity.state == AnimationState::Attacking
        {
            attacking_observed = true;
        }
    }

    let translation = app.world().get::<Transform>(enemy).unwrap().translation;
    let distance_to_player = (translation - read_player_translation(&mut app)).xz().length();
    assert!(
        translation.z < STAIR_CROSSING_POSITION,
        "fighter never crossed the staircase apex, ended at {translation:?}"
    );
    assert!(
        distance_to_player <= FIGHTER_REACH + REACH_MARGIN,
        "fighter stopped {} away from the player at {translation:?}",
        distance_to_player
    );
    assert!(attacking_observed, "fighter never started attacking the player");
}

#[test]
fn fighter_walks_around_pillar_to_reach_player() {
    let mut app = create_test_app(CurrentMap::TallPillar, Some(PlayerPosition::new(PLAYER_START_POSITION)));
    let enemy = app
        .world_mut()
        .run_system_once(spawn_fighter)
        .expect("fighter should spawn");
    app.update();

    let mut activities = app.world_mut().query::<&EnemyActivity>();
    let mut attacking_observed = false;
    for _ in 0..PATH_FINDING_UPDATES {
        app.update();
        if let Ok(activity) = activities.get(app.world(), enemy)
            && activity.state == AnimationState::Attacking
        {
            attacking_observed = true;
        }
    }

    let translation = app.world().get::<Transform>(enemy).unwrap().translation;
    assert!(
        translation.x - FIGHTER_START_POSITION.x > MINIMUM_PROGRESS,
        "fighter never left its start position, ended at {translation:?}"
    );
    assert!(
        translation.x > TALL_PILLAR_POSITION.x,
        "fighter never got past the pillar, stuck at {translation:?}"
    );
    let distance_to_player = (translation - read_player_translation(&mut app)).xz().length();
    assert!(
        distance_to_player <= FIGHTER_REACH + REACH_MARGIN,
        "fighter stopped {} away from the player at {translation:?}",
        distance_to_player
    );
    assert!(attacking_observed, "fighter never started attacking the player");
}
