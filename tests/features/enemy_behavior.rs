use bevy::prelude::*;
use bevy::time::TimeUpdateStrategy;
use cathedral::app::build_test_app;
use cathedral::enemies::{AnimationState, Enemy, EnemyActivity, EnemyBehavior, EnemyKind};
use cathedral::health::{CharacterDamaged, Health};
use cathedral::maps::{Map, MapBuilder};
use cathedral::player::Player;
use cathedral::shooting::{RangedWeapon, Weapon, WeaponFired};
use cathedral::state::GameState;
use std::time::Duration;

use super::support::FIXED_TIMESTEP_SECONDS;

const ENEMY_POSITION: Vec3 = Vec3::new(0.0, 0.05, 0.0);
const HIDDEN_PLAYER_POSITION: Vec3 = Vec3::new(8.0, 0.05, 8.0);
const MAXIMUM_UPDATES: usize = 2_000;

fn build_enemy_map(
    kind: EnemyKind,
    initial_behavior: EnemyBehavior,
    positions: Vec<Vec3>,
    enemy_look_at: Vec3,
    player_position: Vec3,
    wall: Option<(Vec3, Vec3)>,
) -> Map {
    let floor_size = Vec3::new(60.0, 0.1, 60.0);
    let mut map_builder = MapBuilder::new("enemy_behavior")
        .with_player_position(player_position, player_position + Vec3::NEG_Z)
        .with_floor(floor_size)
        .with_enemy(positions, enemy_look_at, kind, initial_behavior);
    if let Some((wall_position, wall_size)) = wall {
        map_builder = map_builder.with_wall(wall_position, wall_size);
    }
    map_builder.build()
}

fn create_enemy_app(kind: EnemyKind, positions: Vec<Vec3>, enemy_look_at: Vec3, player_position: Vec3) -> App {
    let fixed_timestep = Duration::from_secs_f64(FIXED_TIMESTEP_SECONDS);
    let mut app = build_test_app(build_enemy_map(
        kind,
        EnemyBehavior::Patrol,
        positions,
        enemy_look_at,
        player_position,
        None,
    ));
    app.insert_resource(Time::<Fixed>::from_duration(fixed_timestep));
    app.world_mut()
        .resource_mut::<NextState<GameState>>()
        .set(GameState::Paused);
    app.update();
    app.world_mut()
        .resource_mut::<NextState<GameState>>()
        .set(GameState::Playing);
    app.insert_resource(TimeUpdateStrategy::ManualDuration(fixed_timestep));
    app
}

fn find_enemy(app: &mut App) -> Entity {
    app.world_mut()
        .query_filtered::<Entity, With<Enemy>>()
        .single(app.world())
        .expect("one enemy should spawn")
}

fn find_player(app: &mut App) -> Entity {
    app.world_mut()
        .query_filtered::<Entity, With<Player>>()
        .single(app.world())
        .expect("one player should spawn")
}

fn read_behavior(app: &App, enemy: Entity) -> &EnemyBehavior {
    app.world().get::<EnemyBehavior>(enemy).expect("enemy behavior")
}

fn read_position(app: &App, entity: Entity) -> Vec3 {
    app.world()
        .get::<Transform>(entity)
        .expect("entity transform")
        .translation
}

fn read_animation_state(app: &App, enemy: Entity) -> AnimationState {
    app.world().get::<EnemyActivity>(enemy).expect("enemy activity").state
}

fn read_health(app: &App, entity: Entity) -> f32 {
    app.world().get::<Health>(entity).expect("entity health").get_current()
}

fn move_player(app: &mut App, player: Entity, position: Vec3) {
    app.world_mut()
        .entity_mut(player)
        .get_mut::<Transform>()
        .expect("player transform")
        .translation = position;
}

fn send_gunshot(app: &mut App, origin: Vec3, weapon: RangedWeapon) {
    app.world_mut()
        .resource_mut::<Messages<WeaponFired>>()
        .write(WeaponFired {
            shooter_entity: None,
            weapon_entity: Entity::PLACEHOLDER,
            weapon: Weapon::Ranged(weapon),
            damage_coefficient: 0.0,
            origin,
            direction: Dir3::NEG_Z,
        });
}

