use bevy::prelude::*;
use cathedral::app::build_headless_app;
use cathedral::maps::CurrentMap;
use cathedral::maps::pieces::LampShade;
use std::time::Duration;

mod support;

fn create_lamp_test_app() -> App {
    let fixed_timestep = Duration::from_secs_f64(support::FIXED_TIMESTEP_SECONDS);
    let mut app = build_headless_app(CurrentMap::Map01, None);
    app.insert_resource(Time::<Fixed>::from_duration(fixed_timestep));
    app
}

fn wait_for_lamp_shade(app: &mut App) -> Entity {
    for _ in 0..600 {
        app.update();
        let shade = app
            .world_mut()
            .query_filtered::<Entity, With<LampShade>>()
            .iter(app.world())
            .next();
        if let Some(shade) = shade {
            return shade;
        }
    }
    panic!("map01 should spawn hanging lamp shades within 600 frames");
}

#[test]
fn lamp_hangs_still_on_spawn() {
    let mut app = create_lamp_test_app();
    let shade = wait_for_lamp_shade(&mut app);

    let settling_duration = 15.0;
    support::assert_settles(&mut app, shade, settling_duration);
}

#[test]
fn shot_lamp_receives_kick_and_settles() {
    let mut app = create_lamp_test_app();
    let shade = wait_for_lamp_shade(&mut app);

    support::shoot_entity(&mut app, shade);
    support::assert_received_kick(&app, shade);
    let settling_duration = 15.0;
    support::assert_settles(&mut app, shade, settling_duration);
}
