use std::collections::HashSet;
use std::fs;
use std::path::PathBuf;
use std::process;
use std::sync::Mutex;

use avian3d::prelude::*;
use bevy::prelude::*;
use cathedral::app::{ScreenshotOptions, build_screenshot_app};
use cathedral::maps::CurrentMap;
use cathedral::shooting::Gun;

const RENDER_WIDTH: u32 = 64;
const RENDER_HEIGHT: u32 = 64;
const COLLIDER_RENDER_WIDTH: u32 = 1440;
const COLLIDER_RENDER_HEIGHT: u32 = 1440;
const BLACK_CHANNEL_THRESHOLD: u8 = 32;

static SCREENSHOT_TEST_MUTEX: Mutex<()> = Mutex::new(());

#[test]
fn screenshot_app_captures_map01_frame() {
    let _screenshot_test_guard = SCREENSHOT_TEST_MUTEX.lock().unwrap();
    let screenshot_dir = create_temp_screenshot_dir("map01");
    let viewpoint = (Vec3::new(0.0, 1.85, 6.0), Vec3::new(0.0, 2.0, -6.0));
    let mut app = build_screenshot_app(
        CurrentMap::Map01,
        Some(viewpoint),
        RENDER_WIDTH,
        RENDER_HEIGHT,
        screenshot_dir.to_string_lossy().into_owned(),
        ScreenshotOptions {
            screenshot_name: "map01",
            show_colliders: false,
            show_revolver_muzzle_flash: false,
        },
    );

    let captured_screenshots = app.run_until_screenshot_captured();
    assert_eq!(
        captured_screenshots.0.len(),
        1,
        "expected exactly one captured screenshot"
    );
    let image = &captured_screenshots.0[0];
    assert_eq!(image.width(), RENDER_WIDTH, "unexpected captured width");
    assert_eq!(image.height(), RENDER_HEIGHT, "unexpected captured height");
    let pixels = image.data.as_deref().unwrap();
    let (rgba_pixels, incomplete_pixel_channels) = pixels.as_chunks::<4>();
    assert!(
        incomplete_pixel_channels.is_empty(),
        "image data should contain complete RGBA pixels"
    );
    let distinct_colors: HashSet<[u8; 4]> = rgba_pixels.iter().copied().collect();
    assert!(distinct_colors.len() > 1, "screenshot is uniform, map was not rendered");
    let brightest = rgba_pixels
        .iter()
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

#[test]
fn screenshot_app_captures_revolver_muzzle_flash_and_light() {
    let _screenshot_test_guard = SCREENSHOT_TEST_MUTEX.lock().unwrap();
    let idle_screenshot_dir = create_temp_screenshot_dir("idle-map01");
    let mut idle_app = build_screenshot_app(
        CurrentMap::Map01,
        None,
        RENDER_WIDTH,
        RENDER_HEIGHT,
        idle_screenshot_dir.to_string_lossy().into_owned(),
        ScreenshotOptions {
            screenshot_name: "idle-map01",
            show_colliders: false,
            show_revolver_muzzle_flash: false,
        },
    );
    let idle_image = idle_app.run_until_screenshot_captured().0[0].clone();

    let muzzle_flash_screenshot_dir = create_temp_screenshot_dir("revolver-muzzle-flash");
    let mut muzzle_flash_app = build_screenshot_app(
        CurrentMap::Map01,
        None,
        RENDER_WIDTH,
        RENDER_HEIGHT,
        muzzle_flash_screenshot_dir.to_string_lossy().into_owned(),
        ScreenshotOptions {
            screenshot_name: "revolver-muzzle-flash",
            show_colliders: false,
            show_revolver_muzzle_flash: true,
        },
    );
    let muzzle_flash_image = muzzle_flash_app.run_until_screenshot_captured().0[0].clone();

    let idle_pixels = idle_image.data.as_deref().unwrap();
    let muzzle_flash_pixels = muzzle_flash_image.data.as_deref().unwrap();
    let greatest_channel_difference = idle_pixels
        .iter()
        .zip(muzzle_flash_pixels)
        .map(|(idle_channel, muzzle_flash_channel)| idle_channel.abs_diff(*muzzle_flash_channel))
        .max()
        .unwrap();
    let required_channel_difference = 32;
    assert!(
        greatest_channel_difference >= required_channel_difference,
        "expected the revolver muzzle flash and light to change the captured scene"
    );

    fs::remove_dir_all(&idle_screenshot_dir).unwrap();
    fs::remove_dir_all(&muzzle_flash_screenshot_dir).unwrap();
}

#[test]
fn screenshot_app_with_colliders_hides_weapons_and_enables_gizmos() {
    let _screenshot_test_guard = SCREENSHOT_TEST_MUTEX.lock().unwrap();
    let screenshot_dir = create_temp_screenshot_dir("collider-inspection");
    let mut app = build_screenshot_app(
        CurrentMap::ColliderInspection,
        None,
        COLLIDER_RENDER_WIDTH,
        COLLIDER_RENDER_HEIGHT,
        screenshot_dir.to_string_lossy().into_owned(),
        ScreenshotOptions {
            screenshot_name: "collider-inspection",
            show_colliders: true,
            show_revolver_muzzle_flash: false,
        },
    );

    let captured_screenshots = app.run_until_screenshot_captured();
    assert_eq!(
        captured_screenshots.0.len(),
        1,
        "expected exactly one captured screenshot"
    );
    let image = &captured_screenshots.0[0];
    assert_eq!(image.width(), COLLIDER_RENDER_WIDTH, "unexpected captured width");
    assert_eq!(image.height(), COLLIDER_RENDER_HEIGHT, "unexpected captured height");

    let world = app.0.main.world_mut();
    let mut gun_visibility_query = world.query_filtered::<&Visibility, With<Gun>>();
    let gun_visibilities: Vec<Visibility> = gun_visibility_query.iter(world).copied().collect();
    assert!(!gun_visibilities.is_empty(), "expected at least one gun entity");
    assert!(
        gun_visibilities
            .iter()
            .all(|visibility| *visibility == Visibility::Hidden),
        "screenshot app with colliders must hide weapons"
    );

    let gizmo_configs = world.resource::<GizmoConfigStore>();
    let (physics_gizmos_config, _) = gizmo_configs.config::<PhysicsGizmos>();
    assert!(
        physics_gizmos_config.enabled,
        "screenshot app with colliders must enable collider gizmos"
    );

    fs::remove_dir_all(&screenshot_dir).unwrap();
}

fn create_temp_screenshot_dir(test_name: &str) -> PathBuf {
    let screenshot_dir =
        std::env::temp_dir().join(format!("cathedral-screenshot-test-{}-{}", process::id(), test_name));
    fs::remove_dir_all(&screenshot_dir).ok();
    fs::create_dir_all(&screenshot_dir).unwrap();
    screenshot_dir
}
