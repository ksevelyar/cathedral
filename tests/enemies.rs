use avian3d::prelude::*;
use bevy::animation::AnimationPlugin;
use bevy::asset::AssetEvent;
use bevy::ecs::message::Messages;
use bevy::ecs::system::RunSystemOnce;
use bevy::gltf::GltfPlugin;
use bevy::image::Image;
use bevy::mesh::MeshPlugin;
use bevy::prelude::*;
use bevy::time::TimeUpdateStrategy;
use bevy::world_serialization::WorldSerializationPlugin;
use bevy::world_serialization::{WorldAsset, WorldAssetRoot};
use cathedral::enemies::{EnemiesPlugin, Enemy, EnemyKind, Fighter, Gunner, spawn_enemy};
use cathedral::player::Player;
use cathedral::ragdoll::{OwnedByEnemy, RagdollBodyPart, RagdollPlugin};
use cathedral::shooting::shoot;
use cathedral::state::GameStatePlugin;
use std::time::Duration;

const FIXED_TIMESTEP_SECONDS: f64 = 1.0 / 64.0;
const ASSET_LOAD_ATTEMPTS: usize = 10_000;
const EXPECTED_RAGDOLL_BODIES: usize = 14;
const WALKING_UPDATES: usize = 512;
const CLOUD_RADIUS: f32 = 5.0;
const MAX_KICK_SPEED: f32 = 12.0;
const TEST_RAGDOLL_GROUP: u32 = 0b10;

#[derive(Resource, Clone)]
struct TestEnemyKind(EnemyKind);

fn spawn_test_enemy(mut commands: Commands, asset_server: Res<AssetServer>, kind: Res<TestEnemyKind>) {
    spawn_enemy(
        &mut commands,
        &asset_server,
        kind.0.clone(),
        Transform::from_xyz(0.0, 0.5, 0.0),
        true,
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
        bevy::state::app::StatesPlugin,
        GameStatePlugin,
        EnemiesPlugin,
        RagdollPlugin,
    ))
    .init_asset::<Image>()
    .insert_resource(Time::<Fixed>::from_duration(fixed_timestep))
    .insert_resource(TimeUpdateStrategy::ManualDuration(fixed_timestep))
    .insert_resource(bevy::prelude::GizmoConfigStore::default())
    .insert_resource(TestEnemyKind(kind))
    .add_systems(Startup, spawn_test_enemy)
    .add_systems(Update, shoot);
    app.finish();
    app.cleanup();
    app
}

fn count_ragdoll_bodies(world: &mut World) -> usize {
    world
        .query_filtered::<Entity, With<RagdollBodyPart>>()
        .iter(world)
        .count()
}

fn wait_for_inert_ragdoll(app: &mut App) {
    for _ in 0..ASSET_LOAD_ATTEMPTS {
        app.update();
        if count_ragdoll_bodies(app.world_mut()) == EXPECTED_RAGDOLL_BODIES {
            let inert_body_count = app
                .world_mut()
                .query::<(&RagdollBodyPart, Has<RigidBodyDisabled>)>()
                .iter(app.world())
                .filter(|(_, disabled)| *disabled)
                .count();
            assert_eq!(inert_body_count, EXPECTED_RAGDOLL_BODIES);
            return;
        }
        std::thread::yield_now();
    }
    panic!("alive enemy ragdoll colliders did not spawn");
}

fn spawn_walking_enemy_with_player(kind: EnemyKind) -> (App, Entity) {
    let mut app = create_test_app(kind);
    app.world_mut().spawn((Player, Transform::from_xyz(5.0, 0.5, 0.0)));
    wait_for_inert_ragdoll(&mut app);
    let enemy = app
        .world_mut()
        .query_filtered::<Entity, With<Enemy>>()
        .single(app.world())
        .expect("one enemy should spawn");
    for _ in 0..WALKING_UPDATES {
        app.update();
    }
    (app, enemy)
}

fn find_ragdoll_body(app: &mut App, enemy: Entity, part: RagdollBodyPart) -> (Entity, Vec3) {
    app.world_mut()
        .query_filtered::<(Entity, &Position, &RagdollBodyPart), With<OwnedByEnemy>>()
        .iter(app.world())
        .find(|(entity, _, body_part)| {
            **body_part == part
                && app
                    .world()
                    .get::<OwnedByEnemy>(*entity)
                    .is_some_and(|owner| owner.0 == enemy)
        })
        .map(|(entity, position, _)| (entity, position.0))
        .unwrap_or_else(|| panic!("ragdoll should have {part:?}"))
}