fn damage_enemy(app: &mut App, enemy: Entity) {
    app.world_mut()
        .resource_mut::<Messages<CharacterDamaged>>()
        .write(CharacterDamaged {
            character: enemy,
            amount: 0.0,
            attacker: None,
            body_part: None,
        });
}

fn update_until(app: &mut App, predicate: impl Fn(&App) -> bool, description: &str) {
    for _ in 0..MAXIMUM_UPDATES {
        app.update();
        if predicate(app) {
            return;
        }
    }
    panic!("{description} within {MAXIMUM_UPDATES} updates");
}

fn is_patrolling(app: &App, enemy: Entity) -> bool {
    matches!(read_behavior(app, enemy), EnemyBehavior::Patrol)
}

fn is_investigating(app: &App, enemy: Entity, position: Vec3) -> bool {
    matches!(read_behavior(app, enemy), EnemyBehavior::Investigating { position: investigation_position } if investigation_position.distance(position) < 0.01)
}

fn is_engaged(app: &App, enemy: Entity) -> bool {
    matches!(read_behavior(app, enemy), EnemyBehavior::Engaged)
}

#[test]
fn enemy_patrols_when_player_is_not_detected() {
    let destination = Vec3::new(0.0, 0.05, -2.0);
    let mut app = create_enemy_app(
        EnemyKind::Fighter,
        vec![ENEMY_POSITION, destination],
        ENEMY_POSITION + Vec3::NEG_Z,
        HIDDEN_PLAYER_POSITION,
    );
    let enemy = find_enemy(&mut app);
    update_until(
        &mut app,
        |app| read_position(app, enemy).distance(destination) < 0.3,
        "enemy should reach patrol destination",
    );
    assert!(is_patrolling(&app, enemy));
    assert_eq!(read_animation_state(&app, enemy), AnimationState::Moving);
}

#[test]
fn enemy_cycles_through_patrol_points() {
    let first_destination = Vec3::new(0.0, 0.05, -2.0);
    let second_destination = Vec3::new(2.0, 0.05, -2.0);
    let mut app = create_enemy_app(
        EnemyKind::Fighter,
        vec![ENEMY_POSITION, first_destination, second_destination],
        ENEMY_POSITION + Vec3::NEG_Z,
        HIDDEN_PLAYER_POSITION,
    );
    let enemy = find_enemy(&mut app);
    update_until(
        &mut app,
        |app| read_position(app, enemy).distance(first_destination) < 0.3,
        "enemy should reach first patrol point",
    );
    update_until(
        &mut app,
        |app| read_position(app, enemy).distance(second_destination) < 0.3,
        "enemy should reach second patrol point",
    );
    assert!(is_patrolling(&app, enemy));
}

#[test]
fn enemy_waits_between_patrol_points() {
    let destination = Vec3::new(0.0, 0.05, -1.0);
    let mut app = create_enemy_app(
        EnemyKind::Fighter,
        vec![ENEMY_POSITION, destination],
        ENEMY_POSITION + Vec3::NEG_Z,
        HIDDEN_PLAYER_POSITION,
    );
    let enemy = find_enemy(&mut app);
    update_until(
        &mut app,
        |app| read_position(app, enemy).distance(destination) < 0.3,
        "enemy should reach patrol point",
    );
    let arrival_position = read_position(&app, enemy);
    for _ in 0..128 {
        app.update();
    }
    assert!(
        read_position(&app, enemy).distance(arrival_position) < 0.05,
        "enemy should wait at patrol point"
    );
    update_until(
        &mut app,
        |app| read_position(app, enemy).distance(ENEMY_POSITION) < 0.3,
        "enemy should leave after waiting",
    );
}

#[test]
fn enemy_with_default_patrol_stays_at_spawn() {
    let mut app = create_enemy_app(
        EnemyKind::Fighter,
        vec![ENEMY_POSITION],
        ENEMY_POSITION + Vec3::NEG_Z,
        HIDDEN_PLAYER_POSITION,
    );
    let enemy = find_enemy(&mut app);
    for _ in 0..512 {
        app.update();
    }
    assert!(read_position(&app, enemy).distance(ENEMY_POSITION) < 0.01);
    assert!(is_patrolling(&app, enemy));
}

