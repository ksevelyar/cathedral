use std::collections::HashSet;
use std::fs;
use std::path::PathBuf;
use std::process;

use avian3d::prelude::*;
use bevy::prelude::*;
use cathedral::app::{Configuration, build_screenshot_app, request_screenshot, wait_screenshot_capture};
use cathedral::enemies::{EnemyBehavior, EnemyKind};
use cathedral::maps::{Map, MapBuilder, build_map01};
use cathedral::player::PlayerCamera;
use cathedral::shooting::Gun;

const RENDER_WIDTH: u32 = 64;
const RENDER_HEIGHT: u32 = 64;
const COLLIDER_RENDER_WIDTH: u32 = 1440;
const COLLIDER_RENDER_HEIGHT: u32 = 1440;
const BLACK_CHANNEL_THRESHOLD: u8 = 32;

fn build_gunner_colliders_map() -> Map {
    let name = "gunner-colliders";
    let floor_size = Vec3::new(60.0, 0.1, 60.0);
    MapBuilder::new(name)
        .with_player_position(Vec3::new(0.0, 1.8, 3.0), Vec3::new(0.0, 1.0, 0.0))
        .with_floor(floor_size)
        .with_enemy(
            vec![Vec3::ZERO],
            Vec3::ZERO + Vec3::NEG_Z,
            EnemyKind::Gunner,
            EnemyBehavior::Patrol,
        )
        .build()
}

#[test]
fn screenshot_app_captures_map01_frame() {
    let screenshot_dir = create_temp_screenshot_dir("map01");
    let viewpoint = "control-room-entrance".to_owned();
    let mut app = build_screenshot_app(
        build_map01(),
        Configuration {
            viewpoint: Some(viewpoint),
            ..default()
        },
        RENDER_WIDTH,
        RENDER_HEIGHT,
        screenshot_dir.to_string_lossy().into_owned(),
        "map01",
    );
    request_screenshot(app.world_mut());

    let captured_image = wait_screenshot_capture(&mut app)
        .0
        .first()
        .expect("exactly one screenshot should be captured")
        .clone();
    assert_eq!(captured_image.width(), RENDER_WIDTH, "unexpected captured width");
    assert_eq!(captured_image.height(), RENDER_HEIGHT, "unexpected captured height");
    let screenshot_camera_position = app
        .world_mut()
        .query_filtered::<&Transform, With<PlayerCamera>>()
        .single(app.world())
        .expect("screenshot should retain the selected player camera")
        .translation;
    let expected_screenshot_camera_position = Vec3::new(10.0, 1.9, 7.0);
    assert!(
        screenshot_camera_position.distance(expected_screenshot_camera_position) < 0.001,
        "screenshot camera should use the selected named viewpoint, got {screenshot_camera_position:?}"
    );
    let pixels = captured_image.data.as_deref().unwrap();
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
    let expected_name = format!("{}-map01-control-room-entrance.png", env!("CARGO_PKG_VERSION"));
    assert_eq!(
        entries[0].file_name().unwrap().to_string_lossy(),
        expected_name,
        "unexpected screenshot file name"
    );

    fs::remove_dir_all(&screenshot_dir).unwrap();
}

#[test]
fn screenshot_app_captures_the_intro_dunes_viewpoint_by_name() {
    let screenshot_dir = create_temp_screenshot_dir("intro-dunes");
    let mut app = build_screenshot_app(
        cathedral::maps::build_intro(),
        Configuration {
            viewpoint: Some("dunes".to_owned()),
            ..default()
        },
        RENDER_WIDTH,
        RENDER_HEIGHT,
        screenshot_dir.to_string_lossy().into_owned(),
        "intro",
    );
    request_screenshot(app.world_mut());
    assert_eq!(wait_screenshot_capture(&mut app).0.len(), 1);
    let screenshot_camera_position = app
        .world_mut()
        .query_filtered::<&Transform, With<PlayerCamera>>()
        .single(app.world())
        .expect("screenshot should retain the selected player camera")
        .translation;
    let expected_screenshot_camera_position = Vec3::new(-14.755, 3.537, 37.628);
    assert!(
        screenshot_camera_position.distance(expected_screenshot_camera_position) < 0.001,
        "dunes screenshot should use the map's named viewpoint"
    );
    let screenshot_entries: Vec<PathBuf> = fs::read_dir(&screenshot_dir)
        .unwrap()
        .map(|entry| entry.unwrap().path())
        .collect();
    let expected_screenshot_name = format!("{}-intro-dunes.png", env!("CARGO_PKG_VERSION"));
    assert_eq!(
        screenshot_entries[0].file_name().unwrap().to_string_lossy(),
        expected_screenshot_name
    );
    fs::remove_dir_all(&screenshot_dir).unwrap();
}

#[test]
#[should_panic(expected = "unknown viewpoint 'dunes'")]
fn screenshot_app_panics_with_available_viewpoints_for_an_unknown_name() {
    let screenshot_dir = create_temp_screenshot_dir("unknown-viewpoint");
    let _screenshot_app = build_screenshot_app(
        build_map01(),
        Configuration {
            viewpoint: Some("dunes".to_owned()),
            ..default()
        },
        RENDER_WIDTH,
        RENDER_HEIGHT,
        screenshot_dir.to_string_lossy().into_owned(),
        "map01",
    );
}

#[test]
fn screenshot_app_with_colliders_hides_weapons_and_enables_gizmos() {
    let screenshot_dir = create_temp_screenshot_dir("gunner-colliders");
    let mut app = build_screenshot_app(
        build_gunner_colliders_map(),
        Configuration {
            show_colliders: true,
            ..default()
        },
        COLLIDER_RENDER_WIDTH,
        COLLIDER_RENDER_HEIGHT,
        screenshot_dir.to_string_lossy().into_owned(),
        "gunner-colliders",
    );
    request_screenshot(app.world_mut());

    let captured_screenshots = wait_screenshot_capture(&mut app);
    assert_eq!(
        captured_screenshots.0.len(),
        1,
        "expected exactly one captured screenshot"
    );
    let image = &captured_screenshots.0[0];
    assert_eq!(image.width(), COLLIDER_RENDER_WIDTH, "unexpected captured width");
    assert_eq!(image.height(), COLLIDER_RENDER_HEIGHT, "unexpected captured height");
    let screenshot_entries: Vec<PathBuf> = fs::read_dir(&screenshot_dir)
        .unwrap()
        .map(|entry| entry.unwrap().path())
        .collect();
    let expected_screenshot_name = format!("{}-gunner-colliders-default.png", env!("CARGO_PKG_VERSION"));
    assert_eq!(
        screenshot_entries[0].file_name().unwrap().to_string_lossy(),
        expected_screenshot_name,
        "unexpected screenshot file name"
    );

    let world = app.world_mut();
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
    let (physics_gizmos_config, physics_gizmos) = gizmo_configs.config::<PhysicsGizmos>();
    assert!(
        physics_gizmos_config.enabled,
        "screenshot app with colliders must enable collider gizmos"
    );
    let expected_collider_color = Color::srgb(1.0, 0.2, 0.2);
    assert_eq!(
        physics_gizmos.collider_color,
        Some(expected_collider_color),
        "collider gizmos must be red"
    );
    let expected_joint_color = Color::srgb(0.2, 0.2, 1.0);
    assert_eq!(
        physics_gizmos.joint_anchor_color,
        Some(expected_joint_color),
        "joint anchor gizmos must be blue"
    );
    assert_eq!(
        physics_gizmos.joint_separation_color,
        Some(expected_joint_color),
        "joint separation gizmos must be blue"
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
