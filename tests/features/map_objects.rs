use bevy::prelude::*;
use cathedral::app::build_test_app;
use cathedral::maps::build_map01;
use std::time::Duration;

use super::support::{
    FIXED_TIMESTEP_SECONDS, aim_at_entity, assert_kick_speed, assert_settles, find_named_body,
    press_shot_button_before_physics, release_shot_button,
};

fn create_map_test_app() -> App {
    let fixed_timestep = Duration::from_secs_f64(FIXED_TIMESTEP_SECONDS);
    let mut app = build_test_app(build_map01());
    app.insert_resource(Time::<Fixed>::from_duration(fixed_timestep));
    app
}

#[test]
fn shooting_laptop_applies_momentum_and_laptop_settles() {
    let mut app = create_map_test_app();
    let laptop = find_named_body(&mut app, "laptop");

    aim_at_entity(&mut app, laptop);
    let minimum_kick_speed = 0.00001;
    press_shot_button_before_physics(&mut app);
    assert_kick_speed(&mut app, laptop, minimum_kick_speed);
    release_shot_button(&mut app);

    let settling_deadline = 5.0;
    assert_settles(&mut app, laptop, settling_deadline);
}
