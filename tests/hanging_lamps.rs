use avian3d::prelude::*;
use bevy::prelude::*;
use cathedral::app::build_test_app;
use cathedral::maps::CurrentMap;
use cathedral::maps::pieces::LampShade;
use cathedral::player::Player;
use std::time::Duration;

mod support;

use support::{FIXED_TIMESTEP_SECONDS, shoot_entity};

fn find_closest_body_with_marker<Marker: Component>(app: &mut App) -> Option<Entity> {
    let player_position = app
        .world_mut()
        .query_filtered::<&GlobalTransform, With<Player>>()
        .single(app.world())
        .ok()?
        .translation();
    app.world_mut()
        .query_filtered::<(Entity, &GlobalTransform), With<Marker>>()
        .iter(app.world())
        .map(|(entity, transform)| (entity, transform.translation().distance_squared(player_position)))
        .min_by(|left, right| left.1.total_cmp(&right.1))
        .map(|(entity, _)| entity)
}

struct SwingSample {
    horizontal_displacement: Vec2,
}

struct SwingTrace {
    samples: Vec<SwingSample>,
}

fn record_shade_swing(app: &mut App, shade: Entity, duration_seconds: f64) -> SwingTrace {
    let rest_position = app
        .world()
        .get::<GlobalTransform>(shade)
        .expect("shade transform")
        .translation();
    let sample_count = (duration_seconds / FIXED_TIMESTEP_SECONDS) as usize;
    let mut samples = Vec::with_capacity(sample_count);
    for _ in 0..sample_count {
        let position = app
            .world()
            .get::<GlobalTransform>(shade)
            .expect("shade transform")
            .translation();
        let displacement = position - rest_position;
        samples.push(SwingSample {
            horizontal_displacement: Vec2::new(displacement.x, displacement.z),
        });
        app.update();
    }
    SwingTrace { samples }
}

fn find_swing_zero_crossings(displacement: &[f32]) -> Vec<usize> {
    let mut crossings = Vec::new();
    for sample_index in 1..displacement.len() {
        if displacement[sample_index - 1] * displacement[sample_index] < 0.0 {
            crossings.push(sample_index);
        }
    }
    crossings
}

fn find_half_swing_peaks(displacement: &[f32]) -> Vec<(f64, f32)> {
    let zero_crossings = find_swing_zero_crossings(displacement);
    let mut segment_bounds = Vec::new();
    let mut previous_bound = 0;
    for &crossing in &zero_crossings {
        segment_bounds.push((previous_bound, crossing));
        previous_bound = crossing;
    }
    segment_bounds.push((previous_bound, displacement.len()));
    let mut peaks = Vec::new();
    for (segment_start, segment_end) in segment_bounds {
        let Some((peak_index, _)) = displacement[segment_start..segment_end]
            .iter()
            .enumerate()
            .max_by(|left, right| left.1.abs().total_cmp(&right.1.abs()))
        else {
            continue;
        };
        if displacement[segment_start + peak_index].abs() < 0.005 {
            continue;
        }
        peaks.push((
            (segment_start + peak_index) as f64 * FIXED_TIMESTEP_SECONDS,
            displacement[segment_start + peak_index],
        ));
    }
    peaks
}

fn expected_period_seconds() -> f64 {
    2.0 * std::f64::consts::PI * (2.2f64 / 9.8).sqrt()
}
const PERIOD_TOLERANCE: f64 = 0.25;
const MINIMUM_COMPLETE_SWING_COUNT: usize = 5;

fn create_lamp_test_app() -> App {
    let fixed_timestep = Duration::from_secs_f64(FIXED_TIMESTEP_SECONDS);
    let mut app = build_test_app(CurrentMap::Map01, None);
    app.insert_resource(Time::<Fixed>::from_duration(fixed_timestep));
    app
}

fn wait_for_lamp_shade(app: &mut App) -> Entity {
    for _ in 0..600 {
        app.update();
        let shade = find_closest_body_with_marker::<LampShade>(app);
        if let Some(shade) = shade {
            return shade;
        }
    }
    panic!("map01 should spawn hanging lamp shades within 600 frames");
}

fn record_shot_swing_trace(app: &mut App, shade: Entity) -> Vec<f32> {
    let shot_direction = shoot_entity(app, shade);
    let swing_horizontal_direction = Vec2::new(shot_direction.x, shot_direction.z).normalize();
    let trace = record_shade_swing(app, shade, 15.0);
    trace
        .samples
        .iter()
        .map(|sample| sample.horizontal_displacement.dot(swing_horizontal_direction))
        .collect()
}

#[test]
fn lamp_hangs_still_on_spawn() {
    let mut app = create_lamp_test_app();
    let shade = wait_for_lamp_shade(&mut app);

    let settling_deadline = 1.0;
    support::assert_settles(&mut app, shade, settling_deadline);
}

