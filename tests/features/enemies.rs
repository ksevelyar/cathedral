use avian3d::prelude::*;
use bevy::mesh::skinning::SkinnedMesh;
use bevy::prelude::*;

use super::support;

fn get_current_game_state(app: &mut App) -> GameState {
    app.world().resource::<State<GameState>>().get().clone()
}
use bevy::world_serialization::WorldAssetRoot;
use std::collections::{HashMap, HashSet};
use std::time::Duration;
use terminal::app::build_test_app;
use terminal::enemies::{AnimationState, Dying, Enemy, EnemyActivity, EnemyBehavior, EnemyKind};
use terminal::health::Health;
use terminal::maps::{Map, MapBuilder};
use terminal::player::Player;
use terminal::player::{PlayerCamera, PlayerDead};
use terminal::ragdoll::{BoneMap, OwnedByCharacter, RagdollBodyPart, list_ragdoll_limbs};
use terminal::shooting::Weapon;
use terminal::state::GameState;

fn assert_enemy_ragdoll_is_inert(app: &mut App, enemy: Entity) {
    let minimum_ragdoll_bodies = 19;
    let enemy_ragdoll_bodies: Vec<_> = app
        .world_mut()
        .query::<(&RagdollBodyPart, &OwnedByCharacter, Has<RigidBodyDisabled>)>()
        .iter(app.world())
        .filter(|(_, owner, _)| owner.get_owner() == enemy)
        .map(|(_, _, disabled)| disabled)
        .collect();
    let body_count = enemy_ragdoll_bodies.len();
    assert!(
        body_count >= minimum_ragdoll_bodies,
        "alive enemy should have at least {minimum_ragdoll_bodies} ragdoll bodies, got {body_count}"
    );
    let inert_body_count = enemy_ragdoll_bodies.iter().filter(|disabled| **disabled).count();
    assert_eq!(inert_body_count, body_count);
    let mut spherical_joints = app
        .world_mut()
        .query_filtered::<&OwnedByCharacter, With<SphericalJoint>>();
    let mut revolute_joints = app
        .world_mut()
        .query_filtered::<&OwnedByCharacter, With<RevoluteJoint>>();
    let active_joint_count = spherical_joints
        .iter(app.world())
        .filter(|owner| owner.get_owner() == enemy)
        .count()
        + revolute_joints
            .iter(app.world())
            .filter(|owner| owner.get_owner() == enemy)
            .count();
    assert_eq!(active_joint_count, 0, "alive enemy should not have active joints");
}

fn build_map(player_position: Vec3, kind: EnemyKind, initial_behavior: EnemyBehavior) -> Map {
    let floor_size = Vec3::new(60.0, 0.1, 60.0);
    MapBuilder::new("enemies")
        .with_player_position(player_position, player_position + Vec3::NEG_Z)
        .with_floor(floor_size)
        .with_enemy(
            vec![Vec3::new(0.0, 0.05, 0.0)],
            Vec3::new(0.0, 0.05, 0.0) + Vec3::NEG_Z,
            kind,
            initial_behavior,
        )
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
        .map_or_else(
            || panic!("enemy {enemy_name} should spawn with the map"),
            |(entity, _)| entity,
        )
}

fn simulate_enemy_near_player(
    kind: EnemyKind,
    initial_behavior: EnemyBehavior,
    walking_updates: usize,
    player_position: Vec3,
) -> (App, Entity) {
    let fixed_timestep = Duration::from_secs_f64(support::FIXED_TIMESTEP_SECONDS);
    let mut app = build_test_app(build_map(player_position, kind, initial_behavior));
    app.insert_resource(Time::<Fixed>::from_duration(fixed_timestep));
    app.world_mut()
        .resource_mut::<NextState<GameState>>()
        .set(GameState::Paused);
    app.update();
    let enemy = find_enemy_by_name(&mut app, kind);
    assert_enemy_ragdoll_is_inert(&mut app, enemy);
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
    while let Some(parent) = app.world().get::<ChildOf>(current).map(ChildOf::parent) {
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
        .find(|(_, owner)| owner.get_owner() == enemy)
        .map_or_else(
            || panic!("enemy {enemy:?} should drop its weapon"),
            |(entity, _)| entity,
        )
}

fn count_playing_animations(app: &mut App, animator: Entity) -> usize {
    app.world()
        .get::<AnimationPlayer>(animator)
        .expect("animator should keep its animation player")
        .playing_animations()
        .count()
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
        app.world().get::<ChildOf>(weapon).is_none(),
        "dropped weapon should be detached from the hand"
    );
}

