use avian3d::prelude::RigidBody;
use bevy::prelude::*;
use terminal::app::build_test_app;
use terminal::maps::{SandField, build_intro};
use terminal::player::{CameraState, Player, compute_look_angles};

const DOOR_TRIGGER_DISTANCE: f32 = 7.0;
const STARTING_DISTANCE_OUTSIDE_DOOR: f32 = 9.0;
const TRANSFORM_TOLERANCE: f32 = 0.001;
const DOOR_ALIGNMENT_TOLERANCE: f32 = 0.01;
const MAX_CONDITION_UPDATES: usize = 1000;

struct DoorTestFixture {
    left_panel: Entity,
    right_panel: Entity,
    connector_center: Vec3,
    inward_direction: Vec3,
    left_closed_transform: Transform,
    right_closed_transform: Transform,
}

#[test]
fn walk_player_through_opening_door_and_close_it_inside() {
    let mut app = build_test_app(build_intro());
    let tower_root = find_named_entity(&mut app, "Intro tower");
    let door_mount = find_named_descendant(&app, tower_root, "Door mount");
    let mut door_test_fixture = None;
    for _ in 0..MAX_CONDITION_UPDATES {
        app.update();
        door_test_fixture = create_door_test_fixture(&mut app, door_mount);
        if door_test_fixture.is_some() {
            break;
        }
    }
    let door_test_fixture = door_test_fixture.expect("tower door should finish loading");
    let player = place_player_outside_door(&mut app, &door_test_fixture);
    wait_until_player_is_outside_with_closed_panels(&mut app, player, &door_test_fixture);

    let player_start_position = get_player_position(&app, player);
    assert!(
        player_start_position.distance(door_test_fixture.connector_center) > DOOR_TRIGGER_DISTANCE,
        "player should start outside the {DOOR_TRIGGER_DISTANCE} m trigger radius"
    );
    assert!(
        (player_start_position - door_test_fixture.connector_center).dot(door_test_fixture.inward_direction) < 0.0,
        "player should start outside the doorway"
    );

    app.world_mut()
        .resource_mut::<ButtonInput<KeyCode>>()
        .press(KeyCode::KeyW);
    walk_until_entering_trigger_radius(&mut app, player, &door_test_fixture);
    wait_until_panels_start_opening(&mut app, player, &door_test_fixture);
    walk_until_crossing_doorway(&mut app, player, &door_test_fixture);
    walk_until_beyond_trigger_radius_inside(&mut app, player, &door_test_fixture);
    wait_until_panels_are_closed(&mut app, &door_test_fixture);
    app.world_mut()
        .resource_mut::<ButtonInput<KeyCode>>()
        .release(KeyCode::KeyW);
}

fn create_door_test_fixture(app: &mut App, door_mount: Entity) -> Option<DoorTestFixture> {
    let door_mount_position = app.world().get::<GlobalTransform>(door_mount)?.translation();
    let door_roots = find_named_entities(app, "Door");
    let door_root = door_roots.into_iter().find_map(|door_root| {
        let door_connector = find_named_descendant_if_exists(app, door_root, "Door connector")?;
        let connector_position = app.world().get::<GlobalTransform>(door_connector)?.translation();
        (door_mount_position.distance(connector_position) <= DOOR_ALIGNMENT_TOLERANCE).then_some(door_root)
    })?;
    let door_connector = find_named_descendant_if_exists(app, door_root, "Door connector")?;
    let left_panel = find_named_descendant_if_exists(app, door_root, "Door panel left")?;
    let right_panel = find_named_descendant_if_exists(app, door_root, "Door panel right")?;
    if !app
        .world()
        .get::<RigidBody>(left_panel)
        .is_some_and(|rigid_body| *rigid_body == RigidBody::Kinematic)
        || !app
            .world()
            .get::<RigidBody>(right_panel)
            .is_some_and(|rigid_body| *rigid_body == RigidBody::Kinematic)
    {
        return None;
    }
    let connector_transform = app.world().get::<GlobalTransform>(door_connector)?;
    Some(DoorTestFixture {
        left_panel,
        right_panel,
        connector_center: connector_transform.translation(),
        inward_direction: connector_transform.affine().transform_vector3(Vec3::NEG_Z).normalize(),
        left_closed_transform: *app.world().get::<Transform>(left_panel)?,
        right_closed_transform: *app.world().get::<Transform>(right_panel)?,
    })
}

