use avian3d::dynamics::solver::joint_graph::JointGraph;
use avian3d::prelude::*;
use bevy::input::ButtonState;
use bevy::input::mouse::MouseButtonInput;
use bevy::prelude::*;
use cathedral::player::Player;
use std::collections::{HashSet, VecDeque};

pub const FIXED_TIMESTEP_SECONDS: f64 = 1.0 / 64.0;

pub fn find_named_body(app: &mut App, body_name: &str) -> Entity {
    app.world_mut()
        .query_filtered::<(Entity, &Name), With<RigidBody>>()
        .iter(app.world())
        .find(|(_, name)| name.as_str() == body_name)
        .map_or_else(
            || panic!("body {body_name} should exist after the game enters the playing state"),
            |(entity, _)| entity,
        )
}

pub fn shoot_entity(app: &mut App, target: Entity) -> Vec3 {
    let target_position = app.world().get::<Position>(target).expect("target body position").0;
    let mut player_transform = app
        .world_mut()
        .query_filtered::<&mut Transform, With<Player>>()
        .single_mut(app.world_mut())
        .expect("one player should spawn");
    let shot_direction = (target_position - player_transform.translation).normalize();
    let (yaw, pitch) = cathedral::player::compute_look_angles(player_transform.translation, target_position);
    player_transform.rotation = Quat::from_euler(EulerRot::YXZ, yaw, 0.0, 0.0);
    let mut camera_transform = app
        .world_mut()
        .query_filtered::<&mut Transform, With<cathedral::player::PlayerCamera>>()
        .single_mut(app.world_mut())
        .expect("one player camera should spawn");
    camera_transform.rotation = Quat::from_euler(EulerRot::YXZ, 0.0, pitch, 0.0);
    let mut pressed_message_writer = app.world_mut().resource_mut::<Messages<MouseButtonInput>>();
    pressed_message_writer.write(MouseButtonInput {
        button: MouseButton::Left,
        state: ButtonState::Pressed,
        window: Entity::PLACEHOLDER,
    });
    app.update();
    let mut released_message_writer = app.world_mut().resource_mut::<Messages<MouseButtonInput>>();
    released_message_writer.write(MouseButtonInput {
        button: MouseButton::Left,
        state: ButtonState::Released,
        window: Entity::PLACEHOLDER,
    });
    app.update();
    shot_direction
}

pub fn assert_received_kick(app: &mut App, target: Entity) {
    let minimum_kick_speed = 3.0;
    let linear_kick = app
        .world()
        .get::<LinearVelocity>(target)
        .expect("shot target should have linear velocity")
        .0;
    let angular_kick = app
        .world()
        .get::<AngularVelocity>(target)
        .expect("shot target should have angular velocity")
        .0;
    let target_kick_speed = linear_kick.length() + angular_kick.length();
    assert!(
        target_kick_speed >= minimum_kick_speed,
        "shot target should receive a kick of at least {minimum_kick_speed}, got linear_speed={:.6}, angular_speed={:.6}",
        linear_kick.length(),
        angular_kick.length()
    );

    let most_kicked_body = find_joint_connected_bodies(app, target)
        .into_iter()
        .max_by(|left_body, right_body| {
            let left_linear_kick = app
                .world()
                .get::<LinearVelocity>(*left_body)
                .expect("joint-connected body should have linear velocity")
                .0;
            let left_angular_kick = app
                .world()
                .get::<AngularVelocity>(*left_body)
                .expect("joint-connected body should have angular velocity")
                .0;
            let right_linear_kick = app
                .world()
                .get::<LinearVelocity>(*right_body)
                .expect("joint-connected body should have linear velocity")
                .0;
            let right_angular_kick = app
                .world()
                .get::<AngularVelocity>(*right_body)
                .expect("joint-connected body should have angular velocity")
                .0;
            let left_kick_speed = left_linear_kick.length() + left_angular_kick.length();
            let right_kick_speed = right_linear_kick.length() + right_angular_kick.length();
            left_kick_speed.total_cmp(&right_kick_speed)
        })
        .expect("target should be in its joint-connected bodies");
    assert_eq!(most_kicked_body, target, "shot target should receive the most kick");
}

