use avian3d::prelude::*;
use bevy::animation::AnimationPlugin;
use bevy::asset::AssetPlugin;
use bevy::audio::AudioPlugin;
use bevy::ecs::system::RunSystemOnce;
use bevy::gltf::GltfPlugin;
use bevy::input::InputPlugin;
use bevy::mesh::MeshPlugin;
use bevy::prelude::*;
use bevy::time::TimeUpdateStrategy;
use bevy::world_serialization::WorldSerializationPlugin;
use cathedral::enemies::{Dying, EnemiesPlugin, Enemy};
use cathedral::maps::MapsPlugin;
use cathedral::player::{Player, PlayerPlugin};
use cathedral::ragdoll::{OwnedByEnemy, RagdollBodyPart, RagdollPlugin};
use cathedral::shooting::ShootingPlugin;
use cathedral::state::GameStatePlugin;
use std::time::Duration;

const FIXED_TIMESTEP_SECONDS: f64 = 1.0 / 64.0;
const ADVANCE_ATTEMPTS: usize = 600;
const MAP01_ENEMY_COUNT: usize = 3;
const MAP02_ENEMY_COUNT: usize = 4;
const BODIES_PER_ENEMY: usize = 14;
const AIM_DISTANCE: f32 = 2.0;
const SETTLE_FRAMES: usize = 4;

#[derive(Component)]
struct SpawnedEnemy;

fn create_map_test_app() -> App {
    let fixed_timestep = Duration::from_secs_f64(FIXED_TIMESTEP_SECONDS);
    let mut app = App::new();
    app.add_plugins((
        MinimalPlugins,
        TransformPlugin,
        AssetPlugin::default(),
        InputPlugin,
        AudioPlugin::default(),
        WorldSerializationPlugin,
        MeshPlugin,
        AnimationPlugin,
        GltfPlugin::default(),
        PhysicsPlugins::default(),
    ));
    app.add_plugins((
        bevy::state::app::StatesPlugin,
        GameStatePlugin,
        MapsPlugin,
        PlayerPlugin,
        EnemiesPlugin,
        RagdollPlugin,
        ShootingPlugin,
    ))
    .init_asset::<StandardMaterial>()
    .init_asset::<Image>()
    .insert_resource(bevy::prelude::GizmoConfigStore::default())
    .add_systems(Startup, cathedral::player::setup_player)
    .insert_resource(Time::<Fixed>::from_duration(fixed_timestep))
    .insert_resource(TimeUpdateStrategy::ManualDuration(fixed_timestep))
    .add_systems(Update, tag_spawned_enemies);
    app.finish();
    app.cleanup();
    app
}

fn tag_spawned_enemies(mut commands: Commands, freshly_spawned: Query<Entity, Added<Enemy>>) {
    for entity in &freshly_spawned {
        commands.entity(entity).insert(SpawnedEnemy);
    }
}

fn enemies_of(world: &mut World) -> Vec<Entity> {
    world
        .query_filtered::<Entity, (With<Enemy>, With<SpawnedEnemy>)>()
        .iter(world)
        .collect()
}

fn wait_for_map01_enemies(app: &mut App) -> Vec<Entity> {
    for _ in 0..ADVANCE_ATTEMPTS {
        app.update();
        let enemies = enemies_of(app.world_mut());
        if enemies.len() == MAP01_ENEMY_COUNT && all_bodies_spawned(app.world_mut(), &enemies) {
            return enemies;
        }
        std::thread::yield_now();
    }
    panic!("map01 enemies did not spawn with {BODIES_PER_ENEMY} ragdoll bodies each");
}

fn all_bodies_spawned(world: &mut World, enemies: &[Entity]) -> bool {
    let mut bodies = world.query::<(&OwnedByEnemy, &RagdollBodyPart)>();
    enemies
        .iter()
        .all(|enemy| bodies.iter(world).filter(|(owner, _)| owner.0 == *enemy).count() == BODIES_PER_ENEMY)
}

