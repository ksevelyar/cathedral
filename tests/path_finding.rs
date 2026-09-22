use avian3d::prelude::*;
use bevy::animation::AnimationPlugin;
use bevy::ecs::system::RunSystemOnce;
use bevy::gltf::GltfPlugin;
use bevy::mesh::MeshPlugin;
use bevy::prelude::*;
use bevy::time::TimeUpdateStrategy;
use bevy::world_serialization::WorldSerializationPlugin;
use cathedral::enemies::{AnimationState, EnemyActivity, EnemyKind, Fighter, spawn_enemy};
use cathedral::player::{CameraState, Player, move_player};
use cathedral::state::{GameState, GameStatePlugin};
use std::time::Duration;

const FIXED_TIMESTEP_SECONDS: f64 = 1.0 / 64.0;
const PATH_FINDING_UPDATES: usize = 2048;
const PLAYER_BLOCKER_UPDATES: usize = 1024;
const FIGHTER_START_POSITION: Vec3 = Vec3::new(-6.0, 0.5, 0.0);
const PILLAR_POSITION: Vec3 = Vec3::new(0.0, 1.5, 0.0);
const PILLAR_SIZE: Vec3 = Vec3::new(1.0, 3.0, 1.0);
const PLAYER_START_POSITION: Vec3 = Vec3::new(6.0, 0.5, 0.0);
const FIGHTER_REACH: f32 = 2.5;
const REACH_MARGIN: f32 = 0.5;
const MINIMUM_PROGRESS: f32 = 0.5;
const PLAYER_PILLAR_START_POSITION: Vec3 = Vec3::new(-6.0, 1.5, 0.0);
const PLAYER_PILLAR_POSITION: Vec3 = Vec3::new(0.0, 10.0, 0.0);
const PLAYER_PILLAR_SIZE: Vec3 = Vec3::new(1.0, 20.0, 1.0);
const MAP02_PILLAR_POSITION: Vec3 = Vec3::new(-6.0, 1.0, -7.0);
const MAP02_PILLAR_SIZE: Vec3 = Vec3::new(4.0, 2.0, 4.0);
const MAP02_PLAYER_START_POSITION: Vec3 = Vec3::new(-6.0, 1.5, -2.0);
const STAIR_PLAYER_START_POSITION: Vec3 = Vec3::new(0.0, 1.5, 12.0);
const STAIRCASE_APEX_POSITION: Vec3 = Vec3::new(0.0, 1.025, 0.0);
const STAIRCASE_APEX_SIZE: Vec3 = Vec3::new(4.0, 2.05, 1.0);
const STAIR_FIGHTER_START_POSITION: Vec3 = Vec3::new(0.0, 0.5, 2.5);
const STAIR_PLAYER_MARKER_POSITION: Vec3 = Vec3::new(0.0, 0.5, -8.0);
const STAIR_CROSSING_POSITION: f32 = -1.0;

fn spawn_floor(mut commands: Commands) {
    commands.spawn((
        RigidBody::Static,
        Collider::cuboid(60.0, 0.1, 60.0),
        Transform::from_xyz(0.0, 0.0, 0.0),
    ));
}