#[test]
fn lamp_completes_five_swings_within_fifteen_seconds() {
    let mut app = create_lamp_test_app();
    let shade = wait_for_lamp_shade(&mut app);

    let displacement = record_shot_swing_trace(&mut app, shade);
    let peaks = find_half_swing_peaks(&displacement);
    let required_peak_count = MINIMUM_COMPLETE_SWING_COUNT * 2;
    assert!(
        peaks.len() >= required_peak_count,
        "lamp should complete {MINIMUM_COMPLETE_SWING_COUNT} swings after a shot, got {peaks:?}"
    );
    let swing_peaks = &peaks[..required_peak_count];
    for peak_pair in swing_peaks.windows(2) {
        assert!(
            peak_pair[0].1.signum() != peak_pair[1].1.signum(),
            "successive swing peaks should alternate sides, got {swing_peaks:?}"
        );
    }
}

#[test]
fn lamp_swing_period_matches_cable_length() {
    let mut app = create_lamp_test_app();
    let shade = wait_for_lamp_shade(&mut app);

    let displacement = record_shot_swing_trace(&mut app, shade);
    let crossings = find_swing_zero_crossings(&displacement);
    assert!(
        crossings.len() >= 3,
        "not enough zero crossings to measure a period: {crossings:?}"
    );
    let crossing_times: Vec<f64> = crossings
        .iter()
        .map(|&index| index as f64 * FIXED_TIMESTEP_SECONDS)
        .collect();
    let mut half_periods = Vec::new();
    for pair in crossing_times.windows(2) {
        half_periods.push(pair[1] - pair[0]);
    }
    let average_half_period = half_periods.iter().sum::<f64>() / half_periods.len() as f64;
    let measured_period = average_half_period * 2.0;
    let expected_period = expected_period_seconds();
    assert!(
        (measured_period - expected_period).abs() <= expected_period * PERIOD_TOLERANCE,
        "swing period {measured_period:.3}s should match cable length period {expected_period:.3}s within {}%",
        PERIOD_TOLERANCE * 100.0
    );
}

#[test]
fn lamp_swing_amplitude_decays_monotonically() {
    let mut app = create_lamp_test_app();
    let shade = wait_for_lamp_shade(&mut app);

    let displacement = record_shot_swing_trace(&mut app, shade);
    let peaks = find_half_swing_peaks(&displacement);
    assert!(peaks.len() >= 4, "expected at least 4 half-swing peaks, got {peaks:?}");
    let same_sign_peak_pairs: Vec<(f64, f32, f64, f32)> = peaks
        .windows(3)
        .filter(|triple| triple[0].1.signum() == triple[2].1.signum())
        .map(|triple| (triple[0].0, triple[0].1, triple[2].0, triple[2].1))
        .collect();
    assert!(
        same_sign_peak_pairs.len() >= 3,
        "expected at least 3 same-sign peak pairs, got {same_sign_peak_pairs:?}"
    );
    for (previous_time, previous_peak, peak_time, peak) in same_sign_peak_pairs {
        assert!(
            peak.abs() < previous_peak.abs(),
            "full-swing peaks should decay monotonically, got {previous_peak} at {previous_time}s then {peak} at {peak_time}s"
        );
    }
}

#[test]
fn lamp_swing_amplitude_matches_bullet_momentum() {
    let mut app = create_lamp_test_app();
    let shade = wait_for_lamp_shade(&mut app);

    let displacement = record_shot_swing_trace(&mut app, shade);
    let first_peak = displacement
        .iter()
        .fold(0.0f32, |maximum, value| maximum.max(value.abs()));
    assert!(
        first_peak >= 0.1,
        "bullet momentum should produce a clearly visible swing, first peak was {first_peak}"
    );
    let maximum_safe_horizontal_displacement = 2.0;
    assert!(
        first_peak <= maximum_safe_horizontal_displacement,
        "lamp should not slam the ceiling after a shot, first peak was {first_peak}"
    );
}

#[test]
fn shot_lamp_settles_within_documented_time() {
    let mut app = create_lamp_test_app();
    let shade = wait_for_lamp_shade(&mut app);

    shoot_entity(&mut app, shade);
    support::assert_received_kick(&app, shade);
    let visible_swing_duration = 30.0;
    let visible_swing_steps = (visible_swing_duration / FIXED_TIMESTEP_SECONDS) as usize;
    for _ in 0..visible_swing_steps {
        app.update();
        assert!(
            app.world().get::<Sleeping>(shade).is_none(),
            "lamp should still be swinging through {MINIMUM_COMPLETE_SWING_COUNT} visible swings after the shot"
        );
    }
    let settling_deadline = 45.0;
    let remaining_settling_duration = settling_deadline - visible_swing_duration;
    support::assert_settles(&mut app, shade, remaining_settling_duration);
}
