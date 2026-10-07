use std::fs;
use std::path::PathBuf;
use std::process;

use bevy::prelude::*;
use cathedral::app::{CapturedScreenshots, build_headless_screenshot_app};

const RENDER_WIDTH: u32 = 64;
const RENDER_HEIGHT: u32 = 64;
const WARMUP_FRAMES: u32 = 30;
const BLACK_CHANNEL_THRESHOLD: u8 = 32;

#[test]
fn check_map01_screenshot() {
    let screenshot_dir = create_temp_screenshot_dir();
    let viewpoint = (Vec3::new(0.0, 1.85, 6.0), Vec3::new(0.0, 2.0, -6.0));
    let mut app = build_headless_screenshot_app(
        Some(viewpoint),
        RENDER_WIDTH,
        RENDER_HEIGHT,
        screenshot_dir.to_string_lossy().into_owned(),
    );

    for _ in 0..WARMUP_FRAMES {
        app.update();
    }

    let captured_screenshots = app.0.main.world().resource::<CapturedScreenshots>();
    assert_eq!(
        captured_screenshots.0.len(),
        1,
        "expected exactly one captured screenshot"
    );
    let image = &captured_screenshots.0[0];
    assert_eq!(image.width(), RENDER_WIDTH, "unexpected captured width");
    assert_eq!(image.height(), RENDER_HEIGHT, "unexpected captured height");
    let pixels = image.data.as_deref().unwrap();
    let distinct_colors: std::collections::HashSet<[u8; 4]> = pixels
        .chunks_exact(4)
        .map(|pixel| [pixel[0], pixel[1], pixel[2], pixel[3]])
        .collect();
    assert!(distinct_colors.len() > 1, "screenshot is uniform, map was not rendered");
    let brightest = pixels
        .chunks_exact(4)
        .flat_map(|pixel| pixel[..3].iter())
        .max()
        .copied()
        .unwrap();
    assert!(
        brightest > BLACK_CHANNEL_THRESHOLD,
        "screenshot is black, map lights were not rendered"
    );

    let entries: Vec<PathBuf> = fs::read_dir(&screenshot_dir)
        .unwrap()
        .map(|entry| entry.unwrap().path())
        .collect();
    assert_eq!(
        entries.len(),
        1,
        "expected exactly one screenshot in {screenshot_dir:?}, got {entries:?}"
    );
    let expected_name = format!("{}-map01.png", env!("CARGO_PKG_VERSION"));
    assert_eq!(
        entries[0].file_name().unwrap().to_string_lossy(),
        expected_name,
        "unexpected screenshot file name"
    );

    fs::remove_dir_all(&screenshot_dir).unwrap();
}

fn create_temp_screenshot_dir() -> PathBuf {
    let screenshot_dir = std::env::temp_dir().join(format!("cathedral-screenshot-test-{}", process::id()));
    fs::remove_dir_all(&screenshot_dir).ok();
    fs::create_dir_all(&screenshot_dir).unwrap();
    screenshot_dir
}
