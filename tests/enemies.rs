use avian3d::prelude::*;
use bevy::asset::AssetEvent;
use bevy::ecs::message::Messages;
use bevy::ecs::system::RunSystemOnce;
use bevy::prelude::*;
use bevy::world_serialization::{WorldAsset, WorldAssetRoot};
use cathedral::app::build_headless_app;
use cathedral::enemies::{Enemy, EnemyKind, Fighter, Gunner, spawn_enemy};
use cathedral::maps::{CurrentMap, PlayerPosition};
use cathedral::player::Player;
use cathedral::ragdoll::{OwnedByEnemy, RagdollBodyPart};
use cathedral::shooting::shoot;
use std::time::Duration;

const EXPECTED_RAGDOLL_BODIES: usize = 14;
const MAX_KICK_SPEED: f32 = 12.0;

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
    let fixed_timestep = Duration::from_secs_f64(1.0 / 64.0);
    let player_position = PlayerPosition::new(Vec3::new(5.0, 0.5, 0.0));
    let mut app = build_headless_app(CurrentMap::FlatFloor, Some(player_position));
    app.insert_resource(Time::<Fixed>::from_duration(fixed_timestep))
        .insert_resource(TestEnemyKind(kind))
        .add_systems(Startup, spawn_test_enemy);
    app
}

fn count_ragdoll_bodies(world: &mut World) -> usize {
    world
        .query_filtered::<Entity, With<RagdollBodyPart>>()
        .iter(world)
        .count()
}

fn wait_for_inert_ragdoll(app: &mut App) {
    let asset_load_attempts = 10_000;
    for _ in 0..asset_load_attempts {
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
    wait_for_inert_ragdoll(&mut app);
    let enemy = app
        .world_mut()
        .query_filtered::<Entity, With<Enemy>>()
        .single(app.world())
        .expect("one enemy should spawn");
    let walking_updates = 512;
    for _ in 0..walking_updates {
        app.update();
    }
    (app, enemy)
}

fn find_ragdoll_body(app: &mut App, enemy: Entity, part: RagdollBodyPart) -> (Entity, Vec3) {
    app.world_mut()
        .query_filtered::<(Entity, &OwnedByEnemy, &Position, &RagdollBodyPart), ()>()
        .iter(app.world())
        .find(|(_, owner, _, body_part)| owner.0 == enemy && **body_part == part)
        .map(|(entity, _, position, _)| (entity, position.0))
        .unwrap_or_else(|| panic!("ragdoll should have {part:?}"))
}

fn aim_gun_at_body(
    target: Res<ShotTarget>,
    mut player: Query<&mut Transform, With<Player>>,
    positions: Query<&Position>,
    spatial_query: SpatialQuery,
    owners: Query<&OwnedByEnemy>,
) -> Vec3 {
    let test_ragdoll_group = 0b10;
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
                &SpatialQueryFilter::from_mask(test_ragdoll_group),
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
    let fixed_timestep_seconds = 1.0 / 64.0;
    let settling_deadline_seconds = 10.0;
    let settling_steps = (settling_deadline_seconds / fixed_timestep_seconds) as usize;
    for _ in 0..settling_steps {
        app.update();
        if list_awake_ragdoll_bodies(app, enemy).is_empty() {
            break;
        }
    }
    let still_awake_bodies = list_awake_ragdoll_bodies(app, enemy);
    assert!(
        still_awake_bodies.is_empty(),
        "ragdoll bodies did not sleep within {settling_deadline_seconds} seconds:\n{}",
        still_awake_bodies.join("\n")
    );
    let cloud_radius = 5.0;
    let bodies = app
        .world_mut()
        .query::<(
            &OwnedByEnemy,
            &RagdollBodyPart,
            &Position,
            &LinearVelocity,
            Has<RigidBodyDisabled>,
            Has<Sleeping>,
        )>()
        .iter(app.world())
        .filter(|(owner, ..)| owner.0 == enemy)
        .map(|(_, _, position, velocity, disabled, sleeping)| (position.0, velocity.0, disabled, sleeping))
        .collect::<Vec<_>>();
    assert_eq!(bodies.len(), EXPECTED_RAGDOLL_BODIES);
    for (position, velocity, disabled, sleeping) in &bodies {
        assert!(!*disabled, "ragdoll body should be dynamic after the kill");
        assert!(position.is_finite() && velocity.is_finite());
        assert!(
            position.distance(kill_position) < cloud_radius,
            "body scattered from kill position {kill_position:?} to {position:?}"
        );
        assert!(
            *sleeping,
            "body still moving at {} m/s, the ragdoll never settles",
            velocity.length()
        );
    }
}

fn list_awake_ragdoll_bodies(app: &mut App, enemy: Entity) -> Vec<String> {
    app.world_mut()
        .query::<(
            &OwnedByEnemy,
            &RagdollBodyPart,
            &RigidBody,
            &LinearVelocity,
            &AngularVelocity,
            Has<Sleeping>,
        )>()
        .iter(app.world())
        .filter(|(owner, _, body, _, _, sleeping)| owner.0 == enemy && **body == RigidBody::Dynamic && !sleeping)
        .map(|(_, part, _, velocity, angular_velocity, _)| {
            format!(
                "{part:?}: linear_speed={:.6}, angular_speed={:.6}",
                velocity.length(),
                angular_velocity.length()
            )
        })
        .collect()
}

fn assert_body_received_bounded_kick(world: &World, body: Entity, shot_direction: Vec3) {
    let kick = world
        .get::<LinearVelocity>(body)
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

    fire_gun(&mut app);

    assert!(app.world().entity(enemy).contains::<cathedral::enemies::Dying>());
    assert_body_received_bounded_kick(app.world(), hit_body, shot_direction);

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

    fire_gun(&mut app);

    assert!(app.world().entity(enemy).contains::<cathedral::enemies::Dying>());
    assert_body_received_bounded_kick(app.world(), hit_body, shot_direction);

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

    fire_gun(&mut app);

    assert!(app.world().entity(enemy).contains::<cathedral::enemies::Dying>());
    assert_body_received_bounded_kick(app.world(), hit_body, shot_direction);

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

    fire_gun(&mut app);

    assert!(app.world().entity(enemy).contains::<cathedral::enemies::Dying>());
    assert_body_received_bounded_kick(app.world(), hit_body, shot_direction);

    respawn_scene_instance(&mut app, enemy);
    assert_ragdoll_settled(&mut app, enemy, part_position);
}