fn log_speeds(app: &App, body: Entity, label: &str) {
    let body_name = app
        .world()
        .get::<Name>(body)
        .map_or_else(|| format!("{body:?}"), ToString::to_string);
    let velocity = app.world().get::<LinearVelocity>(body).expect("body velocity").0;
    let angular_velocity = app
        .world()
        .get::<AngularVelocity>(body)
        .expect("body angular velocity")
        .0;
    let most_energetic_awake_island_body = find_awake_island_body_with_most_kinetic_energy(app, body);
    println!(
        "{body_name} {label}: linear_speed={:.2}, angular_speed={:.2}\nmost active element: {}\n",
        velocity.length(),
        angular_velocity.length(),
        most_energetic_awake_island_body.unwrap_or_default()
    );
}

fn find_joint_connected_bodies(app: &App, body: Entity) -> Vec<Entity> {
    let joint_graph = app.world().resource::<JointGraph>();
    let mut connected_bodies = HashSet::from([body]);
    let mut bodies_to_visit = VecDeque::from([body]);
    while let Some(current_body) = bodies_to_visit.pop_front() {
        for joint in joint_graph.joints_of(current_body) {
            let connected_body = if joint.body1 == current_body {
                joint.body2
            } else {
                joint.body1
            };
            if connected_bodies.insert(connected_body) {
                bodies_to_visit.push_back(connected_body);
            }
        }
    }
    connected_bodies.into_iter().collect()
}

fn find_awake_island_body_with_most_kinetic_energy(app: &App, body: Entity) -> Option<String> {
    find_joint_connected_bodies(app, body)
        .into_iter()
        .filter_map(|connected_body| {
            let entity_reference = app.world().get_entity(connected_body).ok()?;
            let rigid_body = entity_reference.get::<RigidBody>()?;
            let linear_velocity = entity_reference.get::<LinearVelocity>()?;
            let angular_velocity = entity_reference.get::<AngularVelocity>()?;
            let mass = entity_reference.get::<ComputedMass>()?;
            let angular_inertia = entity_reference.get::<ComputedAngularInertia>()?;
            let is_sleeping = entity_reference.contains::<Sleeping>();
            (*rigid_body == RigidBody::Dynamic && !is_sleeping).then(|| {
                let body_name = entity_reference
                    .get::<Name>()
                    .map_or_else(|| format!("{connected_body:?}"), ToString::to_string);
                let linear_kinetic_energy = 0.5 * mass.value() * linear_velocity.length_squared();
                let angular_momentum = angular_inertia.tensor() * angular_velocity.0;
                let angular_kinetic_energy = 0.5 * angular_velocity.0.dot(angular_momentum);
                let kinetic_energy = linear_kinetic_energy + angular_kinetic_energy;
                let diagnostic = format!(
                    "{body_name}: linear_speed={:.2}, angular_speed={:.2}",
                    linear_velocity.length(),
                    angular_velocity.length()
                );
                (kinetic_energy, diagnostic)
            })
        })
        .max_by(|left, right| left.0.total_cmp(&right.0))
        .map(|(_, diagnostic)| diagnostic)
}

pub fn assert_settles(app: &mut App, body: Entity, settling_duration: f64) {
    let settling_steps = (settling_duration / FIXED_TIMESTEP_SECONDS) as usize;
    let log_interval = (0.5 / FIXED_TIMESTEP_SECONDS) as usize;
    for step in 0..settling_steps {
        app.update();
        if step % log_interval == 0 {
            let elapsed_seconds = (step as f64) * FIXED_TIMESTEP_SECONDS;
            log_speeds(app, body, &format!("after {elapsed_seconds:.1}s"));
        }
        if app.world().get::<Sleeping>(body).is_some() {
            log_speeds(app, body, "after sleep");
            return;
        }
    }
    let most_energetic_awake_island_body = find_awake_island_body_with_most_kinetic_energy(app, body);
    let body_name = app
        .world()
        .get::<Name>(body)
        .map_or_else(|| format!("{body:?}"), ToString::to_string);
    let velocity = app.world().get::<LinearVelocity>(body).expect("body velocity");
    let angular_velocity = app.world().get::<AngularVelocity>(body).expect("body angular velocity");
    panic!(
        "{body_name} never settled within {settling_duration} seconds, linear_speed={:.2}, angular_speed={:.2}\nmost active element: {}",
        velocity.length(),
        angular_velocity.length(),
        most_energetic_awake_island_body.unwrap_or_default()
    );
}
