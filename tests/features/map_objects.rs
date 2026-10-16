use bevy::prelude::*;
use cathedral::app::build_test_app;
use cathedral::maps::build_map01;
use std::time::Duration;

use super::support::{FIXED_TIMESTEP_SECONDS, assert_received_kick, assert_settles, find_named_body, shoot_entity};

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

    shoot_entity(&mut app, laptop);
    assert_received_kick(&app, laptop);

    let settling_deadline = 5.0;
    assert_settles(&mut app, laptop, settling_deadline);
}
