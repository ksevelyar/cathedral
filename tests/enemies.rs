use avian3d::prelude::*;
use bevy::prelude::*;
use cathedral::app::build_test_app;
use cathedral::enemies::{Dying, Enemy, EnemyKind, Fighter, Gunner, spawn_enemy};
use cathedral::maps::{CurrentMap, PlayerPosition};
use cathedral::ragdoll::{OwnedByCharacter, RagdollBodyPart};
use cathedral::shooting::Weapon;
use std::time::Duration;

pub mod support;

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

fn wait_for_inert_ragdoll(app: &mut App, enemy: Entity) {
    let minimum_ragdoll_bodies = 14;
    let asset_load_attempts = 10_000;
    for _ in 0..asset_load_attempts {
        app.update();
        let enemy_ragdoll_bodies: Vec<_> = app
            .world_mut()
            .query::<(&RagdollBodyPart, &OwnedByCharacter, Has<RigidBodyDisabled>)>()
            .iter(app.world())
            .filter(|(_, owner, _)| owner.0 == enemy)
            .map(|(_, _, disabled)| disabled)
            .collect();
        let body_count = enemy_ragdoll_bodies.len();
        if body_count >= minimum_ragdoll_bodies {
            let inert_body_count = enemy_ragdoll_bodies.iter().filter(|disabled| **disabled).count();
            assert_eq!(inert_body_count, body_count);
            return;
        }
        std::thread::yield_now();
    }
    panic!("alive enemy ragdoll colliders did not spawn");
}

fn spawn_walking_enemy_with_player(kind: EnemyKind) -> (App, Entity) {
    let fixed_timestep = Duration::from_secs_f64(support::FIXED_TIMESTEP_SECONDS);
    let player_position = PlayerPosition::new(Vec3::new(5.0, 0.5, 0.0));
    let mut app = build_test_app(CurrentMap::FlatFloor, Some(player_position));
    app.insert_resource(Time::<Fixed>::from_duration(fixed_timestep))
        .insert_resource(TestEnemyKind(kind))
        .add_systems(Startup, spawn_test_enemy);
    let spawn_wait_updates = 10_000;
    let mut enemy = None;
    for _ in 0..spawn_wait_updates {
        app.update();
        let spawned_enemies: Vec<_> = app
            .world_mut()
            .query_filtered::<Entity, With<Enemy>>()
            .iter(app.world())
            .collect();
        if spawned_enemies.len() == 1 {
            enemy = Some(spawned_enemies[0]);
            break;
        }
    }
    let enemy = enemy.expect("one enemy should spawn");
    wait_for_inert_ragdoll(&mut app, enemy);
    let walking_updates = 512;
    for _ in 0..walking_updates {
        app.update();
    }
    (app, enemy)
}

fn find_enemy_animator(app: &mut App, enemy: Entity) -> Entity {
    let animation_players: Vec<_> = app
        .world_mut()
        .query_filtered::<Entity, With<AnimationPlayer>>()
        .iter(app.world())
        .collect();
    animation_players
        .into_iter()
        .find(|animator| is_descendant_of(app, *animator, enemy))
        .unwrap_or_else(|| panic!("enemy {enemy:?} should have an animation player"))
}

fn is_descendant_of(app: &App, candidate: Entity, ancestor: Entity) -> bool {
    let mut current = candidate;
    while let Some(parent) = app.world().get::<ChildOf>(current).map(|child_of| child_of.parent()) {
        if parent == ancestor {
            return true;
        }
        current = parent;
    }
    false
}

fn find_dropped_weapon(app: &mut App, enemy: Entity) -> Entity {
    app.world_mut()
        .query_filtered::<(Entity, &OwnedByCharacter), With<Weapon>>()
        .iter(app.world())
        .find(|(_, owner)| owner.0 == enemy)
        .map(|(entity, _)| entity)
        .unwrap_or_else(|| panic!("enemy {enemy:?} should drop its weapon"))
}

fn count_playing_animations(app: &mut App, animator: Entity) -> usize {
    app.world()
        .get::<AnimationPlayer>(animator)
        .expect("animator should keep its animation player")
        .playing_animations()
        .count()
}

fn update_until_animator_plays(app: &mut App, animator: Entity, label: &str) {
    let maximum_wait_updates = 10_000;
    for _ in 0..maximum_wait_updates {
        app.update();
        if count_playing_animations(app, animator) > 0 {
            return;
        }
    }
    panic!("{label} animator should start playing an animation");
}

fn run_updates(app: &mut App, update_count: usize) {
    for _ in 0..update_count {
        app.update();
    }
}

fn assert_animator_stays_stopped(app: &mut App, animator: Entity) {
    let settle_updates = 512;
    run_updates(app, settle_updates);
    let playing_count = count_playing_animations(app, animator);
    assert_eq!(playing_count, 0, "killed enemy animator should not resume animations");
}

fn assert_death(app: &mut App, enemy: Entity) {
    run_updates(app, 32);
    assert!(app.world().entity(enemy).contains::<Dying>(), "killed enemy should carry the dying marker");
    let animator = find_enemy_animator(app, enemy);
    assert_animator_stays_stopped(app, animator);
    let weapon = find_dropped_weapon(app, enemy);
    assert!(
        app.world().get::<RigidBody>(weapon).is_some_and(|body| *body == RigidBody::Dynamic),
        "dropped weapon should become a dynamic rigid body"
    );
    assert!(!app.world().get::<ChildOf>(weapon).is_some(), "dropped weapon should be detached from the hand");
}

fn killed_enemy_drops_weapon(kind: EnemyKind) {
    let (mut app, enemy) = spawn_walking_enemy_with_player(kind);
    let hit_body = find_ragdoll_body(&mut app, enemy, RagdollBodyPart::Head);

    support::shoot_entity(&mut app, hit_body);
    assert_death(&mut app, enemy);
}

fn killed_enemy_stops_attacking_animation(kind: EnemyKind) {
    let (mut app, enemy) = spawn_walking_enemy_with_player(kind);
    let animator = find_enemy_animator(&mut app, enemy);
    update_until_animator_plays(&mut app, animator, "alive");
    let hit_body = find_ragdoll_body(&mut app, enemy, RagdollBodyPart::Head);

    support::shoot_entity(&mut app, hit_body);
    assert_death(&mut app, enemy);
}

#[test]
fn killed_fighter_drops_weapon() {
    killed_enemy_drops_weapon(EnemyKind::Fighter(Fighter::default()));
}

#[test]
fn killed_gunner_drops_weapon() {
    killed_enemy_drops_weapon(EnemyKind::Gunner(Gunner::default()));
}

#[test]
fn killed_fighter_stops_attacking_animation() {
    killed_enemy_stops_attacking_animation(EnemyKind::Fighter(Fighter::default()));
}

#[test]
fn killed_gunner_stops_attacking_animation() {
    killed_enemy_stops_attacking_animation(EnemyKind::Gunner(Gunner::default()));
}

fn find_ragdoll_body(app: &mut App, enemy: Entity, part: RagdollBodyPart) -> Entity {
    app.world_mut()
        .query_filtered::<(Entity, &OwnedByCharacter, &RagdollBodyPart), ()>()
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
    eprintln!("DEBUG enemy={:?} hit_body={:?} dying={}", enemy, hit_body, app.world().entity(enemy).contains::<cathedral::enemies::Dying>());
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
