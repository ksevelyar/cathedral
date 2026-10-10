use avian3d::dynamics::solver::joint_graph::JointGraph;
use avian3d::prelude::*;
use bevy::input::ButtonState;
use bevy::input::mouse::MouseButtonInput;
use bevy::prelude::*;
use cathedral::player::Player;
use std::collections::{HashSet, VecDeque};

pub const FIXED_TIMESTEP_SECONDS: f64 = 1.0 / 64.0;

pub fn shoot_entity(app: &mut App, target: Entity) -> Vec3 {
    let target_position = app.world().get::<Position>(target).expect("target body position").0;
    let mut player_transform = app
        .world_mut()
        .query_filtered::<&mut Transform, With<Player>>()
        .single_mut(app.world_mut())
        .expect("one player should spawn");
    let shot_direction = (target_position - player_transform.translation).normalize();
    *player_transform = Transform::from_translation(player_transform.translation).looking_at(target_position, Vec3::Y);
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

pub fn assert_received_kick(app: &App, body: Entity) {
    let minimum_kick_speed = 0.5;
    let kick = app
        .world()
        .get::<LinearVelocity>(body)
        .expect("shot body should have linear velocity")
        .0;
    let angular_kick = app
        .world()
        .get::<AngularVelocity>(body)
        .expect("shot body should have angular velocity")
        .0;
    assert!(kick.is_finite(), "kick {kick:?} should be finite");
    assert!(
        kick.length() + angular_kick.length() >= minimum_kick_speed,
        "shot body should receive a kick of at least {minimum_kick_speed}, got linear_speed={:.6}, angular_speed={:.6}",
        kick.length(),
        angular_kick.length()
    );
    log_speeds(app, body, "after kick");
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
    println!(
        "{body_name} {label}: linear_speed={:.2}, angular_speed={:.2}",
        velocity.length(),
        angular_velocity.length()
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
        "{body_name} never settled within {settling_duration} seconds, linear_speed={:.2}, angular_speed={:.2}\n\nThe most active element:\n{}",
        velocity.length(),
        angular_velocity.length(),
        most_energetic_awake_island_body.unwrap_or_default()
    );
}
