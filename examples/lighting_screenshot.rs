//! Headless renderer that captures screenshots of map01 lighting for tuning.
use bevy::prelude::*;
use cathedral::app::build_screenshot_app;

const EYE_HEIGHT: f32 = 1.9;
const FIRST_CAPTURE_FRAME: u32 = 16;
const FRAMES_BETWEEN_CAPTURES: u32 = 3;
const FINAL_FRAMES: u32 = 3;

fn main() {
    let viewpoints = vec![
        (Vec3::new(0.0, EYE_HEIGHT, 6.0), Vec3::new(0.0, 2.5, -9.0)),
        (Vec3::new(10.0, EYE_HEIGHT, 7.0), Vec3::new(-6.0, 2.0, -2.0)),
        (Vec3::new(0.0, EYE_HEIGHT, -2.0), Vec3::new(-6.0, 4.5, 4.5)),
    ];
    let frame_count = FIRST_CAPTURE_FRAME + FRAMES_BETWEEN_CAPTURES * viewpoints.len() as u32 + FINAL_FRAMES;

    let mut app = build_screenshot_app(viewpoints);
    let render_target = app.create_render_target(1280, 720);
    app.spawn_camera(render_target);

    for _ in 0..frame_count {
        app.update();
    }
}