fn aim_gun_at_body(
    target: Res<ShotTarget>,
    mut player: Query<&mut Transform, With<Player>>,
    positions: Query<&Position>,
    spatial_query: SpatialQuery,
    owners: Query<&OwnedByEnemy>,
) -> Vec3 {
    let target_position = positions.get(target.0).expect("target body position").0;
    let offsets = [
        Vec3::Z * 2.0,
        Vec3::X * 2.0,
        Vec3::NEG_X * 2.0,
        Vec3::Y * 2.0,
        Vec3::NEG_Y * 2.0,
        Vec3::NEG_Z * 2.0,
    ];
    for offset in offsets {
        let camera_transform =
            Transform::from_translation(target_position + offset).looking_at(target_position, Vec3::Y);
        let direction = camera_transform.forward();
        if spatial_query
            .cast_ray(
                camera_transform.translation,
                direction,
                100.0,
                true,
                &SpatialQueryFilter::from_mask(TEST_RAGDOLL_GROUP),
            )
            .is_some_and(|hit| hit.entity == target.0 && owners.get(hit.entity).is_ok())
        {
            *player.single_mut().expect("one player should spawn") = camera_transform;
            return direction.as_vec3();
        }
    }
    panic!("target body is not visible to a ray");
}

#[derive(Resource, Clone, Copy)]
struct ShotTarget(Entity);

fn fire_gun(app: &mut App) {
    app.world_mut()
        .resource_mut::<ButtonInput<MouseButton>>()
        .press(MouseButton::Left);
    app.world_mut().run_system_once(shoot).expect("shot system should run");
    app.update();
    app.update();
}

fn print_ragdoll_metrics(app: &mut App, enemy: Entity, phase: &str) {
    let bodies = app
        .world_mut()
        .query::<(
            Entity,
            &OwnedByEnemy,
            &RagdollBodyPart,
            &Position,
            &LinearVelocity,
            Option<&ComputedMass>,
        )>()
        .iter(app.world())
        .filter(|(_, owner, ..)| owner.0 == enemy)
        .map(|(entity, _, part, position, velocity, mass)| {
            let mass = mass.map(|mass| 1.0 / mass.inverse()).unwrap_or_default();
            (entity, *part, position.0, velocity.0, mass)
        })
        .collect::<Vec<_>>();
    let total_momentum = bodies
        .iter()
        .map(|(_, _, _, velocity, mass)| *velocity * *mass)
        .sum::<Vec3>();
    let total_energy = bodies
        .iter()
        .map(|(_, _, _, velocity, mass)| 0.5 * *mass * velocity.length_squared())
        .sum::<f32>();
    let (max_speed, max_speed_part) = bodies
        .iter()
        .map(|(_, part, _, velocity, _)| (velocity.length(), *part))
        .max_by(|left, right| left.0.total_cmp(&right.0))
        .unwrap_or((0.0, RagdollBodyPart::Torso));

    let mut joints = app
        .world_mut()
        .query::<(Entity, &OwnedByEnemy, &SphericalJoint)>()
        .iter(app.world())
        .filter(|(_, owner, _)| owner.0 == enemy)
        .map(|(entity, _, joint)| (entity, joint.body1, joint.body2, joint.frame1, joint.frame2))
        .collect::<Vec<_>>();
    joints.extend(
        app.world_mut()
            .query::<(Entity, &OwnedByEnemy, &RevoluteJoint)>()
            .iter(app.world())
            .filter(|(_, owner, _)| owner.0 == enemy)
            .map(|(entity, _, joint)| (entity, joint.body1, joint.body2, joint.frame1, joint.frame2)),
    );
    let mut max_anchor_error = 0.0;
    let mut worst_joint = Entity::PLACEHOLDER;
    for (joint, body1, body2, frame1, frame2) in joints {
        let Some((_, _, position1, _, _)) = bodies.iter().find(|(entity, ..)| *entity == body1) else {
            continue;
        };
        let Some((_, _, position2, _, _)) = bodies.iter().find(|(entity, ..)| *entity == body2) else {
            continue;
        };
        let rotation1 = app
            .world()
            .get::<Rotation>(body1)
            .map_or(Quat::IDENTITY, |rotation| rotation.0);
        let rotation2 = app
            .world()
            .get::<Rotation>(body2)
            .map_or(Quat::IDENTITY, |rotation| rotation.0);
        let anchor1 = match frame1.anchor {
            JointAnchor::Local(anchor) => *position1 + rotation1 * anchor,
            JointAnchor::FromGlobal(anchor) => anchor,
        };
        let anchor2 = match frame2.anchor {
            JointAnchor::Local(anchor) => *position2 + rotation2 * anchor,
            JointAnchor::FromGlobal(anchor) => anchor,
        };
        let error = anchor1.distance(anchor2);
        if error > max_anchor_error {
            max_anchor_error = error;
            worst_joint = joint;
        }
    }

    println!(
        "RAGDOLL_METRIC phase={phase} bodies={} max_speed={max_speed:.4} max_speed_part={max_speed_part:?} momentum={total_momentum:?} energy={total_energy:.4} max_anchor_error={max_anchor_error:.6} worst_joint={worst_joint:?}",
        bodies.len(),
    );
}