#[test]
fn enemy_returns_to_patrol_after_reaching_investigation_sound() {
    let patrol_destination = Vec3::new(0.0, 0.05, -2.0);
    let sound_position = Vec3::new(2.0, 0.05, 0.0);
    let mut app = create_enemy_app(
        EnemyKind::Fighter,
        vec![ENEMY_POSITION, patrol_destination],
        ENEMY_POSITION + Vec3::NEG_Z,
        HIDDEN_PLAYER_POSITION,
    );
    let enemy = find_enemy(&mut app);
    send_gunshot(&mut app, sound_position, RangedWeapon::PISTOL);
    update_until(
        &mut app,
        |app| is_investigating(app, enemy, sound_position),
        "enemy should investigate sound",
    );
    update_until(
        &mut app,
        |app| is_patrolling(app, enemy) && read_position(app, enemy).distance(sound_position) < 0.3,
        "enemy should return to patrol after investigation",
    );
    update_until(
        &mut app,
        |app| read_position(app, enemy).distance(patrol_destination) < 0.3,
        "enemy should resume patrol route",
    );
}

fn assert_detects_after(kind: EnemyKind, player_position: Vec3, expected_seconds: f32) {
    let mut app = create_enemy_app(kind, vec![ENEMY_POSITION], player_position, player_position);
    let enemy = find_enemy(&mut app);
    let fixed_timestep_seconds = FIXED_TIMESTEP_SECONDS as f32;
    let updates_before_detection = (expected_seconds / fixed_timestep_seconds) as usize - 2;
    for _ in 0..updates_before_detection {
        app.update();
        assert!(!is_engaged(&app, enemy));
    }
    update_until(&mut app, |app| is_engaged(app, enemy), "enemy should finish detection");
}

#[test]
fn fighter_detects_visible_player_after_two_seconds() {
    assert_detects_after(EnemyKind::Fighter, Vec3::new(0.0, 0.05, -4.0), 2.0);
}

#[test]
fn gunner_detects_visible_player_after_four_seconds() {
    assert_detects_after(EnemyKind::Gunner, Vec3::new(0.0, 0.05, -8.0), 4.0);
}

#[test]
fn fighter_does_not_detect_player_outside_visibility_cone() {
    let player_position = Vec3::new(4.0, 0.05, -1.0);
    let mut app = create_enemy_app(
        EnemyKind::Fighter,
        vec![ENEMY_POSITION],
        ENEMY_POSITION + Vec3::NEG_Z,
        player_position,
    );
    let enemy = find_enemy(&mut app);
    for _ in 0..256 {
        app.update();
    }
    assert!(is_patrolling(&app, enemy));
}

#[test]
fn fighter_does_not_detect_player_behind_visible_facing() {
    let player_position = Vec3::new(0.0, 0.05, 4.0);
    let mut app = create_enemy_app(
        EnemyKind::Fighter,
        vec![ENEMY_POSITION],
        ENEMY_POSITION + Vec3::NEG_Z,
        player_position,
    );
    let enemy = find_enemy(&mut app);
    for _ in 0..256 {
        app.update();
    }
    assert!(is_patrolling(&app, enemy));
}

#[test]
fn gunner_does_not_detect_player_outside_visibility_range() {
    let player_position = Vec3::new(0.0, 0.05, -13.0);
    let mut app = create_enemy_app(
        EnemyKind::Gunner,
        vec![ENEMY_POSITION],
        player_position,
        player_position,
    );
    let enemy = find_enemy(&mut app);
    for _ in 0..384 {
        app.update();
    }
    assert!(is_patrolling(&app, enemy));
}

#[test]
fn wall_blocks_enemy_visibility() {
    let player_position = Vec3::new(0.0, 0.05, -4.0);
    let wall_position = Vec3::new(0.0, 1.5, -2.0);
    let wall_size = Vec3::new(3.0, 3.0, 0.2);
    let mut app = build_test_app(build_enemy_map(
        EnemyKind::Fighter,
        EnemyBehavior::Patrol,
        vec![ENEMY_POSITION],
        player_position,
        player_position,
        Some((wall_position, wall_size)),
    ));
    app.insert_resource(Time::<Fixed>::from_duration(Duration::from_secs_f64(
        FIXED_TIMESTEP_SECONDS,
    )));
    app.world_mut()
        .resource_mut::<NextState<GameState>>()
        .set(GameState::Paused);
    app.update();
    app.world_mut()
        .resource_mut::<NextState<GameState>>()
        .set(GameState::Playing);
    let enemy = find_enemy(&mut app);
    for _ in 0..256 {
        app.update();
    }
    assert!(is_patrolling(&app, enemy));
}