fn killed_enemy_drops_weapon(kind: EnemyKind) {
    let (mut app, enemy) = simulate_enemy_near_player(kind, EnemyBehavior::Patrol, 160, Vec3::new(5.0, 0.05, 0.0));
    let hit_body = find_ragdoll_body(&mut app, enemy, RagdollBodyPart::Head);

    support::shoot_entity(&mut app, hit_body);
    assert_death(&mut app, enemy);
}

fn killed_enemy_stops_attacking_animation(kind: EnemyKind) {
    let (mut app, enemy) = simulate_enemy_near_player(kind, EnemyBehavior::Engaged, 160, Vec3::new(5.0, 0.05, 0.0));
    let animator = find_enemy_animator(&mut app, enemy);
    assert!(
        count_playing_animations(&mut app, animator) > 0,
        "alive enemy animator should play an animation"
    );
    let hit_body = find_ragdoll_body(&mut app, enemy, RagdollBodyPart::Head);

    support::shoot_entity(&mut app, hit_body);
    assert_death(&mut app, enemy);
}

#[test]
fn player_damages_fighter_on_first_hit_and_kills_on_second() {
    let (mut app, enemy) = simulate_enemy_near_player(
        EnemyKind::Fighter,
        EnemyBehavior::Patrol,
        160,
        Vec3::new(5.0, 0.05, 0.0),
    );
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
    let (mut app, _) = simulate_enemy_near_player(
        EnemyKind::Fighter,
        EnemyBehavior::Engaged,
        16,
        Vec3::new(1.0, 0.05, 0.0),
    );
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

    wait_first_melee_hit(&mut app, survivable_health);
    let health_after_first_hit = get_player_health(&mut app);
    assert!(health_after_first_hit > 0.0);
    assert_eq!(health_after_first_hit, survivable_health - katana_damage);
    assert!(app.world().get::<PlayerDead>(player_entity).is_none());

    wait_player_death(&mut app);
    assert_eq!(get_player_health(&mut app), 0.0);
    assert_eq!(get_current_game_state(&mut app), GameState::GameOver);
}

