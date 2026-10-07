use avian3d::prelude::*;
use bevy::ecs::system::RunSystemOnce;
use bevy::prelude::*;
use cathedral::player::Player;
use cathedral::shooting::shoot;

pub const FIXED_TIMESTEP_SECONDS: f64 = 1.0 / 64.0;

pub fn shoot_entity(app: &mut App, target: Entity) -> Vec3 {
    let target_position = app
        .world()
        .get::<GlobalTransform>(target)
        .expect("target body transform")
        .translation();
    let mut player_transform = app
        .world_mut()
        .query_filtered::<&mut Transform, With<Player>>()
        .single_mut(app.world_mut())
        .expect("one player should spawn");
    let shot_direction = (target_position - player_transform.translation).normalize();
    *player_transform = Transform::from_translation(player_transform.translation).looking_at(target_position, Vec3::Y);
    app.world_mut()
        .resource_mut::<ButtonInput<MouseButton>>()
        .press(MouseButton::Left);
    app.world_mut().run_system_once(shoot).expect("shot system should run");
    app.world_mut()
        .resource_mut::<ButtonInput<MouseButton>>()
        .release(MouseButton::Left);
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
    let velocity = app.world().get::<LinearVelocity>(body).expect("body velocity").0;
    let angular_velocity = app
        .world()
        .get::<AngularVelocity>(body)
        .expect("body angular velocity")
        .0;
    println!(
        "body {body:?} {label}: linear_speed={:.3}, angular_speed={:.3}",
        velocity.length(),
        angular_velocity.length()
    );
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
    let velocity = app.world().get::<LinearVelocity>(body).expect("body velocity");
    let angular_velocity = app.world().get::<AngularVelocity>(body).expect("body angular velocity");
    panic!(
        "body {body:?} never settled within {settling_duration} seconds, linear_speed={:.6}, angular_speed={:.6}",
        velocity.length(),
        angular_velocity.length()
    );
}