fn respawn_scene_instance(app: &mut App, enemy: Entity) {
    let scene_handle = app
        .world()
        .get::<WorldAssetRoot>(enemy)
        .expect("enemy scene root")
        .0
        .clone();
    app.world_mut()
        .resource_mut::<Messages<AssetEvent<WorldAsset>>>()
        .write(AssetEvent::Modified { id: scene_handle.id() });
    for _ in 0..30 {
        app.update();
    }
}

fn assert_ragdoll_settled(app: &mut App, enemy: Entity, kill_position: Vec3) {
    let bodies = app
        .world_mut()
        .query::<(&OwnedByEnemy, &Position, &LinearVelocity, Has<RigidBodyDisabled>)>()
        .iter(app.world())
        .filter(|(owner, ..)| owner.0 == enemy)
        .map(|(_, position, velocity, disabled)| (position.0, velocity.0, disabled))
        .collect::<Vec<_>>();
    assert_eq!(bodies.len(), EXPECTED_RAGDOLL_BODIES);
    for (position, velocity, disabled) in &bodies {
        assert!(!*disabled, "ragdoll body should be dynamic after the kill");
        assert!(position.is_finite() && velocity.is_finite());
        assert!(
            position.distance(kill_position) < CLOUD_RADIUS,
            "body scattered from kill position {kill_position:?} to {position:?}"
        );
        assert!(
            velocity.length() < MAX_KICK_SPEED,
            "body still moving at {} m/s, the ragdoll never settles",
            velocity.length()
        );
    }
}

#[test]
fn head_shot_kills_fighter_without_scattering_ragdoll() {
    let (mut app, enemy) = spawn_walking_enemy_with_player(EnemyKind::Fighter(Fighter::default()));
    let (hit_body, part_position) = find_ragdoll_body(&mut app, enemy, RagdollBodyPart::Head);
    app.insert_resource(ShotTarget(hit_body));
    let shot_direction = app
        .world_mut()
        .run_system_once(aim_gun_at_body)
        .expect("target body should be aimable");
    print_ragdoll_metrics(&mut app, enemy, "before_shot");

    fire_gun(&mut app);
    print_ragdoll_metrics(&mut app, enemy, "after_first_step");

    assert!(app.world().entity(enemy).contains::<cathedral::enemies::Dying>());
    let kick = app
        .world()
        .get::<LinearVelocity>(hit_body)
        .expect("hit body should have velocity after the shot")
        .0;
    assert!(
        kick.dot(shot_direction) > 0.0,
        "kick {kick:?} should push the hit body along the shot direction {shot_direction:?}"
    );
    assert!(
        kick.length() < MAX_KICK_SPEED,
        "kick launched the hit body at {} m/s, it must be mass-capped below {MAX_KICK_SPEED} m/s",
        kick.length()
    );

    respawn_scene_instance(&mut app, enemy);
    assert_ragdoll_settled(&mut app, enemy, part_position);
}