fn create_test_app() -> App {
    let fixed_timestep = Duration::from_secs_f64(FIXED_TIMESTEP_SECONDS);
    let mut app = App::new();
    app.add_plugins((
        MinimalPlugins,
        TransformPlugin,
        AssetPlugin::default(),
        bevy::input::InputPlugin,
        WorldSerializationPlugin,
        MeshPlugin,
        AnimationPlugin,
        GltfPlugin::default(),
        PhysicsPlugins::default(),
        bevy::state::app::StatesPlugin,
        GameStatePlugin,
        cathedral::enemies::EnemiesPlugin,
    ))
    .insert_resource(Time::<Fixed>::from_duration(fixed_timestep))
    .insert_resource(TimeUpdateStrategy::ManualDuration(fixed_timestep))
    .insert_resource(bevy::prelude::GizmoConfigStore::default())
    .add_systems(Startup, spawn_floor)
    .add_systems(Update, move_player.run_if(in_state(GameState::Playing)));
    app.finish();
    app.cleanup();
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

fn spawn_pillar_course(app: &mut App) -> Entity {
    app.world_mut()
        .spawn((Player, Transform::from_translation(PLAYER_START_POSITION)));
    spawn_static_cuboid(app, PILLAR_POSITION, PILLAR_SIZE);
    let enemy = app
        .world_mut()
        .run_system_once(spawn_fighter)
        .expect("fighter should spawn");
    app.update();
    enemy
}

fn spawn_player_pillar_course(app: &mut App) -> Entity {
    let player = app.world_mut().spawn((
        Player,
        Transform::from_translation(PLAYER_PILLAR_START_POSITION),
        CameraState {
            yaw: -std::f32::consts::FRAC_PI_2,
            pitch: 0.0,
        },
        LinearVelocity::default(),
    ));
    let player_entity = player.id();
    spawn_static_cuboid(app, PLAYER_PILLAR_POSITION, PLAYER_PILLAR_SIZE);
    player_entity
}

fn spawn_player_walking_course(
    app: &mut App,
    cuboid_position: Vec3,
    cuboid_size: Vec3,
    player_position: Vec3,
) -> Entity {
    let player = app.world_mut().spawn((
        Player,
        Transform::from_translation(player_position),
        CameraState::default(),
        LinearVelocity::default(),
    ));
    let player_entity = player.id();
    spawn_static_cuboid(app, cuboid_position, cuboid_size);
    player_entity
}

fn spawn_static_cuboid(app: &mut App, position: Vec3, size: Vec3) {
    app.world_mut().spawn((
        RigidBody::Static,
        Collider::cuboid(size.x, size.y, size.z),
        Transform::from_translation(position),
    ));
}

fn spawn_staircase(app: &mut App) {
    for step_index in 0..4 {
        let step_center_height = 0.3 + step_index as f32 * 0.5;
        let up_z = 4.5 - step_index as f32;
        let down_z = -4.5 + step_index as f32;
        for step_z in [up_z, down_z] {
            spawn_static_cuboid(
                app,
                Vec3::new(0.0, step_center_height, step_z),
                Vec3::new(4.0, 0.5, 1.0),
            );
        }
    }
    spawn_static_cuboid(app, STAIRCASE_APEX_POSITION, STAIRCASE_APEX_SIZE);
}

fn press_forward(app: &mut App) {
    app.world_mut()
        .resource_mut::<ButtonInput<KeyCode>>()
        .press(KeyCode::KeyW);
}

#[test]
fn player_pressing_w_is_blocked_by_pillar() {
    let mut app = create_test_app();
    app.insert_resource(ButtonInput::<KeyCode>::default());
    let player = spawn_player_pillar_course(&mut app);
    press_forward(&mut app);

    for _ in 0..PLAYER_BLOCKER_UPDATES {
        app.update();
    }

    let translation = app.world().get::<Transform>(player).unwrap().translation;
    assert!(
        translation.x - PLAYER_PILLAR_START_POSITION.x > MINIMUM_PROGRESS,
        "player never moved while pressing W, ended at {translation:?}"
    );
    assert!(
        translation.x < PLAYER_PILLAR_POSITION.x,
        "player went through the pillar, ended at {translation:?}"
    );
}

#[test]
fn player_pressing_w_is_blocked_by_map02_pillar() {
    let mut app = create_test_app();
    app.insert_resource(ButtonInput::<KeyCode>::default());
    let player = spawn_player_walking_course(
        &mut app,
        MAP02_PILLAR_POSITION,
        MAP02_PILLAR_SIZE,
        MAP02_PLAYER_START_POSITION,
    );
    press_forward(&mut app);

    for _ in 0..PLAYER_BLOCKER_UPDATES {
        app.update();
    }

    let translation = app.world().get::<Transform>(player).unwrap().translation;
    let pillar_south_face = MAP02_PILLAR_POSITION.z - MAP02_PILLAR_SIZE.z / 2.0;
    assert!(
        translation.z > pillar_south_face,
        "player went through the map02 pillar, ended at {translation:?}"
    );
}

#[test]
fn player_walks_over_the_staircase() {
    let mut app = create_test_app();
    app.insert_resource(ButtonInput::<KeyCode>::default());
    let player = spawn_player_walking_course(
        &mut app,
        STAIRCASE_APEX_POSITION,
        STAIRCASE_APEX_SIZE,
        STAIR_PLAYER_START_POSITION,
    );
    spawn_staircase(&mut app);
    press_forward(&mut app);

    let mut highest_camera = f32::MIN;
    for _ in 0..PLAYER_BLOCKER_UPDATES {
        app.update();
        let camera_height = app.world().get::<Transform>(player).unwrap().translation.y;
        highest_camera = highest_camera.max(camera_height);
    }

    let translation = app.world().get::<Transform>(player).unwrap().translation;
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
    let mut app = create_test_app();
    app.world_mut()
        .spawn((Player, Transform::from_translation(STAIR_PLAYER_MARKER_POSITION)));
    spawn_staircase(&mut app);
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
    let distance_to_player = (translation - STAIR_PLAYER_MARKER_POSITION).length();
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
    let mut app = create_test_app();
    let enemy = spawn_pillar_course(&mut app);

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
        translation.x > PILLAR_POSITION.x,
        "fighter never got past the pillar, stuck at {translation:?}"
    );
    let distance_to_player = (translation - PLAYER_START_POSITION).length();
    assert!(
        distance_to_player <= FIGHTER_REACH + REACH_MARGIN,
        "fighter stopped {} away from the player at {translation:?}",
        distance_to_player
    );
    assert!(attacking_observed, "fighter never started attacking the player");
}