#[test]
fn interrupted_visibility_resets_detection_progress() {
    let visible_position = Vec3::new(0.0, 0.05, -4.0);
    let mut app = create_enemy_app(
        EnemyKind::Fighter,
        vec![ENEMY_POSITION],
        visible_position,
        visible_position,
    );
    let enemy = find_enemy(&mut app);
    let player = find_player(&mut app);
    for _ in 0..96 {
        app.update();
    }
    assert!(is_patrolling(&app, enemy));
    move_player(&mut app, player, HIDDEN_PLAYER_POSITION);
    app.update();
    move_player(&mut app, player, visible_position);
    for _ in 0..96 {
        app.update();
    }
    assert!(is_patrolling(&app, enemy));
    update_until(
        &mut app,
        |app| is_engaged(app, enemy),
        "enemy should detect after uninterrupted visibility",
    );
}

#[test]
fn engaged_enemy_remains_engaged_after_losing_sight() {
    let visible_position = Vec3::new(0.0, 0.05, -4.0);
    let mut app = create_enemy_app(
        EnemyKind::Fighter,
        vec![ENEMY_POSITION],
        visible_position,
        visible_position,
    );
    let enemy = find_enemy(&mut app);
    let player = find_player(&mut app);
    update_until(&mut app, |app| is_engaged(app, enemy), "enemy should detect player");
    move_player(&mut app, player, HIDDEN_PLAYER_POSITION);
    for _ in 0..256 {
        app.update();
    }
    assert!(is_engaged(&app, enemy));
}

#[test]
fn audible_gunshot_starts_investigation() {
    let sound_position = Vec3::new(2.0, 0.05, 0.0);
    let mut app = create_enemy_app(
        EnemyKind::Fighter,
        vec![ENEMY_POSITION],
        ENEMY_POSITION + Vec3::NEG_Z,
        HIDDEN_PLAYER_POSITION,
    );
    let enemy = find_enemy(&mut app);
    send_gunshot(&mut app, sound_position, RangedWeapon::PISTOL);
    update_until(
        &mut app,
        |app| is_investigating(app, enemy, sound_position),
        "audible gunshot should start investigation",
    );
}

#[test]
fn inaudible_gunshot_does_not_interrupt_patrol() {
    let mut app = create_enemy_app(
        EnemyKind::Fighter,
        vec![ENEMY_POSITION],
        ENEMY_POSITION + Vec3::NEG_Z,
        HIDDEN_PLAYER_POSITION,
    );
    let enemy = find_enemy(&mut app);
    send_gunshot(&mut app, Vec3::new(6.0, 0.05, 0.0), RangedWeapon::PISTOL);
    app.update();
    assert!(is_patrolling(&app, enemy));
}

#[test]
fn louder_gunshot_replaces_current_investigation() {
    let quiet_position = Vec3::new(4.5, 0.05, 0.0);
    let loud_position = Vec3::new(-4.0, 0.05, 0.0);
    let mut app = create_enemy_app(
        EnemyKind::Fighter,
        vec![ENEMY_POSITION],
        ENEMY_POSITION + Vec3::NEG_Z,
        HIDDEN_PLAYER_POSITION,
    );
    let enemy = find_enemy(&mut app);
    send_gunshot(&mut app, quiet_position, RangedWeapon::PISTOL);
    update_until(
        &mut app,
        |app| is_investigating(app, enemy, quiet_position),
        "first gunshot should investigate",
    );
    send_gunshot(&mut app, loud_position, RangedWeapon::SHOTGUN_SHORT_STOCK);
    update_until(
        &mut app,
        |app| is_investigating(app, enemy, loud_position),
        "louder gunshot should replace investigation",
    );
}