fn place_player_outside_door(app: &mut App, door_test_fixture: &DoorTestFixture) -> Entity {
    let inward_direction = door_test_fixture.inward_direction;
    let horizontal_inward_direction = Vec3::new(inward_direction.x, 0.0, inward_direction.z).normalize();
    let exterior_player_position =
        door_test_fixture.connector_center - horizontal_inward_direction * STARTING_DISTANCE_OUTSIDE_DOOR;
    let exterior_terrain_height = SandField::calculate_height_at(exterior_player_position.xz());
    let exterior_player_position = Vec3::new(
        exterior_player_position.x,
        exterior_terrain_height + 1.9,
        exterior_player_position.z,
    );
    let player = app
        .world_mut()
        .query_filtered::<Entity, With<Player>>()
        .single(app.world())
        .expect("intro map should spawn one player");
    let (player_yaw, _) = compute_look_angles(
        exterior_player_position,
        exterior_player_position + horizontal_inward_direction,
    );
    let mut player_transform = app
        .world_mut()
        .get_mut::<Transform>(player)
        .expect("player should have a transform");
    player_transform.translation = exterior_player_position;
    player_transform.rotation = Quat::from_rotation_y(player_yaw);
    app.world_mut()
        .get_mut::<CameraState>(player)
        .expect("player should have camera state")
        .yaw = player_yaw;
    player
}

fn wait_until_player_is_outside_with_closed_panels(app: &mut App, player: Entity, door_test_fixture: &DoorTestFixture) {
    let mut player_is_outside_with_closed_panels = false;
    for _ in 0..MAX_CONDITION_UPDATES {
        app.update();
        let player_position = get_player_position(app, player);
        if player_position.distance(door_test_fixture.connector_center) > DOOR_TRIGGER_DISTANCE
            && check_panels_match_closed_transforms(app, door_test_fixture)
        {
            player_is_outside_with_closed_panels = true;
            break;
        }
    }
    assert!(
        player_is_outside_with_closed_panels,
        "player should settle outside the trigger radius with the panels closed"
    );
}

fn walk_until_entering_trigger_radius(app: &mut App, player: Entity, door_test_fixture: &DoorTestFixture) {
    let mut entered_trigger_radius = false;
    for _ in 0..MAX_CONDITION_UPDATES {
        app.update();
        let player_position = get_player_position(app, player);
        if player_position.distance(door_test_fixture.connector_center) < DOOR_TRIGGER_DISTANCE {
            entered_trigger_radius = true;
            break;
        }
        assert_panels_are_closed(app, door_test_fixture);
    }
    assert!(
        entered_trigger_radius,
        "player should enter the {DOOR_TRIGGER_DISTANCE} m trigger radius"
    );
}

fn wait_until_panels_start_opening(app: &mut App, player: Entity, door_test_fixture: &DoorTestFixture) {
    let mut panels_started_opening_inside_trigger_radius = false;
    for _ in 0..MAX_CONDITION_UPDATES {
        app.update();
        let player_position = get_player_position(app, player);
        let (left_panel_transform, right_panel_transform) = get_panel_transforms(app, door_test_fixture);
        let left_panel_started_opening =
            !check_transforms_are_close(left_panel_transform, door_test_fixture.left_closed_transform);
        let right_panel_started_opening =
            !check_transforms_are_close(right_panel_transform, door_test_fixture.right_closed_transform);
        if left_panel_started_opening || right_panel_started_opening {
            assert!(
                left_panel_started_opening && right_panel_started_opening,
                "both door panels should start opening together"
            );
            assert!(
                player_position.distance(door_test_fixture.connector_center) < DOOR_TRIGGER_DISTANCE,
                "door panels should start opening only inside the trigger radius"
            );
            panels_started_opening_inside_trigger_radius = true;
            break;
        }
    }
    assert!(
        panels_started_opening_inside_trigger_radius,
        "door panels should start opening after the player enters the trigger radius"
    );
}

fn walk_until_crossing_doorway(app: &mut App, player: Entity, door_test_fixture: &DoorTestFixture) {
    let mut crossed_doorway = false;
    for _ in 0..MAX_CONDITION_UPDATES {
        app.update();
        let player_position = get_player_position(app, player);
        if (player_position - door_test_fixture.connector_center).dot(door_test_fixture.inward_direction) > 0.0 {
            crossed_doorway = true;
            break;
        }
    }
    assert!(crossed_doorway, "player should physically cross the doorway");
}

