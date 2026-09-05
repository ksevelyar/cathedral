use avian3d::prelude::*;
use bevy::animation::AnimationPlugin;
use bevy::ecs::system::RunSystemOnce;
use bevy::gltf::GltfPlugin;
use bevy::image::Image;
use bevy::mesh::MeshPlugin;
use bevy::prelude::*;
use bevy::time::TimeUpdateStrategy;
use bevy::world_serialization::WorldSerializationPlugin;
use cathedral::enemies::{Enemy, EnemyKind, EnemyLifeState, EnemySpawn, spawn_enemy};
use cathedral::player::Player;
use cathedral::ragdoll::{OwnedByEnemy, RagdollBodyPart, RagdollPlugin};
use cathedral::shooting::shoot;
use std::time::Duration;

const FIXED_TIMESTEP_SECONDS: f64 = 1.0 / 64.0;
const ASSET_LOAD_ATTEMPTS: usize = 10_000;
const EXPECTED_RAGDOLL_BODIES: usize = 14;

#[derive(Resource, Clone, Copy)]
struct TestEnemyKind(EnemyKind);

fn spawn_test_enemy(mut commands: Commands, asset_server: Res<AssetServer>, kind: Res<TestEnemyKind>) {
    spawn_enemy(
        &mut commands,
        &asset_server,
        EnemySpawn {
            kind: kind.0,
            transform: Transform::from_xyz(0.0, 0.5, 0.0),
            life_state: EnemyLifeState::Alive,
        },
    );
}

fn create_test_app(kind: EnemyKind) -> App {
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
        RagdollPlugin,
    ))
    .init_asset::<Image>()
    .insert_resource(Time::<Fixed>::from_duration(fixed_timestep))
    .insert_resource(TimeUpdateStrategy::ManualDuration(fixed_timestep))
    .insert_resource(TestEnemyKind(kind))
    .add_systems(Startup, spawn_test_enemy)
    .add_systems(Update, shoot);
    app.finish();
    app.cleanup();
    app.world_mut().resource_mut::<Time<Physics>>().pause();
    app
}

fn ragdoll_body_count(world: &mut World) -> usize {
    world
        .query_filtered::<Entity, With<RagdollBodyPart>>()
        .iter(world)
        .count()
}

fn wait_for_ragdoll(app: &mut App) {
    for _ in 0..ASSET_LOAD_ATTEMPTS {
        app.update();
        if ragdoll_body_count(app.world_mut()) == EXPECTED_RAGDOLL_BODIES {
            return;
        }
        std::thread::yield_now();
    }
    panic!("alive enemy ragdoll colliders did not spawn");
}

fn run_shooting_test(kind: EnemyKind) {
    let mut app = create_test_app(kind);
    wait_for_ragdoll(&mut app);

    let enemy = app
        .world_mut()
        .query_filtered::<Entity, With<Enemy>>()
        .single(app.world())
        .expect("one enemy should spawn");
    let (head, head_position) = app
        .world_mut()
        .query::<(Entity, &Position, &RagdollBodyPart)>()
        .iter(app.world())
        .find_map(|(entity, position, body_part)| (*body_part == RagdollBodyPart::Head).then_some((entity, position.0)))
        .expect("ragdoll should have a head");

    let camera_position = head_position + Vec3::Z * 3.0;
    let aim_point = head_position + Vec3::X * 0.05;
    let camera_transform = Transform::from_translation(camera_position).looking_at(aim_point, Vec3::Y);
    let shot_direction = camera_transform.forward().as_vec3();
    app.world_mut().spawn((Player, camera_transform));

    let disabled_owned_body_count = app
        .world_mut()
        .query::<(&OwnedByEnemy, &RagdollBodyPart, Has<RigidBodyDisabled>)>()
        .iter(app.world())
        .filter(|(owner, _, disabled)| owner.0 == enemy && *disabled)
        .count();
    assert_eq!(disabled_owned_body_count, EXPECTED_RAGDOLL_BODIES);

    app.world_mut().resource_mut::<Time<Physics>>().unpause();
    app.update();
    app.world_mut()
        .resource_mut::<ButtonInput<MouseButton>>()
        .press(MouseButton::Left);
    app.world_mut().run_system_once(shoot).expect("shot system should run");
    app.update();

    assert!(app.world().entity(enemy).contains::<cathedral::enemies::Dying>());

    let dynamic_owned_body_count = app
        .world_mut()
        .query::<(&OwnedByEnemy, &RigidBody)>()
        .iter(app.world())
        .filter(|(owner, rigid_body)| owner.0 == enemy && **rigid_body == RigidBody::Dynamic)
        .count();
    assert_eq!(dynamic_owned_body_count, EXPECTED_RAGDOLL_BODIES);

    let enabled_owned_body_count = app
        .world_mut()
        .query::<(&OwnedByEnemy, &RagdollBodyPart, Has<RigidBodyDisabled>)>()
        .iter(app.world())
        .filter(|(owner, _, disabled)| owner.0 == enemy && !*disabled)
        .count();
    assert_eq!(enabled_owned_body_count, EXPECTED_RAGDOLL_BODIES);

    let head_linear_velocity = app
        .world()
        .get::<LinearVelocity>(head)
        .expect("head should have velocity");
    let head_angular_velocity = app.world().get::<AngularVelocity>(head).expect("head should rotate");
    assert!(head_linear_velocity.dot(shot_direction) > 0.0);
    assert!(head_angular_velocity.length() > 0.0);
}

#[test]
fn shooting_alive_fighter_enemy_in_head_kills_and_impacts_head() {
    run_shooting_test(EnemyKind::Fighter);
}

#[test]
fn shooting_alive_gunner_enemy_in_head_kills_and_impacts_head() {
    run_shooting_test(EnemyKind::Gunner);
}

#[test]
fn gunner_enemy_constructs_ragdoll_bodies() {
    let mut app = create_test_app(EnemyKind::Gunner);
    wait_for_ragdoll(&mut app);

    let body_count = ragdoll_body_count(app.world_mut());
    assert_eq!(body_count, EXPECTED_RAGDOLL_BODIES);
}

#[test]
fn fighter_enemy_constructs_ragdoll_bodies() {
    let mut app = create_test_app(EnemyKind::Fighter);
    wait_for_ragdoll(&mut app);

    let body_count = ragdoll_body_count(app.world_mut());
    assert_eq!(body_count, EXPECTED_RAGDOLL_BODIES);
}
