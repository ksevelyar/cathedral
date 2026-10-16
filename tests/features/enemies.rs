use avian3d::prelude::*;
use bevy::prelude::*;

use super::support;

fn get_current_game_state(app: &mut App) -> GameState {
    app.world().resource::<State<GameState>>().get().clone()
}
use bevy::world_serialization::WorldAssetRoot;
use cathedral::app::build_test_app;
use cathedral::enemies::{Dying, Enemy, EnemyKind};
use cathedral::health::Health;
use cathedral::maps::{Map, MapBuilder};
use cathedral::player::Player;
use cathedral::player::{PlayerCamera, PlayerDead};
use cathedral::ragdoll::{OwnedByCharacter, RagdollBodyPart};
use cathedral::shooting::Weapon;
use cathedral::state::GameState;
use std::time::Duration;

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

fn build_map(player_position: Vec3, kind: EnemyKind) -> Map {
    let floor_size = Vec3::new(60.0, 0.1, 60.0);
    MapBuilder::new("enemies")
        .with_player_position(player_position, player_position + Vec3::NEG_Z)
        .with_floor(floor_size)
        .with_enemy(Vec3::new(0.0, 0.5, 0.0), kind)
        .build()
}

fn find_enemy_by_name(app: &mut App, kind: EnemyKind) -> Entity {
    let enemy_name = match kind {
        EnemyKind::Fighter => "Fighter 1",
        EnemyKind::Gunner => "Gunner 1",
    };
    app.world_mut()
        .query_filtered::<(Entity, &Name), With<Enemy>>()
        .iter(app.world())
        .find(|(_, name)| name.as_str() == enemy_name)
        .map(|(entity, _)| entity)
        .unwrap_or_else(|| panic!("enemy {enemy_name} should spawn with the map"))
}