fn head_body_position(world: &mut World, enemy: Entity) -> Vec3 {
    world
        .query::<(&Position, &RagdollBodyPart, &OwnedByEnemy)>()
        .iter(world)
        .find(|(_, part, owner)| **part == RagdollBodyPart::Head && owner.0 == enemy)
        .map(|(position, ..)| position.0)
        .unwrap_or_else(|| panic!("enemy {enemy:?} should have a head body"))
}

fn aim_player_at_head_of(app: &mut App, enemy: Entity) {
    let head = head_body_position(app.world_mut(), enemy);
    let camera_transform = Transform::from_translation(head + Vec3::Z * AIM_DISTANCE).looking_at(head, Vec3::Y);
    let player = app
        .world_mut()
        .query_filtered::<Entity, With<Player>>()
        .single(app.world_mut())
        .expect("one player should spawn");
    app.world_mut().entity_mut(player).insert((
        Transform::from_translation(camera_transform.translation).with_rotation(camera_transform.rotation),
        LinearVelocity::default(),
    ));
}

fn fire(app: &mut App) {
    app.world_mut()
        .resource_mut::<ButtonInput<MouseButton>>()
        .press(MouseButton::Left);
    app.world_mut()
        .run_system_once(cathedral::shooting::shoot)
        .expect("shoot should run");
    app.update();
    app.world_mut()
        .resource_mut::<ButtonInput<MouseButton>>()
        .release(MouseButton::Left);
}

fn is_dying(world: &mut World, enemy: Entity) -> bool {
    world.entity(enemy).contains::<Dying>()
}

fn owned_bodies(app: &mut App, enemy: Entity) -> Vec<Entity> {
    app.world_mut()
        .query::<(Entity, &OwnedByEnemy)>()
        .iter(app.world_mut())
        .filter(|(_, owner)| owner.0 == enemy)
        .map(|(entity, _)| entity)
        .collect()
}

fn wait_for_dying(app: &mut App, enemy: Entity) {
    for _ in 0..ADVANCE_ATTEMPTS {
        if app.world().get_entity(enemy).is_err() {
            return;
        }
        aim_player_at_head_of(app, enemy);
        fire(app);
        if app.world().get_entity(enemy).is_err() {
            return;
        }
        eprintln!(
            "MARK fired {enemy:?} dying={} bodies={:?}",
            is_dying(app.world_mut(), enemy),
            owned_bodies(app, enemy)
        );
        if is_dying(app.world_mut(), enemy) {
            return;
        }
    }
    panic!("enemy {enemy:?} did not die from headshots");
}

fn doomed_entities_of(app: &mut App, enemy: Entity) -> Vec<Entity> {
    if app.world().get_entity(enemy).is_err() {
        return Vec::new();
    }
    let mut owned = app.world_mut().query_filtered::<(Entity, &OwnedByEnemy), ()>();
    let mut doomed = owned
        .iter(app.world_mut())
        .filter(|(_, owner)| owner.0 == enemy)
        .map(|(entity, _)| entity)
        .collect::<Vec<_>>();
    doomed.push(enemy);
    doomed
}

fn run_until_map_advances(app: &mut App) {
    for _ in 0..ADVANCE_ATTEMPTS {
        app.update();
        if enemies_of(app.world_mut()).len() == MAP02_ENEMY_COUNT {
            for _ in 0..SETTLE_FRAMES {
                app.update();
            }
            return;
        }
    }
    panic!("map did not advance to map02");
}

#[test]
fn killing_every_enemy_advances_the_map_and_the_map_takes_everything_with_it() {
    let mut app = create_map_test_app();
    let first_generation = wait_for_map01_enemies(&mut app);

    let mut doomed = Vec::new();
    for enemy in &first_generation {
        wait_for_dying(&mut app, *enemy);
        doomed.extend(doomed_entities_of(&mut app, *enemy));
    }

    run_until_map_advances(&mut app);

    assert_eq!(enemies_of(app.world_mut()).len(), MAP02_ENEMY_COUNT);
    for entity in &doomed {
        if let Ok(entity_ref) = app.world().get_entity(*entity) {
            let owner = entity_ref.get::<OwnedByEnemy>().map(|owner| owner.0);
            let name = entity_ref.get::<Name>().map(|name| name.as_str());
            panic!("entity {entity:?} outlived its enemy: owner={owner:?} name={name:?}");
        }
    }
}
