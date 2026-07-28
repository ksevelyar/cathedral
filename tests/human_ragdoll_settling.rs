use avian3d::prelude::*;
use bevy::animation::AnimationPlugin;
use bevy::gltf::GltfPlugin;
use bevy::image::Image;
use bevy::mesh::MeshPlugin;
use bevy::prelude::*;
use bevy::time::TimeUpdateStrategy;
use bevy::world_serialization::WorldSerializationPlugin;
use cathedral::enemies::{EnemyKind, EnemyLifeState, EnemySpawn, spawn_enemy};
use cathedral::ragdoll::RagdollPlugin;
use std::time::Duration;

const FIXED_TIMESTEP_SECONDS: f64 = 1.0 / 64.0;
const SLEEP_DEADLINE_SECONDS: f64 = 10.0;
const ASSET_LOAD_ATTEMPTS: usize = 10_000;
const EXPECTED_RAGDOLL_BODIES: usize = 14;

fn spawn_dead_enemy(mut commands: Commands, asset_server: Res<AssetServer>) {
    spawn_enemy(
        &mut commands,
        &asset_server,
        EnemySpawn {
            kind: EnemyKind::Standard,
            transform: Transform::from_xyz(0.0, 0.5, 0.0),
            life_state: EnemyLifeState::Dead,
        },
    );
}

fn create_test_app() -> App {
    let fixed_timestep = Duration::from_secs_f64(FIXED_TIMESTEP_SECONDS);
    let mut app = App::new();
    app.add_plugins((
        MinimalPlugins,
        TransformPlugin,
        AssetPlugin::default(),
        WorldSerializationPlugin,
        MeshPlugin,
        AnimationPlugin,
        GltfPlugin::default(),
        PhysicsPlugins::default(),
        RagdollPlugin,
    ))
    .add_systems(Startup, spawn_dead_enemy)
    .init_asset::<Image>()
    .insert_resource(Time::<Fixed>::from_duration(fixed_timestep))
    .insert_resource(TimeUpdateStrategy::ManualDuration(fixed_timestep))
    .insert_resource(SubstepCount(6));
    app.finish();
    app.cleanup();
    app.world_mut().resource_mut::<Time<Physics>>().pause();
    app.world_mut().spawn((
        Name::new("floor"),
        RigidBody::Static,
        Collider::cuboid(20.0, 0.2, 20.0),
        Transform::from_xyz(0.0, -0.1, 0.0),
    ));
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

#[test]
fn human_ragdoll_sleeps_before_deadline() {
    let mut app = create_test_app();
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
