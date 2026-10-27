use avian3d::prelude::{Position, RigidBodyDisabled};
use bevy::prelude::*;
use criterion::{BatchSize, Criterion, criterion_group, criterion_main};
use std::hint::black_box;
use std::time::Duration;
use terminal::app::build_test_app;
use terminal::enemies::{Enemy, EnemyBehavior, EnemyKind};
use terminal::maps::{Map, MapBuilder};
use terminal::ragdoll::{OwnedByCharacter, RagdollBodyPart};
use terminal::shooting::{RangedWeapon, Weapon, WeaponFired};
use terminal::state::GameState;

const FIXED_TIMESTEP_SECONDS: f64 = 1.0 / 64.0;

fn build_map() -> Map {
    let player_position = Vec3::new(5.0, 0.5, 0.0);
    let floor_size = Vec3::new(60.0, 0.1, 60.0);
    MapBuilder::new("bloodmarks benchmark")
        .with_player_position(player_position, player_position + Vec3::NEG_Z)
        .with_floor(floor_size)
        .with_enemy(
            vec![Vec3::new(0.0, 0.5, 0.0)],
            Vec3::new(0.0, 0.5, -1.0),
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

fn prepare_gunner() -> (App, Entity) {
    let fixed_timestep = Duration::from_secs_f64(FIXED_TIMESTEP_SECONDS);
    let mut app = build_test_app(build_map());
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

fn queue_submachine_gun_magazine(app: &mut App, target: Entity) {
    let target_position = app.world().get::<Position>(target).expect("target body position").0;
    let origin = target_position + Vec3::X * 5.0;
    let direction = Dir3::new(target_position - origin).expect("target should not share the shot origin");
    let magazine_capacity = 24;
    let mut weapon_fired_messages = app.world_mut().resource_mut::<Messages<WeaponFired>>();
    for _ in 0..magazine_capacity {
        weapon_fired_messages.write(WeaponFired {
            shooter_entity: None,
            weapon_entity: Entity::PLACEHOLDER,
            weapon: Weapon::Ranged(RangedWeapon::SUBMACHINE_GUN_3),
            damage_coefficient: 0.0,
            origin,
            direction,
        });
    }
}

fn count_blood_marks(app: &mut App) -> usize {
    app.world_mut()
        .query_filtered::<Entity, With<Name>>()
        .iter(app.world())
        .filter(|entity| {
            app.world()
                .get::<Name>(*entity)
                .is_some_and(|name| name.as_str() == "Blood mark")
        })
        .count()
}

fn benchmark_submachine_gun_magazine(criterion: &mut Criterion) {
    let mut benchmark_group = criterion.benchmark_group("bloodmarks");
    benchmark_group.bench_function("submachine_gun_magazine", |bencher| {
        bencher.iter_batched_ref(
            || {
                let (mut app, enemy) = prepare_gunner();
                let head = find_ragdoll_body(&mut app, enemy, RagdollBodyPart::Head);
                queue_submachine_gun_magazine(&mut app, head);
                app
            },
            |app| {
                app.update();
                let blood_mark_count = count_blood_marks(app);
                assert_eq!(
                    blood_mark_count, 24,
                    "one magazine should create twenty-four blood marks"
                );
                black_box(blood_mark_count);
            },
            BatchSize::SmallInput,
        );
    });
    benchmark_group.finish();
}

criterion_group!(bloodmarks, benchmark_submachine_gun_magazine);
criterion_main!(bloodmarks);
