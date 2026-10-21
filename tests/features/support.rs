use avian3d::dynamics::solver::joint_graph::JointGraph;
use avian3d::prelude::*;
use bevy::input::ButtonState;
use bevy::input::mouse::MouseButtonInput;
use bevy::prelude::*;
use bevy::time::TimeUpdateStrategy;
use cathedral::player::Player;
use std::collections::{HashSet, VecDeque};
use std::time::Duration;

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

pub fn aim_at_entity(app: &mut App, target: Entity) {
    let target_position = app.world().get::<Position>(target).expect("target body position").0;
    let mut player_transform = app
        .world_mut()
        .query_filtered::<&mut Transform, With<Player>>()
        .single_mut(app.world_mut())
        .expect("one player should spawn");
    let (yaw, pitch) = cathedral::player::compute_look_angles(player_transform.translation, target_position);
    player_transform.rotation = Quat::from_euler(EulerRot::YXZ, yaw, 0.0, 0.0);
    let mut camera_transform = app
        .world_mut()
        .query_filtered::<&mut Transform, With<cathedral::player::PlayerCamera>>()
        .single_mut(app.world_mut())
        .expect("one player camera should spawn");
    camera_transform.rotation = Quat::from_euler(EulerRot::YXZ, 0.0, pitch, 0.0);
}

pub fn release_shot_button(app: &mut App) {
    let mut released_message_writer = app.world_mut().resource_mut::<Messages<MouseButtonInput>>();
    released_message_writer.write(MouseButtonInput {
        button: MouseButton::Left,
        state: ButtonState::Released,
        window: Entity::PLACEHOLDER,
    });
    app.update();
}

pub fn press_shot_button(app: &mut App) {
    let mut pressed_message_writer = app.world_mut().resource_mut::<Messages<MouseButtonInput>>();
    pressed_message_writer.write(MouseButtonInput {
        button: MouseButton::Left,
        state: ButtonState::Pressed,
        window: Entity::PLACEHOLDER,
    });
    app.update();
}

pub fn press_shot_button_before_physics(app: &mut App) {
    let mut pressed_message_writer = app.world_mut().resource_mut::<Messages<MouseButtonInput>>();
    pressed_message_writer.write(MouseButtonInput {
        button: MouseButton::Left,
        state: ButtonState::Pressed,
        window: Entity::PLACEHOLDER,
    });
    let mut fixed_time = app.world_mut().resource_mut::<Time<Fixed>>();
    let fixed_time_overstep = fixed_time.overstep();
    fixed_time.discard_overstep(fixed_time_overstep);
    app.insert_resource(TimeUpdateStrategy::ManualDuration(Duration::ZERO));
    app.update();
    app.insert_resource(TimeUpdateStrategy::ManualDuration(Duration::from_millis(16)));
}

pub fn shoot_entity(app: &mut App, target: Entity) -> Vec3 {
    let target_position = app.world().get::<Position>(target).expect("target body position").0;
    let player_transform = app
        .world_mut()
        .query_filtered::<&Transform, With<Player>>()
        .single(app.world())
        .expect("one player should spawn");
    let shot_direction = (target_position - player_transform.translation).normalize();
    aim_at_entity(app, target);
    press_shot_button(app);
    release_shot_button(app);
    shot_direction
}

pub fn shoot_and_assert_kick_speed(app: &mut App, target: Entity, minimum_kick_speed: f32) {
    aim_at_entity(app, target);
    press_shot_button_before_physics(app);

    if app.world().get::<RigidBodyDisabled>(target).is_some() {
        let activation_wait_updates = 64;
        for _ in 0..activation_wait_updates {
            if app.world().get::<RigidBodyDisabled>(target).is_none() {
                break;
            }
            app.update();
        }
        assert!(
            app.world().get::<RigidBodyDisabled>(target).is_none(),
            "shot target should activate within {activation_wait_updates} updates"
        );
    }

    let target_linear_speed = app
        .world()
        .get::<LinearVelocity>(target)
        .expect("shot target should have linear velocity")
        .length();
    if target_linear_speed < minimum_kick_speed {
        let connected_body_kicks: Vec<_> = find_joint_connected_bodies(app, target)
            .into_iter()
            .filter_map(|connected_body| {
                let body_part = app.world().get::<cathedral::ragdoll::RagdollBodyPart>(connected_body)?;
                let velocity = app.world().get::<LinearVelocity>(connected_body)?;
                Some((*body_part, velocity.length()))
            })
            .collect();
        panic!(
            "shot should produce linear speed of at least {minimum_kick_speed}, got target_linear_speed={target_linear_speed:.6}, connected body linear speeds={connected_body_kicks:?}"
        );
    }

    release_shot_button(app);
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
    let dynamic_island_bodies: Vec<_> = find_joint_connected_bodies(app, body)
        .into_iter()
        .filter(|island_body| {
            app.world()
                .get::<RigidBody>(*island_body)
                .is_some_and(|rigid_body| *rigid_body == RigidBody::Dynamic)
        })
        .collect();
    let settling_steps = (settling_duration / FIXED_TIMESTEP_SECONDS) as usize;
    for _ in 0..settling_steps {
        app.update();
        let island_is_sleeping = dynamic_island_bodies
            .iter()
            .all(|island_body| app.world().get::<Sleeping>(*island_body).is_some());
        if island_is_sleeping {
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
        "{body_name} never settled within {settling_duration} seconds, linear_speed={:.2}, angular_speed={:.2}; most active element: {}",
        velocity.length(),
        angular_velocity.length(),
        most_energetic_awake_island_body.unwrap_or_default()
    );
}
