use bevy::prelude::*;
use std::time::Duration;
use terminal::app::build_test_app;
use terminal::maps::build_map01;

use super::support::{FIXED_TIMESTEP_SECONDS, assert_settles, find_named_body, shoot_and_assert_kick_speed};

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

    let minimum_kick_speed = 0.000001;
    shoot_and_assert_kick_speed(&mut app, laptop, minimum_kick_speed);

    let settling_deadline = 5.0;
    assert_settles(&mut app, laptop, settling_deadline);
}