fn walk_until_beyond_trigger_radius_inside(app: &mut App, player: Entity, door_test_fixture: &DoorTestFixture) {
    let mut player_is_beyond_trigger_radius_inside = false;
    for _ in 0..MAX_CONDITION_UPDATES {
        app.update();
        let player_position = get_player_position(app, player);
        let player_is_inside = (player_position - door_test_fixture.connector_center)
            .dot(door_test_fixture.inward_direction)
            > DOOR_TRIGGER_DISTANCE;
        let player_is_beyond_trigger_radius =
            player_position.distance(door_test_fixture.connector_center) > DOOR_TRIGGER_DISTANCE;
        if player_is_inside && player_is_beyond_trigger_radius {
            player_is_beyond_trigger_radius_inside = true;
            break;
        }
    }
    assert!(
        player_is_beyond_trigger_radius_inside,
        "player should walk beyond the trigger radius on the inside"
    );
}

fn wait_until_panels_are_closed(app: &mut App, door_test_fixture: &DoorTestFixture) {
    let mut panels_returned_to_closed_transforms = false;
    for _ in 0..MAX_CONDITION_UPDATES {
        if check_panels_match_closed_transforms(app, door_test_fixture) {
            panels_returned_to_closed_transforms = true;
            break;
        }
        app.update();
    }
    assert!(
        panels_returned_to_closed_transforms,
        "both panels should return to their closed transforms while the player remains inside"
    );
}

fn assert_panels_are_closed(app: &App, door_test_fixture: &DoorTestFixture) {
    assert!(
        check_panels_match_closed_transforms(app, door_test_fixture),
        "door panels should remain closed outside the trigger radius"
    );
}

fn check_panels_match_closed_transforms(app: &App, door_test_fixture: &DoorTestFixture) -> bool {
    let (left_panel_transform, right_panel_transform) = get_panel_transforms(app, door_test_fixture);
    check_transforms_are_close(left_panel_transform, door_test_fixture.left_closed_transform)
        && check_transforms_are_close(right_panel_transform, door_test_fixture.right_closed_transform)
}

fn get_panel_transforms(app: &App, door_test_fixture: &DoorTestFixture) -> (Transform, Transform) {
    let left_panel_transform = *app
        .world()
        .get::<Transform>(door_test_fixture.left_panel)
        .expect("left door panel should have a local transform");
    let right_panel_transform = *app
        .world()
        .get::<Transform>(door_test_fixture.right_panel)
        .expect("right door panel should have a local transform");
    (left_panel_transform, right_panel_transform)
}

fn check_transforms_are_close(actual_transform: Transform, expected_transform: Transform) -> bool {
    actual_transform
        .translation
        .abs_diff_eq(expected_transform.translation, TRANSFORM_TOLERANCE)
        && actual_transform.rotation.angle_between(expected_transform.rotation) <= TRANSFORM_TOLERANCE
        && actual_transform
            .scale
            .abs_diff_eq(expected_transform.scale, TRANSFORM_TOLERANCE)
}

fn get_player_position(app: &App, player: Entity) -> Vec3 {
    app.world()
        .get::<GlobalTransform>(player)
        .expect("player should have a global transform")
        .translation()
}

fn find_named_entities(app: &mut App, expected_name: &str) -> Vec<Entity> {
    let world = app.world_mut();
    let mut named_entities = world.query::<(Entity, &Name)>();
    named_entities
        .iter(world)
        .filter(|(_, entity_name)| entity_name.as_str() == expected_name)
        .map(|(entity, _)| entity)
        .collect()
}

fn find_named_entity(app: &mut App, expected_name: &str) -> Entity {
    let mut named_entities = app.world_mut().query::<(Entity, &Name)>();
    named_entities
        .iter(app.world())
        .find(|(_, entity_name)| entity_name.as_str() == expected_name)
        .map_or_else(
            || panic!("entity named {expected_name} should exist"),
            |(entity, _)| entity,
        )
}

fn find_named_descendant(app: &App, root: Entity, expected_name: &str) -> Entity {
    find_named_descendant_if_exists(app, root, expected_name)
        .unwrap_or_else(|| panic!("scene should contain a {expected_name} node"))
}

fn find_named_descendant_if_exists(app: &App, root: Entity, expected_name: &str) -> Option<Entity> {
    let mut entities_to_visit = app
        .world()
        .get::<Children>(root)
        .map_or_else(Vec::new, |children| children.iter().collect::<Vec<_>>());
    while let Some(entity) = entities_to_visit.pop() {
        if app
            .world()
            .get::<Name>(entity)
            .is_some_and(|entity_name| entity_name.as_str() == expected_name)
        {
            return Some(entity);
        }
        if let Some(children) = app.world().get::<Children>(entity) {
            entities_to_visit.extend(children.iter());
        }
    }
    None
}
