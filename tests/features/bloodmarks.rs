use avian3d::prelude::*;
use bevy::mesh::{Mesh, Mesh3d, VertexAttributeValues};
use bevy::prelude::*;
use std::time::Duration;
use terminal::app::build_test_app;
use terminal::enemies::{Enemy, EnemyBehavior, EnemyKind};
use terminal::maps::{Map, MapBuilder};
use terminal::player::{Player, PlayerCamera};
use terminal::ragdoll::{OwnedByCharacter, RagdollBodyPart};
use terminal::shooting::{RangedWeapon, Weapon, WeaponFired};
use terminal::state::GameState;

use super::support;

fn build_map(player_position: Vec3) -> Map {
    let floor_size = Vec3::new(60.0, 0.1, 60.0);
    MapBuilder::new("bloodmarks")
        .with_player_position(player_position, player_position + Vec3::NEG_Z)
        .with_floor(floor_size)
        .with_enemy(
            vec![Vec3::new(0.0, 0.05, 0.0)],
            Vec3::new(0.0, 0.05, 0.0) + Vec3::NEG_Z,
            EnemyKind::Gunner,
            EnemyBehavior::Patrol,
        )
        .build()
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
            .filter(|(_, owner, _)| owner.get_owner() == enemy)
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

fn run_updates(app: &mut App, update_count: usize) {
    for _ in 0..update_count {
        app.update();
    }
}

fn simulate_gunner() -> (App, Entity) {
    let player_position = Vec3::new(5.0, 0.05, 0.0);
    let fixed_timestep = Duration::from_secs_f64(support::FIXED_TIMESTEP_SECONDS);
    let mut app = build_test_app(build_map(player_position));
    app.insert_resource(Time::<Fixed>::from_duration(fixed_timestep));
    app.world_mut()
        .resource_mut::<NextState<GameState>>()
        .set(GameState::Paused);
    app.update();
    let enemy = find_gunner_by_name(&mut app);
    wait_for_inert_ragdoll(&mut app, enemy);
    app.world_mut()
        .resource_mut::<NextState<GameState>>()
        .set(GameState::Playing);
    run_updates(&mut app, 160);
    (app, enemy)
}

fn find_gunner_by_name(app: &mut App) -> Entity {
    app.world_mut()
        .query_filtered::<(Entity, &Name), With<Enemy>>()
        .iter(app.world())
        .find(|(_, name)| name.as_str() == "Gunner 1")
        .map_or_else(|| panic!("gunner should spawn with the map"), |(entity, _)| entity)
}

fn find_ragdoll_body(app: &mut App, enemy: Entity, part: RagdollBodyPart) -> Entity {
    app.world_mut()
        .query::<(Entity, &OwnedByCharacter, &RagdollBodyPart)>()
        .iter(app.world())
        .find(|(_, owner, body_part)| owner.get_owner() == enemy && **body_part == part)
        .map_or_else(|| panic!("ragdoll should have {part:?}"), |(entity, _, _)| entity)
}

fn queue_weapon_at_entity(app: &mut App, weapon: RangedWeapon, target: Entity) {
    let target_position = app.world().get::<Position>(target).expect("target body position").0;
    let player = app
        .world_mut()
        .query_filtered::<Entity, With<Player>>()
        .single(app.world())
        .expect("one player should spawn");
    let origin = app
        .world_mut()
        .query_filtered::<&GlobalTransform, With<PlayerCamera>>()
        .single(app.world())
        .expect("one player camera should spawn")
        .translation();
    let direction = Dir3::new(target_position - origin).expect("target should not share the player position");
    app.world_mut()
        .resource_mut::<Messages<WeaponFired>>()
        .write(WeaponFired {
            shooter_entity: Some(player),
            weapon_entity: Entity::PLACEHOLDER,
            weapon: Weapon::Ranged(weapon),
            damage_coefficient: 0.0,
            origin,
            direction,
        });
}

fn fire_weapon_at_entity(app: &mut App, weapon: RangedWeapon, target: Entity) {
    queue_weapon_at_entity(app, weapon, target);
    app.update();
    app.update();
}

fn find_blood_marks(app: &mut App) -> Vec<Entity> {
    app.world_mut()
        .query_filtered::<Entity, With<Name>>()
        .iter(app.world())
        .filter(|entity| {
            app.world()
                .get::<Name>(*entity)
                .is_some_and(|name| name.as_str() == "Blood mark")
        })
        .collect()
}

fn shoot_gunner_and_find_blood_mark(app: &mut App, enemy: Entity) -> Entity {
    let hit_body = find_ragdoll_body(app, enemy, RagdollBodyPart::Head);
    support::shoot_entity(app, hit_body);
    run_updates(app, 2);
    let blood_marks = find_blood_marks(app);
    assert_eq!(
        blood_marks.len(),
        1,
        "a bullet hit should create one blood mark for {enemy:?}"
    );
    blood_marks[0]
}

#[test]
fn player_bullet_creates_blood_mark() {
    let (mut app, enemy) = simulate_gunner();

    shoot_gunner_and_find_blood_mark(&mut app, enemy);
}

#[test]
fn blood_mark_uses_a_circular_surface_patch() {
    let (mut app, enemy) = simulate_gunner();

    let blood_mark = shoot_gunner_and_find_blood_mark(&mut app, enemy);
    let mesh_handle = app
        .world()
        .get::<Mesh3d>(blood_mark)
        .expect("blood mark should have a mesh");
    let mesh_assets = app.world().resource::<Assets<Mesh>>();
    let mesh = mesh_assets.get(&mesh_handle.0).expect("blood mark mesh should exist");
    let minimum_circle_vertices = 8;
    assert!(
        mesh.count_vertices() >= minimum_circle_vertices,
        "a circular blood mark needs at least {minimum_circle_vertices} vertices, got {}",
        mesh.count_vertices()
    );
}

#[test]
fn twelve_gauge_slug_mark_is_larger_than_nine_millimeter_mark() {
    let (mut app, enemy) = simulate_gunner();
    let hit_body = find_ragdoll_body(&mut app, enemy, RagdollBodyPart::Head);

    fire_weapon_at_entity(&mut app, RangedWeapon::PISTOL, hit_body);
    let nine_millimeter_mark = find_blood_marks(&mut app).pop().expect("9 mm hit should create a mark");
    fire_weapon_at_entity(&mut app, RangedWeapon::SHOTGUN_SHORT_STOCK, hit_body);
    let twelve_gauge_mark = find_blood_marks(&mut app)
        .into_iter()
        .find(|blood_mark| *blood_mark != nine_millimeter_mark)
        .expect("12-gauge hit should create a second mark");
    let mesh_assets = app.world().resource::<Assets<Mesh>>();
    let nine_millimeter_mesh_handle = app.world().get::<Mesh3d>(nine_millimeter_mark).expect("9 mm mark mesh");
    let twelve_gauge_mesh_handle = app
        .world()
        .get::<Mesh3d>(twelve_gauge_mark)
        .expect("12-gauge mark mesh");
    let nine_millimeter_mesh = mesh_assets
        .get(&nine_millimeter_mesh_handle.0)
        .expect("9 mm mesh asset");
    let twelve_gauge_mesh = mesh_assets
        .get(&twelve_gauge_mesh_handle.0)
        .expect("12-gauge mesh asset");

    let nine_millimeter_radius = get_largest_ultraviolet_distance(nine_millimeter_mesh);
    let twelve_gauge_radius = get_largest_ultraviolet_distance(twelve_gauge_mesh);
    assert!(
        twelve_gauge_radius < nine_millimeter_radius,
        "the same surface distance should occupy less UV space for the larger 12-gauge mark"
    );
}

fn get_largest_ultraviolet_distance(mesh: &Mesh) -> f32 {
    let Some(VertexAttributeValues::Float32x2(ultraviolet_coordinates)) = mesh.attribute(Mesh::ATTRIBUTE_UV_0) else {
        panic!("blood mark should have UV coordinates");
    };
    ultraviolet_coordinates
        .iter()
        .map(|coordinate| Vec2::from(*coordinate).distance(Vec2::splat(0.5)))
        .fold(0.0, f32::max)
}

#[test]
fn oldest_blood_mark_is_replaced_when_the_limit_is_exceeded() {
    let (mut app, enemy) = simulate_gunner();
    let hit_body = find_ragdoll_body(&mut app, enemy, RagdollBodyPart::Head);

    fire_weapon_at_entity(&mut app, RangedWeapon::SUBMACHINE_GUN_3, hit_body);
    let oldest_blood_mark = find_blood_marks(&mut app)
        .pop()
        .expect("first hit should create a blood mark");
    for _ in 0..24 {
        queue_weapon_at_entity(&mut app, RangedWeapon::SUBMACHINE_GUN_3, hit_body);
    }
    app.update();

    let remaining_blood_marks = find_blood_marks(&mut app);
    assert_eq!(
        remaining_blood_marks.len(),
        24,
        "an enemy should retain twenty-four blood marks"
    );
    assert!(
        app.world().get_entity(oldest_blood_mark).is_err(),
        "the oldest blood mark should be removed after the limit is exceeded"
    );
}