#[test]
fn newer_equal_loudness_gunshot_replaces_current_investigation() {
    let first_position = Vec3::new(3.0, 0.05, 0.0);
    let second_position = Vec3::new(-3.0, 0.05, 0.0);
    let mut app = create_enemy_app(
        EnemyKind::Fighter,
        vec![ENEMY_POSITION],
        ENEMY_POSITION + Vec3::NEG_Z,
        HIDDEN_PLAYER_POSITION,
    );
    let enemy = find_enemy(&mut app);
    send_gunshot(&mut app, first_position, RangedWeapon::PISTOL);
    update_until(
        &mut app,
        |app| is_investigating(app, enemy, first_position),
        "first gunshot should investigate",
    );
    send_gunshot(&mut app, second_position, RangedWeapon::PISTOL);
    update_until(
        &mut app,
        |app| is_investigating(app, enemy, second_position),
        "newer equal gunshot should replace investigation",
    );
}

#[test]
fn weapon_loudness_scales_gunshot_audibility() {
    let sound_position = Vec3::new(8.0, 0.05, 0.0);
    let mut quiet_app = create_enemy_app(
        EnemyKind::Fighter,
        vec![ENEMY_POSITION],
        ENEMY_POSITION + Vec3::NEG_Z,
        HIDDEN_PLAYER_POSITION,
    );
    let quiet_enemy = find_enemy(&mut quiet_app);
    send_gunshot(&mut quiet_app, sound_position, RangedWeapon::PISTOL);
    quiet_app.update();
    assert!(is_patrolling(&quiet_app, quiet_enemy));
    let mut loud_app = create_enemy_app(
        EnemyKind::Fighter,
        vec![ENEMY_POSITION],
        ENEMY_POSITION + Vec3::NEG_Z,
        HIDDEN_PLAYER_POSITION,
    );
    let loud_enemy = find_enemy(&mut loud_app);
    send_gunshot(&mut loud_app, sound_position, RangedWeapon::SHOTGUN_SHORT_STOCK);
    update_until(
        &mut loud_app,
        |app| is_investigating(app, loud_enemy, sound_position),
        "louder weapon should be audible farther away",
    );
}

#[test]
fn damaging_enemy_immediately_engages_player() {
    let mut app = create_enemy_app(
        EnemyKind::Fighter,
        vec![ENEMY_POSITION],
        ENEMY_POSITION + Vec3::NEG_Z,
        HIDDEN_PLAYER_POSITION,
    );
    let enemy = find_enemy(&mut app);
    damage_enemy(&mut app, enemy);
    app.update();
    assert!(is_engaged(&app, enemy));
}

#[test]
fn engaged_gunner_turns_toward_player() {
    let player_position = Vec3::new(0.0, 0.05, 5.0);
    let mut app = create_enemy_app(
        EnemyKind::Gunner,
        vec![ENEMY_POSITION],
        ENEMY_POSITION + Vec3::NEG_Z,
        player_position,
    );
    let enemy = find_enemy(&mut app);
    damage_enemy(&mut app, enemy);
    update_until(
        &mut app,
        |app| read_animation_state(app, enemy) == AnimationState::Attacking,
        "gunner should enter the attacking animation",
    );
    let enemy_model_forward = app
        .world()
        .get::<Transform>(enemy)
        .expect("enemy transform")
        .back()
        .as_vec3();
    assert!(
        enemy_model_forward.dot(Vec3::Z) > 0.99,
        "engaged gunner's visible front should face the player"
    );
}

#[test]
fn unalerted_enemy_does_not_attack_player() {
    let player_position = Vec3::new(0.0, 0.05, -4.0);
    let mut app = create_enemy_app(
        EnemyKind::Fighter,
        vec![ENEMY_POSITION],
        ENEMY_POSITION + Vec3::NEG_Z,
        player_position,
    );
    let enemy = find_enemy(&mut app);
    let player = find_player(&mut app);
    let initial_health = read_health(&app, player);
    for _ in 0..64 {
        app.update();
    }
    assert!(is_patrolling(&app, enemy));
    assert_eq!(read_health(&app, player), initial_health);
    assert_ne!(read_animation_state(&app, enemy), AnimationState::Attacking);
}
