use avian3d::prelude::*;
use bevy::prelude::*;
use cathedral::app::build_headless_app;
use cathedral::enemies::{EnemyKind, Fighter, Gunner, spawn_enemy};
use cathedral::maps::CurrentMap;
use std::time::Duration;

const FIXED_TIMESTEP_SECONDS: f64 = 1.0 / 64.0;
const SLEEP_DEADLINE_SECONDS: f64 = 10.0;
const ASSET_LOAD_ATTEMPTS: usize = 10_000;
const EXPECTED_RAGDOLL_BODIES: usize = 14;

#[derive(Resource, Clone)]
struct TestEnemyKind(EnemyKind);

fn spawn_test_enemy(mut commands: Commands, asset_server: Res<AssetServer>, kind: Res<TestEnemyKind>) {
    spawn_enemy(
        &mut commands,
        &asset_server,
        kind.0.clone(),
        Transform::from_xyz(0.0, 0.5, 0.0),
        false,
    );
}

fn create_test_app(kind: EnemyKind) -> App {
    let fixed_timestep = Duration::from_secs_f64(FIXED_TIMESTEP_SECONDS);
    let mut app = build_headless_app(CurrentMap::FlatFloor, None);
    app.insert_resource(Time::<Fixed>::from_duration(fixed_timestep))
        .insert_resource(SubstepCount(6))
        .insert_resource(TestEnemyKind(kind))
        .add_systems(Startup, spawn_test_enemy);
    app.world_mut().resource_mut::<Time<Physics>>().pause();
    app
}

fn dynamic_collider_count(world: &mut World) -> usize {
    world
        .query_filtered::<&RigidBody, With<Collider>>()
        .iter(world)
        .filter(|body| **body == RigidBody::Dynamic)
        .count()
}

fn wait_for_ragdoll(app: &mut App) {
    for _ in 0..ASSET_LOAD_ATTEMPTS {
        app.update();
        if dynamic_collider_count(app.world_mut()) == EXPECTED_RAGDOLL_BODIES {
            return;
        }
        std::thread::yield_now();
    }
    panic!(
        "human ragdoll did not spawn: expected {EXPECTED_RAGDOLL_BODIES} bodies, found {}",
        dynamic_collider_count(app.world_mut())
    );
}

fn all_ragdoll_bodies_sleeping(world: &mut World) -> bool {
    let mut bodies = world.query_filtered::<(&RigidBody, Has<Sleeping>), With<Collider>>();
    let mut dynamic_body_count = 0;
    for (body, sleeping) in bodies.iter(world) {
        if *body == RigidBody::Dynamic {
            dynamic_body_count += 1;
            if !sleeping {
                return false;
            }
        }
    }
    dynamic_body_count == EXPECTED_RAGDOLL_BODIES
}

fn run_ragdoll_sleep_test(kind: EnemyKind) {
    let mut app = create_test_app(kind);
    wait_for_ragdoll(&mut app);
    app.world_mut().resource_mut::<Time<Physics>>().unpause();

    let simulation_steps = (SLEEP_DEADLINE_SECONDS / FIXED_TIMESTEP_SECONDS) as usize;
    for _ in 0..simulation_steps {
        app.update();
        if all_ragdoll_bodies_sleeping(app.world_mut()) {
            return;
        }
    }

    let world = app.world_mut();
    let mut bodies =
        world.query_filtered::<(&Name, &RigidBody, &LinearVelocity, &AngularVelocity, Has<Sleeping>), With<Collider>>();
    let mut awake_bodies = Vec::new();
    for (name, body, linear_velocity, angular_velocity, sleeping) in bodies.iter(world) {
        if *body != RigidBody::Dynamic || sleeping {
            continue;
        }
        awake_bodies.push(format!(
            "{}: linear_speed={:.6}, angular_speed={:.6}",
            name.as_str(),
            linear_velocity.length(),
            angular_velocity.length()
        ));
    }

    panic!(
        "human ragdoll did not sleep within {SLEEP_DEADLINE_SECONDS} seconds:\n{}",
        awake_bodies.join("\n")
    );
}

#[test]
fn fighter_ragdoll_sleeps_before_deadline() {
    run_ragdoll_sleep_test(EnemyKind::Fighter(Fighter::default()));
}

#[test]
fn gunner_ragdoll_sleeps_before_deadline() {
    run_ragdoll_sleep_test(EnemyKind::Gunner(Gunner::default()));
}