#[test]
fn head_shot_kills_gunner_without_scattering_ragdoll() {
    let (mut app, enemy) = spawn_walking_enemy_with_player(EnemyKind::Gunner(Gunner::default()));
    let (hit_body, part_position) = find_ragdoll_body(&mut app, enemy, RagdollBodyPart::Head);
    app.insert_resource(ShotTarget(hit_body));
    let shot_direction = app
        .world_mut()
        .run_system_once(aim_gun_at_body)
        .expect("target body should be aimable");
    print_ragdoll_metrics(&mut app, enemy, "before_shot");

    fire_gun(&mut app);
    print_ragdoll_metrics(&mut app, enemy, "after_first_step");

    assert!(app.world().entity(enemy).contains::<cathedral::enemies::Dying>());
    let kick = app
        .world()
        .get::<LinearVelocity>(hit_body)
        .expect("hit body should have velocity after the shot")
        .0;
    assert!(
        kick.dot(shot_direction) > 0.0,
        "kick {kick:?} should push the hit body along the shot direction {shot_direction:?}"
    );
    assert!(
        kick.length() < MAX_KICK_SPEED,
        "kick launched the hit body at {} m/s, it must be mass-capped below {MAX_KICK_SPEED} m/s",
        kick.length()
    );

    respawn_scene_instance(&mut app, enemy);
    assert_ragdoll_settled(&mut app, enemy, part_position);
}

#[test]
fn hand_shot_kills_fighter_without_scattering_ragdoll() {
    let (mut app, enemy) = spawn_walking_enemy_with_player(EnemyKind::Fighter(Fighter::default()));
    let (hit_body, part_position) = find_ragdoll_body(&mut app, enemy, RagdollBodyPart::LeftHand);
    app.insert_resource(ShotTarget(hit_body));
    let shot_direction = app
        .world_mut()
        .run_system_once(aim_gun_at_body)
        .expect("target body should be aimable");
    print_ragdoll_metrics(&mut app, enemy, "before_shot");

    fire_gun(&mut app);
    print_ragdoll_metrics(&mut app, enemy, "after_first_step");

    assert!(app.world().entity(enemy).contains::<cathedral::enemies::Dying>());
    let kick = app
        .world()
        .get::<LinearVelocity>(hit_body)
        .expect("hit body should have velocity after the shot")
        .0;
    assert!(
        kick.dot(shot_direction) > 0.0,
        "kick {kick:?} should push the hit body along the shot direction {shot_direction:?}"
    );
    assert!(
        kick.length() < MAX_KICK_SPEED,
        "kick launched the hit body at {} m/s, it must be mass-capped below {MAX_KICK_SPEED} m/s",
        kick.length()
    );

    respawn_scene_instance(&mut app, enemy);
    assert_ragdoll_settled(&mut app, enemy, part_position);
}

#[test]
fn hand_shot_kills_gunner_without_scattering_ragdoll() {
    let (mut app, enemy) = spawn_walking_enemy_with_player(EnemyKind::Gunner(Gunner::default()));
    let (hit_body, part_position) = find_ragdoll_body(&mut app, enemy, RagdollBodyPart::LeftHand);
    app.insert_resource(ShotTarget(hit_body));
    let shot_direction = app
        .world_mut()
        .run_system_once(aim_gun_at_body)
        .expect("target body should be aimable");
    print_ragdoll_metrics(&mut app, enemy, "before_shot");

    fire_gun(&mut app);
    print_ragdoll_metrics(&mut app, enemy, "after_first_step");

    assert!(app.world().entity(enemy).contains::<cathedral::enemies::Dying>());
    let kick = app
        .world()
        .get::<LinearVelocity>(hit_body)
        .expect("hit body should have velocity after the shot")
        .0;
    assert!(
        kick.dot(shot_direction) > 0.0,
        "kick {kick:?} should push the hit body along the shot direction {shot_direction:?}"
    );
    assert!(
        kick.length() < MAX_KICK_SPEED,
        "kick launched the hit body at {} m/s, it must be mass-capped below {MAX_KICK_SPEED} m/s",
        kick.length()
    );

    respawn_scene_instance(&mut app, enemy);
    assert_ragdoll_settled(&mut app, enemy, part_position);
}