fn wait_first_melee_hit(app: &mut App, starting_health: f32) {
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
    let far_player_position = Vec3::new(12.0, 0.05, 0.0);
    let (mut app, enemy) =
        simulate_enemy_near_player(EnemyKind::Fighter, EnemyBehavior::Engaged, 16, far_player_position);
    let pelvis = find_ragdoll_body(&mut app, enemy, RagdollBodyPart::Pelvis);
    let hit_body = find_ragdoll_body(&mut app, enemy, RagdollBodyPart::Head);

    let run_direction = (far_player_position - Vec3::new(0.0, 0.05, 0.0)).normalize();
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
    let pelvis_velocity = wait_for_ragdoll_activation_velocity(&mut app, pelvis);
    let forward_run_momentum = pelvis_velocity.dot(run_direction);
    assert!(
        forward_run_momentum >= minimum_kept_run_speed,
        "running fighter ragdoll pelvis should keep at least {minimum_kept_run_speed} of the run momentum of {fighter_run_speed}, got forward component {forward_run_momentum:.3} from velocity {pelvis_velocity}"
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
        .find(|(_, owner, body_part)| owner.get_owner() == enemy && **body_part == part)
        .map_or_else(|| panic!("ragdoll should have {part:?}"), |(entity, _, _)| entity)
}

#[derive(Clone)]
struct BoneState {
    entity: Entity,
    name: Box<str>,
    transform: GlobalTransform,
}

fn capture_gunner_bone_states(app: &mut App, gunner: Entity) -> Vec<BoneState> {
    let armature = find_enemy_animator(app, gunner);
    let skinned_mesh_joints = app
        .world_mut()
        .query::<(Entity, &SkinnedMesh)>()
        .iter(app.world())
        .map(|(entity, skinned_mesh)| (entity, skinned_mesh.joints.clone()))
        .collect::<Vec<_>>();
    let skeletal_bone_entities = skinned_mesh_joints
        .into_iter()
        .filter(|(mesh_entity, _)| is_descendant_of(app, *mesh_entity, armature))
        .flat_map(|(_, joint_entities)| joint_entities)
        .collect::<HashSet<_>>();
    app.world_mut()
        .query::<(Entity, &Name, &GlobalTransform)>()
        .iter(app.world())
        .filter(|(entity, _, _)| skeletal_bone_entities.contains(entity))
        .map(|(entity, name, transform)| BoneState {
            entity,
            name: name.as_str().into(),
            transform: *transform,
        })
        .collect()
}

fn is_finger_bone(name: &str) -> bool {
    ["index_", "middle_", "pinky_", "ring_", "thumb_"]
        .iter()
        .any(|finger_prefix| name.starts_with(finger_prefix))
}

fn assert_gunner_bone_poses_are_preserved_at_ragdoll_activation(
    bone_states_before_headshot: &[BoneState],
    bone_states_after_activation: &[BoneState],
) {
    let maximum_position_change_meters = 0.01;
    let maximum_rotation_change_radians = 0.5_f32.to_radians();
    let changed_bones = bone_states_before_headshot
        .iter()
        .filter(|state_before_headshot| !is_finger_bone(&state_before_headshot.name))
        .filter_map(|state_before_headshot| {
            let state_after_activation = bone_states_after_activation
                .iter()
                .find(|state_after_activation| state_after_activation.entity == state_before_headshot.entity)
                .expect("every gunner bone should exist after ragdoll activation");
            let position_change_meters = state_after_activation
                .transform
                .translation()
                .distance(state_before_headshot.transform.translation());
            let rotation_dot_product = state_before_headshot
                .transform
                .rotation()
                .dot(state_after_activation.transform.rotation())
                .abs()
                .clamp(0.0, 1.0);
            let rotation_change_radians = 2.0 * rotation_dot_product.acos();
            let position_changed = position_change_meters > maximum_position_change_meters;
            let rotation_changed = rotation_change_radians > maximum_rotation_change_radians;
            (position_changed || rotation_changed).then(|| {
                let rotation_change_degrees = rotation_change_radians.to_degrees();
                format!(
                    "{}: shift={position_change_meters:.6}m rotation={rotation_change_degrees:.2}°",
                    state_before_headshot.name,
                )
            })
        })
        .collect::<Vec<_>>();
    assert!(
        changed_bones.is_empty(),
        "ragdoll activation must preserve every gunner bone pose:\n{}",
        changed_bones.join("\n"),
    );
}

fn find_ragdoll_bone_map(app: &mut App, enemy: Entity) -> BoneMap {
    app.world()
        .get::<BoneMap>(enemy)
        .cloned()
        .unwrap_or_else(|| panic!("enemy {enemy:?} should carry a bone map"))
}

#[test]
fn ragdoll_activation_preserves_gunner_bone_poses() {
    let (mut app, gunner) =
        simulate_enemy_near_player(EnemyKind::Gunner, EnemyBehavior::Engaged, 16, Vec3::new(5.0, 0.05, 0.0));
    let player_entity = app
        .world_mut()
        .query_filtered::<Entity, With<Player>>()
        .single(app.world())
        .expect("one player should spawn");
    app.world_mut().entity_mut(player_entity).insert(Health::new(100_000.0));
    wait_until_gunner_is_attacking(&mut app, gunner);
    let bone_states_before_headshot = capture_gunner_bone_states(&mut app, gunner);
    let head = find_ragdoll_body(&mut app, gunner, RagdollBodyPart::Head);

    support::aim_at_entity(&mut app, head);
    support::press_shot_button_before_physics(&mut app);
    let activation_wait_updates = 64;
    let mut bone_states_before_activation = bone_states_before_headshot.clone();
    let mut bone_states_after_activation = None;
    for _ in 0..activation_wait_updates {
        app.update();
        let bone_states_current = capture_gunner_bone_states(&mut app, gunner);
        let enemy_is_dying = app.world().entity(gunner).contains::<Dying>();
        let head_is_active = app.world().get::<RigidBodyDisabled>(head).is_none();
        if enemy_is_dying && head_is_active {
            bone_states_after_activation = Some(bone_states_current);
            break;
        }
        bone_states_before_activation = bone_states_current;
    }
    let bone_states_after_activation =
        bone_states_after_activation.expect("gunner ragdoll should activate within the shot");
    assert_gunner_bone_poses_are_preserved_at_ragdoll_activation(
        &bone_states_before_activation,
        &bone_states_after_activation,
    );
}

#[test]
fn head_shot_kills_fighter() {
    let (mut app, enemy) =
        simulate_enemy_near_player(EnemyKind::Fighter, EnemyBehavior::Patrol, 42, Vec3::new(5.0, 0.05, 0.0));
    let hit_body = find_ragdoll_body(&mut app, enemy, RagdollBodyPart::Head);

    support::shoot_and_assert_kick_speed(&mut app, hit_body, 2.9);
    let settling_duration = 10.0;
    support::assert_settles(&mut app, hit_body, settling_duration);
}

#[test]
fn head_shot_kills_gunner() {
    let (mut app, enemy) =
        simulate_enemy_near_player(EnemyKind::Gunner, EnemyBehavior::Patrol, 0, Vec3::new(5.0, 0.05, 0.0));
    let hit_body = find_ragdoll_body(&mut app, enemy, RagdollBodyPart::Head);

    support::shoot_and_assert_kick_speed(&mut app, hit_body, 2.9);
    let settling_duration = 10.0;
    support::assert_settles(&mut app, hit_body, settling_duration);
}

#[test]
fn gunner_loads_one_hairstyle() {
    let (app, gunner) =
        simulate_enemy_near_player(EnemyKind::Gunner, EnemyBehavior::Patrol, 0, Vec3::new(5.0, 0.05, 0.0));
    let hair_root_count = app.world().get::<Children>(gunner).map_or(0, |children| {
        children
            .iter()
            .filter(|child| app.world().get::<WorldAssetRoot>(*child).is_some())
            .count()
    });
    assert_eq!(hair_root_count, 1, "gunner should load exactly one hairstyle");
}

#[test]
fn hand_shot_kills_fighter() {
    let (mut app, enemy) = simulate_enemy_near_player(
        EnemyKind::Fighter,
        EnemyBehavior::Patrol,
        160,
        Vec3::new(5.0, 0.05, 0.0),
    );
    let hit_body = find_ragdoll_body(&mut app, enemy, RagdollBodyPart::HandRight);

    support::shoot_and_assert_kick_speed(&mut app, hit_body, 2.9);
    let settling_duration = 10.0;
    support::assert_settles(&mut app, hit_body, settling_duration);
}

#[test]
fn hand_shot_kills_gunner() {
    let (mut app, enemy) =
        simulate_enemy_near_player(EnemyKind::Gunner, EnemyBehavior::Patrol, 0, Vec3::new(5.0, 0.05, 0.0));
    app.insert_resource(SubstepCount(30));
    let hit_body = find_ragdoll_body(&mut app, enemy, RagdollBodyPart::HandRight);

    support::shoot_and_assert_kick_speed(&mut app, hit_body, 2.9);
    let settling_duration = 10.0;
    support::assert_settles(&mut app, hit_body, settling_duration);
}

fn find_player_ragdoll_body(app: &mut App, part: RagdollBodyPart) -> Entity {
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
    app.world_mut()
        .query_filtered::<(Entity, &OwnedByCharacter, &RagdollBodyPart), ()>()
        .iter(app.world())
        .find(|(_, owner, body_part)| player_children.contains(&owner.get_owner()) && **body_part == part)
        .map_or_else(
            || panic!("player should have a ragdoll body for {part:?}"),
            |(entity, _, _)| entity,
        )
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

#[derive(Debug)]
struct HullSkinMeasurement {
    body_part: RagdollBodyPart,
    collider_center: Vec3,
    collider_extents: Vec3,
    skinned_center: Vec3,
    skinned_extents: Vec3,
    skinned_vertex_count: usize,
}

fn collect_joint_subtree(app: &App, root_joint: Entity) -> HashSet<Entity> {
    let mut included_joints = HashSet::from([root_joint]);
    let mut pending_joints = vec![root_joint];
    while let Some(joint_entity) = pending_joints.pop() {
        if let Some(children) = app.world().get::<Children>(joint_entity) {
            for child in children.iter() {
                if included_joints.insert(child) {
                    pending_joints.push(child);
                }
            }
        }
    }
    included_joints
}

fn collect_skinned_vertices_by_joint(app: &mut App) -> HashMap<Entity, Vec<Vec3>> {
    use bevy::mesh::skinning::SkinnedMeshInverseBindposes;
    use bevy::mesh::{Mesh, Mesh3d, VertexAttributeValues};
    let mut transforms = app.world_mut().query::<&GlobalTransform>();
    let mut skinned_meshes = app.world_mut().query::<(&Mesh3d, &SkinnedMesh)>();
    let mesh_assets = app.world().resource::<Assets<Mesh>>();
    let bindpose_assets = app.world().resource::<Assets<SkinnedMeshInverseBindposes>>();
    let mut vertices_by_joint = HashMap::<Entity, Vec<Vec3>>::new();
    for (mesh3d, skin) in skinned_meshes.iter(app.world()) {
        let Some(mesh) = mesh_assets.get(&mesh3d.0) else {
            continue;
        };
        let Some(inverse_bindposes) = bindpose_assets.get(&skin.inverse_bindposes) else {
            continue;
        };
        let VertexAttributeValues::Float32x3(positions) =
            mesh.attribute(Mesh::ATTRIBUTE_POSITION).expect("mesh positions")
        else {
            continue;
        };
        let VertexAttributeValues::Uint16x4(joint_indices) =
            mesh.attribute(Mesh::ATTRIBUTE_JOINT_INDEX).expect("joint indices")
        else {
            continue;
        };
        let VertexAttributeValues::Float32x4(joint_weights) =
            mesh.attribute(Mesh::ATTRIBUTE_JOINT_WEIGHT).expect("joint weights")
        else {
            continue;
        };
        let vertex_is_in_triangle: HashSet<usize> = mesh.indices().expect("mesh indices").iter().collect();
        for ((mesh_joint_indices, weights), (vertex_index, vertex_position)) in joint_indices
            .iter()
            .zip(joint_weights.iter())
            .zip(positions.iter().enumerate())
        {
            if !vertex_is_in_triangle.contains(&vertex_index) {
                continue;
            }
            let (strongest_index, strongest_weight) = weights
                .iter()
                .enumerate()
                .max_by(|(_, left_weight), (_, right_weight)| left_weight.total_cmp(right_weight))
                .expect("every vertex should carry weights");
            if *strongest_weight <= 0.0 {
                continue;
            }
            let joint_entity = skin.joints[mesh_joint_indices[strongest_index] as usize];
            let joint_transform = transforms.get(app.world(), joint_entity).expect("joint transform");
            let vertex_in_bind_joint = inverse_bindposes
                .get(mesh_joint_indices[strongest_index] as usize)
                .expect("inverse bindpose")
                .transform_point3(Vec3::from_array(*vertex_position));
            vertices_by_joint
                .entry(joint_entity)
                .or_default()
                .push(joint_transform.transform_point(vertex_in_bind_joint));
        }
    }
    vertices_by_joint
}

fn capture_ragdoll_bodies(app: &mut App, gunner: Entity) -> Vec<(RagdollBodyPart, Position, Rotation, Collider)> {
    let mut body_query = app
        .world_mut()
        .query_filtered::<(&RagdollBodyPart, &OwnedByCharacter, &Position, &Rotation, &Collider), ()>();
    body_query
        .iter(app.world())
        .filter(|(_, owner, _, _, _)| owner.get_owner() == gunner)
        .map(|(body_part, _, position, rotation, collider)| (*body_part, *position, *rotation, collider.clone()))
        .collect()
}

fn measure_hull_colliders_against_skin(app: &mut App, gunner: Entity) -> Vec<HullSkinMeasurement> {
    let bone_map = find_ragdoll_bone_map(app, gunner);
    let vertices_by_joint = collect_skinned_vertices_by_joint(app);
    let body_snapshot = capture_ragdoll_bodies(app, gunner);
    body_snapshot
        .into_iter()
        .map(|(body_part, position, rotation, collider)| {
            let specification = list_ragdoll_limbs()
                .iter()
                .find(|specification| specification.get_body_part() == body_part)
                .expect("every body part should have a limb specification");
            let (hull_center, hull_extents) = match collider.shape().as_typed_shape() {
                avian3d::parry::shape::TypedShape::ConvexPolyhedron(hull) => {
                    let world_hull_points: Vec<Vec3> = hull
                        .points()
                        .iter()
                        .map(|point| position.0 + rotation.0 * Vec3::new(point.x, point.y, point.z))
                        .collect();
                    let hull_min = world_hull_points
                        .iter()
                        .fold(Vec3::MAX, |bounds, point| bounds.min(*point));
                    let hull_max = world_hull_points
                        .iter()
                        .fold(Vec3::MIN, |bounds, point| bounds.max(*point));
                    ((hull_min + hull_max) * 0.5, hull_max - hull_min)
                }
                other => panic!("unexpected collider shape {other:?}"),
            };
            let root_joint = bone_map
                .find_bone(specification.get_start_bone())
                .expect("root joint for subtree");
            let included_joints = if specification.covers_bone_subtree() {
                collect_joint_subtree(app, root_joint)
            } else {
                HashSet::from([root_joint])
            };
            let skinned_vertices: Vec<Vec3> = included_joints
                .iter()
                .filter_map(|joint_entity| vertices_by_joint.get(joint_entity))
                .flatten()
                .copied()
                .collect();
            assert!(
                !skinned_vertices.is_empty(),
                "subtree for {body_part:?} should skin some vertices"
            );
            let skinned_min = skinned_vertices
                .iter()
                .fold(Vec3::MAX, |bounds, point| bounds.min(*point));
            let skinned_max = skinned_vertices
                .iter()
                .fold(Vec3::MIN, |bounds, point| bounds.max(*point));
            HullSkinMeasurement {
                body_part,
                collider_center: hull_center,
                collider_extents: hull_extents,
                skinned_center: (skinned_min + skinned_max) * 0.5,
                skinned_extents: skinned_max - skinned_min,
                skinned_vertex_count: skinned_vertices.len(),
            }
        })
        .collect()
}

struct ColliderPairPenetration {
    first_part: RagdollBodyPart,
    second_part: RagdollBodyPart,
    is_jointed_pair: bool,
    penetration: f32,
}

fn measure_collider_pair_penetrations(app: &mut App, gunner: Entity) -> Vec<ColliderPairPenetration> {
    use avian3d::parry::query::contact;
    let body_snapshot = capture_ragdoll_bodies(app, gunner);
    let jointed_pairs: HashSet<(RagdollBodyPart, RagdollBodyPart)> = list_ragdoll_limbs()
        .iter()
        .filter_map(|specification| {
            specification
                .get_parent_body_part()
                .map(|parent| (parent, specification.get_body_part()))
        })
        .flat_map(|(parent, child)| [(parent, child), (child, parent)])
        .collect();
    let mut penetrations = Vec::new();
    for (first_index, (first_part, first_position, first_rotation, first_collider)) in body_snapshot.iter().enumerate()
    {
        for (second_part, second_position, second_rotation, second_collider) in &body_snapshot[first_index + 1..] {
            let first_shape = match first_collider.shape().as_typed_shape() {
                avian3d::parry::shape::TypedShape::ConvexPolyhedron(hull) => hull,
                other => panic!("unexpected collider shape {other:?}"),
            };
            let second_shape = match second_collider.shape().as_typed_shape() {
                avian3d::parry::shape::TypedShape::ConvexPolyhedron(hull) => hull,
                other => panic!("unexpected collider shape {other:?}"),
            };
            let first_pose = avian3d::parry::math::Pose3::from_parts(first_position.0, first_rotation.0);
            let second_pose = avian3d::parry::math::Pose3::from_parts(second_position.0, second_rotation.0);
            let separation_prediction = 0.0;
            let pair_contact = contact(
                &first_pose,
                first_shape,
                &second_pose,
                second_shape,
                separation_prediction,
            )
            .expect("convex hull contact query should be supported");
            let is_jointed_pair = jointed_pairs.contains(&(*first_part, *second_part));
            let penetration = pair_contact.map_or(0.0, |pair_contact| -pair_contact.dist);
            if penetration > 0.0 {
                penetrations.push(ColliderPairPenetration {
                    first_part: *first_part,
                    second_part: *second_part,
                    is_jointed_pair,
                    penetration,
                });
            }
        }
    }
    penetrations
}

#[test]
fn gunner_colliders_do_not_overlap() {
    let (mut app, gunner) =
        simulate_enemy_near_player(EnemyKind::Gunner, EnemyBehavior::Engaged, 32, Vec3::new(3.0, 0.05, 0.0));
    run_updates(&mut app, 16);
    let penetrations = measure_collider_pair_penetrations(&mut app, gunner);
    for penetration in &penetrations {
        println!(
            "{:?}/{:?}: jointed={}, penetration {:.3} m",
            penetration.first_part, penetration.second_part, penetration.is_jointed_pair, penetration.penetration,
        );
    }
    let maximum_unjointed_penetration = 0.04;
    let offending_rows: Vec<String> = penetrations
        .iter()
        .filter(|penetration| !penetration.is_jointed_pair && penetration.penetration > maximum_unjointed_penetration)
        .map(|penetration| {
            format!(
                "{:?}/{:?}: penetration {:.3} m",
                penetration.first_part, penetration.second_part, penetration.penetration,
            )
        })
        .collect();
    assert!(
        offending_rows.is_empty(),
        "unjointed ragdoll colliders should not overlap:\n{}",
        offending_rows.join("\n"),
    );
}

#[test]
fn gunner_colliders_follow_animation() {
    let (mut app, gunner) =
        simulate_enemy_near_player(EnemyKind::Gunner, EnemyBehavior::Engaged, 32, Vec3::new(3.0, 0.05, 0.0));
    run_updates(&mut app, 16);
    let measurements = measure_hull_colliders_against_skin(&mut app, gunner);
    let maximum_center_offset = 0.08;
    let maximum_extents_delta = 0.15;
    let offending_rows: Vec<String> = measurements
        .iter()
        .filter(|measurement| {
            let center_offset = (measurement.collider_center - measurement.skinned_center).length();
            let extents_delta = (measurement.collider_extents - measurement.skinned_extents)
                .abs()
                .max_element();
            center_offset > maximum_center_offset || extents_delta > maximum_extents_delta
        })
        .map(|measurement| {
            let center_delta = (measurement.collider_center - measurement.skinned_center).length();
            let extents_delta = (measurement.collider_extents - measurement.skinned_extents)
                .abs()
                .max_element();
            format!(
                "{:?}: center delta {center_delta:.3} m, extents delta {extents_delta:.3} m, vertices {}",
                measurement.body_part, measurement.skinned_vertex_count,
            )
        })
        .collect();
    assert!(
        offending_rows.is_empty(),
        "every hull collider should still cover its skinned mesh while the animation plays:\n{}",
        offending_rows.join("\n"),
    );
}

fn wait_until_gunner_is_attacking(app: &mut App, gunner: Entity) {
    let attack_wait_updates = 10_000;
    for _ in 0..attack_wait_updates {
        app.update();
        if app
            .world()
            .get::<EnemyActivity>(gunner)
            .is_some_and(|activity| activity.state == AnimationState::Attacking)
        {
            return;
        }
    }
    panic!("gunner {gunner:?} should enter the attacking animation within {attack_wait_updates} updates");
}

fn wait_player_death(app: &mut App) {
    let death_updates = 10_000;
    for _ in 0..death_updates {
        app.update();
        if get_current_game_state(app) == GameState::GameOver {
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
    let (mut app, _) =
        simulate_enemy_near_player(EnemyKind::Gunner, EnemyBehavior::Engaged, 16, Vec3::new(6.0, 0.05, 0.0));
    let head_body = find_player_ragdoll_body(&mut app, RagdollBodyPart::Head);

    wait_player_death(&mut app);

    assert!(
        get_player_health(&mut app) <= 0.0,
        "dead player should have no health left"
    );
    assert!(
        app.world().get::<RigidBodyDisabled>(head_body).is_none(),
        "player head ragdoll body should activate on death"
    );
    assert_camera_follows_fallen_head(&mut app, head_body);
    assert_eq!(get_current_game_state(&mut app), GameState::GameOver);
}

#[test]
fn fighter_melee_kills_player_and_camera_falls_to_head() {
    let (mut app, _) = simulate_enemy_near_player(
        EnemyKind::Fighter,
        EnemyBehavior::Engaged,
        16,
        Vec3::new(1.0, 0.05, 0.0),
    );
    let initial_health = get_player_health(&mut app);
    let head_body = find_player_ragdoll_body(&mut app, RagdollBodyPart::Head);

    wait_player_death(&mut app);

    let health_after_death = get_player_health(&mut app);
    assert!(
        health_after_death < initial_health,
        "fighter melee should have damaged the player before death, got {health_after_death} from {initial_health}"
    );
    assert!(
        app.world().get::<RigidBodyDisabled>(head_body).is_none(),
        "player head ragdoll body should activate on death"
    );
    assert_camera_follows_fallen_head(&mut app, head_body);
    assert_eq!(get_current_game_state(&mut app), GameState::GameOver);
}
