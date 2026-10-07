use avian3d::prelude::*;
use bevy::prelude::*;
use cathedral::app::build_headless_app;
use cathedral::enemies::{Enemy, EnemyKind, Fighter, Gunner, spawn_enemy};
use cathedral::maps::{CurrentMap, PlayerPosition};
use cathedral::ragdoll::{OwnedByEnemy, RagdollBodyPart};
use std::time::Duration;

mod support;

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
    let fixed_timestep = Duration::from_secs_f64(support::FIXED_TIMESTEP_SECONDS);
    let player_position = PlayerPosition::new(Vec3::new(5.0, 0.5, 0.0));
    let mut app = build_headless_app(CurrentMap::FlatFloor, Some(player_position));
    app.insert_resource(Time::<Fixed>::from_duration(fixed_timestep))
        .insert_resource(TestEnemyKind(kind))
        .add_systems(Startup, spawn_test_enemy);
    app
}

fn wait_for_inert_ragdoll(app: &mut App) {
    let expected_ragdoll_bodies = 14;
    let asset_load_attempts = 10_000;
    for _ in 0..asset_load_attempts {
        app.update();
        let body_count = app
            .world_mut()
            .query_filtered::<Entity, With<RagdollBodyPart>>()
            .iter(app.world())
            .count();
        if body_count == expected_ragdoll_bodies {
            let inert_body_count = app
                .world_mut()
                .query::<(&RagdollBodyPart, Has<RigidBodyDisabled>)>()
                .iter(app.world())
                .filter(|(_, disabled)| *disabled)
                .count();
            assert_eq!(inert_body_count, expected_ragdoll_bodies);
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

fn find_ragdoll_body(app: &mut App, enemy: Entity, part: RagdollBodyPart) -> Entity {
    app.world_mut()
        .query_filtered::<(Entity, &OwnedByEnemy, &RagdollBodyPart), ()>()
        .iter(app.world())
        .find(|(_, owner, body_part)| owner.0 == enemy && **body_part == part)
        .map(|(entity, _, _)| entity)
        .unwrap_or_else(|| panic!("ragdoll should have {part:?}"))
}

#[test]
fn head_shot_kills_fighter() {
    let (mut app, enemy) = spawn_walking_enemy_with_player(EnemyKind::Fighter(Fighter::default()));
    let hit_body = find_ragdoll_body(&mut app, enemy, RagdollBodyPart::Head);

    support::shoot_entity(&mut app, hit_body);
    app.update();
    assert!(app.world().entity(enemy).contains::<cathedral::enemies::Dying>());
    support::assert_received_kick(&app, hit_body);
    let settling_duration = 10.0;
    support::assert_settles(&mut app, hit_body, settling_duration);
}

#[test]
fn head_shot_kills_gunner() {
    let (mut app, enemy) = spawn_walking_enemy_with_player(EnemyKind::Gunner(Gunner::default()));
    let hit_body = find_ragdoll_body(&mut app, enemy, RagdollBodyPart::Head);

    support::shoot_entity(&mut app, hit_body);
    app.update();
    assert!(app.world().entity(enemy).contains::<cathedral::enemies::Dying>());
    support::assert_received_kick(&app, hit_body);
    let settling_duration = 10.0;
    support::assert_settles(&mut app, hit_body, settling_duration);
}

#[test]
fn hand_shot_kills_fighter() {
    let (mut app, enemy) = spawn_walking_enemy_with_player(EnemyKind::Fighter(Fighter::default()));
    let hit_body = find_ragdoll_body(&mut app, enemy, RagdollBodyPart::RightHand);

    support::shoot_entity(&mut app, hit_body);
    app.update();
    assert!(app.world().entity(enemy).contains::<cathedral::enemies::Dying>());
    support::assert_received_kick(&app, hit_body);
    let settling_duration = 10.0;
    support::assert_settles(&mut app, hit_body, settling_duration);
}

#[test]
fn hand_shot_kills_gunner() {
    let (mut app, enemy) = spawn_walking_enemy_with_player(EnemyKind::Gunner(Gunner::default()));
    let hit_body = find_ragdoll_body(&mut app, enemy, RagdollBodyPart::RightHand);

    support::shoot_entity(&mut app, hit_body);
    app.update();
    assert!(app.world().entity(enemy).contains::<cathedral::enemies::Dying>());
    support::assert_received_kick(&app, hit_body);
    let settling_duration = 10.0;
    support::assert_settles(&mut app, hit_body, settling_duration);
}