fn simulate_enemy_near_player(kind: EnemyKind, walking_updates: usize, player_position: Vec3) -> (App, Entity) {
    let fixed_timestep = Duration::from_secs_f64(support::FIXED_TIMESTEP_SECONDS);
    let mut app = build_test_app(build_map(player_position, kind));
    app.insert_resource(Time::<Fixed>::from_duration(fixed_timestep));
    app.world_mut()
        .resource_mut::<NextState<GameState>>()
        .set(GameState::Paused);
    app.update();
    let enemy = find_enemy_by_name(&mut app, kind);
    wait_for_inert_ragdoll(&mut app, enemy);
    app.world_mut()
        .resource_mut::<NextState<GameState>>()
        .set(GameState::Playing);
    run_updates(&mut app, walking_updates);
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

fn wait_until_dying(app: &mut App, enemy: Entity) {
    let death_wait_updates = 64;
    for _ in 0..death_wait_updates {
        app.update();
        if app.world().entity(enemy).contains::<Dying>() {
            return;
        }
    }
    panic!("shot enemy {enemy:?} should become dying within {death_wait_updates} updates");
}

fn assert_animator_stays_stopped(app: &mut App, animator: Entity) {
    let settle_updates = 512;
    run_updates(app, settle_updates);
    let playing_count = count_playing_animations(app, animator);
    assert_eq!(playing_count, 0, "killed enemy animator should not resume animations");
}

fn assert_death(app: &mut App, enemy: Entity) {
    run_updates(app, 32);
    assert!(
        app.world().entity(enemy).contains::<Dying>(),
        "killed enemy should carry the dying marker"
    );
    let animator = find_enemy_animator(app, enemy);
    assert_animator_stays_stopped(app, animator);
    let weapon = find_dropped_weapon(app, enemy);
    assert!(
        app.world()
            .get::<RigidBody>(weapon)
            .is_some_and(|body| *body == RigidBody::Dynamic),
        "dropped weapon should become a dynamic rigid body"
    );
    assert!(
        !app.world().get::<ChildOf>(weapon).is_some(),
        "dropped weapon should be detached from the hand"
    );
}

fn killed_enemy_drops_weapon(kind: EnemyKind) {
    let (mut app, enemy) = simulate_enemy_near_player(kind, 160, Vec3::new(5.0, 0.5, 0.0));
    let hit_body = find_ragdoll_body(&mut app, enemy, RagdollBodyPart::Head);

    support::shoot_entity(&mut app, hit_body);
    assert_death(&mut app, enemy);
}

fn killed_enemy_stops_attacking_animation(kind: EnemyKind) {
    let (mut app, enemy) = simulate_enemy_near_player(kind, 160, Vec3::new(5.0, 0.5, 0.0));
    let animator = find_enemy_animator(&mut app, enemy);
    update_until_animator_plays(&mut app, animator, "alive");
    let hit_body = find_ragdoll_body(&mut app, enemy, RagdollBodyPart::Head);

    support::shoot_entity(&mut app, hit_body);
    assert_death(&mut app, enemy);
}

#[test]
fn player_damages_fighter_on_first_hit_and_kills_on_second() {
    let (mut app, enemy) = simulate_enemy_near_player(EnemyKind::Fighter, 160, Vec3::new(5.0, 0.5, 0.0));
    let hit_body = find_ragdoll_body(&mut app, enemy, RagdollBodyPart::Head);
    let revolver_damage = 40.0;
    let fighter_starting_health = 40.0;
    let one_hit_survivable_health = fighter_starting_health + revolver_damage;
    app.world_mut()
        .entity_mut(enemy)
        .insert(Health::new(one_hit_survivable_health));

    support::shoot_entity(&mut app, hit_body);
    run_updates(&mut app, 32);
    let health_after_first_hit = app
        .world()
        .get::<Health>(enemy)
        .expect("shot enemy should keep its health")
        .get_current();
    assert_eq!(health_after_first_hit, one_hit_survivable_health - revolver_damage);
    assert!(!app.world().entity(enemy).contains::<Dying>());

    support::shoot_entity(&mut app, hit_body);
    assert_death(&mut app, enemy);
}

#[test]
fn fighter_damages_player_on_first_hit_and_kills_on_second() {
    let (mut app, _) = simulate_enemy_near_player(EnemyKind::Fighter, 16, Vec3::new(1.0, 0.5, 0.0));
    let player_entity = app
        .world_mut()
        .query_filtered::<Entity, With<Player>>()
        .single(app.world())
        .expect("one player should spawn");
    let katana_damage = 20.0;
    let survivable_health = katana_damage + 10.0;
    app.world_mut()
        .entity_mut(player_entity)
        .insert(Health::new(survivable_health));

    run_until_first_melee_hit(&mut app, survivable_health);
    let health_after_first_hit = get_player_health(&mut app);
    assert!(health_after_first_hit > 0.0);
    assert_eq!(health_after_first_hit, survivable_health - katana_damage);
    assert!(app.world().get::<PlayerDead>(player_entity).is_none());

    run_until_player_dead(&mut app);
    run_updates(&mut app, 8);
    assert_eq!(get_player_health(&mut app), 0.0);
    assert_eq!(get_current_game_state(&mut app), GameState::GameOver);
}

fn run_until_first_melee_hit(app: &mut App, starting_health: f32) {
    let melee_hit_updates = 10_000;
    for _ in 0..melee_hit_updates {
        app.update();
        let current_health = get_player_health(app);
        if current_health < starting_health {
            return;
        }
    }
    panic!("fighter should damage the player within {melee_hit_updates} updates");
}

#[test]
fn killed_fighter_drops_weapon() {
    killed_enemy_drops_weapon(EnemyKind::Fighter);
}

#[test]
fn killed_gunner_drops_weapon() {
    killed_enemy_drops_weapon(EnemyKind::Gunner);
}

#[test]
fn killed_fighter_stops_attacking_animation() {
    killed_enemy_stops_attacking_animation(EnemyKind::Fighter);
}

#[test]
fn killed_gunner_stops_attacking_animation() {
    killed_enemy_stops_attacking_animation(EnemyKind::Gunner);
}

#[test]
fn fighter_killed_while_running_keeps_its_momentum() {
    let fighter_run_speed = 3.0;
    let minimum_kept_run_speed = fighter_run_speed / 2.0;
    let far_player_position = Vec3::new(12.0, 0.5, 0.0);
    let (mut app, enemy) = simulate_enemy_near_player(EnemyKind::Fighter, 16, far_player_position);
    let torso = find_ragdoll_body(&mut app, enemy, RagdollBodyPart::Torso);
    let hit_body = find_ragdoll_body(&mut app, enemy, RagdollBodyPart::Head);

    let run_direction = (far_player_position - Vec3::new(0.0, 0.5, 0.0)).normalize();
    let running_velocity = app
        .world()
        .get::<LinearVelocity>(enemy)
        .expect("running fighter should have linear velocity")
        .0;
    let velocity_tolerance = 0.01;
    let forward_run_speed = running_velocity.dot(run_direction);
    assert!(
        forward_run_speed >= fighter_run_speed - velocity_tolerance,
        "fighter should run at {fighter_run_speed} toward the player right after spawning, got {forward_run_speed}"
    );
    support::shoot_entity(&mut app, hit_body);
    let torso_velocity = wait_for_ragdoll_activation_velocity(&mut app, torso);
    let forward_run_momentum = torso_velocity.dot(run_direction);
    assert!(
        forward_run_momentum >= minimum_kept_run_speed,
        "running fighter ragdoll torso should keep at least {minimum_kept_run_speed} of the run momentum of {fighter_run_speed}, got forward component {forward_run_momentum:.3} from velocity {torso_velocity}"
    );
}

fn wait_for_ragdoll_activation_velocity(app: &mut App, body: Entity) -> Vec3 {
    let settle_updates = 64;
    for _ in 0..settle_updates {
        app.update();
        let is_active = app.world().get::<RigidBodyDisabled>(body).is_none();
        if is_active && let Some(velocity) = app.world().get::<LinearVelocity>(body) {
            return velocity.0;
        }
    }
    panic!("ragdoll body {body:?} should activate with a velocity");
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
    let (mut app, enemy) = simulate_enemy_near_player(EnemyKind::Fighter, 160, Vec3::new(5.0, 0.5, 0.0));
    let hit_body = find_ragdoll_body(&mut app, enemy, RagdollBodyPart::Head);

    support::shoot_entity(&mut app, hit_body);
    wait_until_dying(&mut app, enemy);
    support::assert_received_kick(&app, hit_body);
    let settling_duration = 10.0;
    support::assert_settles(&mut app, hit_body, settling_duration);
}

#[test]
fn head_shot_kills_gunner() {
    let (mut app, enemy) = simulate_enemy_near_player(EnemyKind::Gunner, 160, Vec3::new(5.0, 0.5, 0.0));
    let hit_body = find_ragdoll_body(&mut app, enemy, RagdollBodyPart::Head);

    support::shoot_entity(&mut app, hit_body);
    wait_until_dying(&mut app, enemy);
    support::assert_received_kick(&app, hit_body);
    let settling_duration = 10.0;
    support::assert_settles(&mut app, hit_body, settling_duration);
}

#[test]
fn gunner_loads_one_hairstyle() {
    let (mut app, gunner) = simulate_enemy_near_player(EnemyKind::Gunner, 0, Vec3::new(5.0, 0.5, 0.0));
    let maximum_asset_load_updates = 10_000;
    let required_stable_updates = 64;
    let mut stable_updates = 0;
    for _ in 0..maximum_asset_load_updates {
        app.update();
        let hair_root_count = app
            .world()
            .get::<Children>(gunner)
            .map(|children| {
                children
                    .iter()
                    .filter(|child| app.world().get::<WorldAssetRoot>(*child).is_some())
                    .count()
            })
            .unwrap_or_default();
        stable_updates = if hair_root_count == 1 { stable_updates + 1 } else { 0 };
        if stable_updates == required_stable_updates {
            return;
        }
        std::thread::yield_now();
    }
    panic!("gunner should load exactly one hairstyle");
}

#[test]
fn hand_shot_kills_fighter() {
    let (mut app, enemy) = simulate_enemy_near_player(EnemyKind::Fighter, 160, Vec3::new(5.0, 0.5, 0.0));
    let hit_body = find_ragdoll_body(&mut app, enemy, RagdollBodyPart::RightHand);

    support::shoot_entity(&mut app, hit_body);
    wait_until_dying(&mut app, enemy);
    support::assert_received_kick(&app, hit_body);
    let settling_duration = 10.0;
    support::assert_settles(&mut app, hit_body, settling_duration);
}

#[test]
fn hand_shot_kills_gunner() {
    let (mut app, enemy) = simulate_enemy_near_player(EnemyKind::Gunner, 160, Vec3::new(5.0, 0.5, 0.0));
    let hit_body = find_ragdoll_body(&mut app, enemy, RagdollBodyPart::RightHand);

    support::shoot_entity(&mut app, hit_body);
    wait_until_dying(&mut app, enemy);
    support::assert_received_kick(&app, hit_body);
    let settling_duration = 10.0;
    support::assert_settles(&mut app, hit_body, settling_duration);
}

fn find_player_ragdoll_body(app: &mut App, part: RagdollBodyPart) -> Entity {
    let spawn_wait_updates = 600;
    for _ in 0..spawn_wait_updates {
        app.update();
        let player_entity = app
            .world_mut()
            .query_filtered::<Entity, With<Player>>()
            .single(app.world())
            .expect("one player should spawn");
        let player_children = app
            .world()
            .get::<Children>(player_entity)
            .expect("player should have children")
            .iter()
            .collect::<Vec<_>>();
        if let Some(body) = app
            .world_mut()
            .query_filtered::<(Entity, &OwnedByCharacter, &RagdollBodyPart), ()>()
            .iter(app.world())
            .find(|(_, owner, body_part)| player_children.contains(&owner.0) && **body_part == part)
            .map(|(entity, _, _)| entity)
        {
            return body;
        }
    }
    panic!("player ragdoll body for {part:?} should exist within {spawn_wait_updates} updates");
}

fn get_player_health(app: &mut App) -> f32 {
    app.world_mut()
        .query_filtered::<&Health, With<Player>>()
        .single(app.world())
        .expect("player should carry health")
        .get_current()
}

fn get_camera_height(app: &mut App) -> f32 {
    app.world_mut()
        .query_filtered::<&GlobalTransform, With<PlayerCamera>>()
        .single(app.world())
        .expect("player camera should exist")
        .translation()
        .y
}

fn get_head_body_height(app: &mut App, head_body: Entity) -> f32 {
    app.world().get::<Position>(head_body).expect("head body position").0.y
}

fn run_until_player_dead(app: &mut App) {
    let death_updates = 10_000;
    for _ in 0..death_updates {
        app.update();
        if app
            .world_mut()
            .query_filtered::<(), With<PlayerDead>>()
            .single(app.world())
            .is_ok()
        {
            return;
        }
    }
    panic!("player should die within {death_updates} updates");
}

fn assert_camera_follows_fallen_head(app: &mut App, head_body: Entity) {
    let settle_updates = 256;
    for _ in 0..settle_updates {
        if get_camera_height(app) < 1.8 {
            break;
        }
        app.update();
    }
    let standing_eye_height = 1.8;
    let camera_height = get_camera_height(app);
    let head_body_height = get_head_body_height(app, head_body);
    let camera_head_height_difference = (camera_height - head_body_height).abs();
    assert!(
        camera_height < standing_eye_height,
        "camera should fall below the standing eye height {standing_eye_height}, got {camera_height}"
    );
    assert!(
        camera_head_height_difference <= 0.5,
        "camera should stay near the fallen head body, got camera {camera_height} vs head {head_body_height}"
    );
}

#[test]
fn gunner_shot_kills_player_and_camera_falls_to_head() {
    let (mut app, _) = simulate_enemy_near_player(EnemyKind::Gunner, 16, Vec3::new(6.0, 0.5, 0.0));
    let head_body = find_player_ragdoll_body(&mut app, RagdollBodyPart::Head);

    run_until_player_dead(&mut app);

    assert!(
        get_player_health(&mut app) <= 0.0,
        "dead player should have no health left"
    );
    assert!(
        !app.world().get::<RigidBodyDisabled>(head_body).is_some(),
        "player head ragdoll body should activate on death"
    );
    assert_camera_follows_fallen_head(&mut app, head_body);
    assert_eq!(get_current_game_state(&mut app), GameState::GameOver);
}

#[test]
fn fighter_melee_kills_player_and_camera_falls_to_head() {
    let (mut app, _) = simulate_enemy_near_player(EnemyKind::Fighter, 16, Vec3::new(1.0, 0.5, 0.0));
    let initial_health = get_player_health(&mut app);
    let head_body = find_player_ragdoll_body(&mut app, RagdollBodyPart::Head);

    run_until_player_dead(&mut app);

    let health_after_death = get_player_health(&mut app);
    assert!(
        health_after_death < initial_health,
        "fighter melee should have damaged the player before death, got {health_after_death} from {initial_health}"
    );
    assert!(
        !app.world().get::<RigidBodyDisabled>(head_body).is_some(),
        "player head ragdoll body should activate on death"
    );
    assert_camera_follows_fallen_head(&mut app, head_body);
    assert_eq!(get_current_game_state(&mut app), GameState::GameOver);
}
