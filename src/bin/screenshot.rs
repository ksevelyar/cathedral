use cathedral::app::{build_screenshot_app, fetch_configuration, run_until_screenshot_captured};
use cathedral::maps::{Map, MapBuilder, build_map01};

use bevy::prelude::*;
use cathedral::enemies::EnemyKind;

const SCREENSHOT_DIR: &str = "screenshots";
const RENDER_WIDTH: u32 = 3440;
const RENDER_HEIGHT: u32 = 1440;
const COLLIDER_RENDER_WIDTH: u32 = 1440;
const COLLIDER_RENDER_HEIGHT: u32 = 1440;

fn build_collider_inspection_map() -> Map {
    let name = "collider-inspection";
    let floor_size = Vec3::new(60.0, 0.1, 60.0);
    MapBuilder::new(name)
        .with_player_position(Vec3::new(0.0, 1.8, 3.0), Vec3::new(0.0, 1.0, 0.0))
        .with_floor(floor_size)
        .with_enemy(Vec3::ZERO, EnemyKind::Fighter)
        .build()
}

fn main() {
    let configuration = fetch_configuration();
    let show_colliders = configuration.show_colliders;
    std::fs::create_dir_all(SCREENSHOT_DIR).unwrap();
    let (map, render_width, render_height, screenshot_name) = if show_colliders {
        (
            build_collider_inspection_map(),
            COLLIDER_RENDER_WIDTH,
            COLLIDER_RENDER_HEIGHT,
            "collider-inspection",
        )
    } else {
        (build_map01(), RENDER_WIDTH, RENDER_HEIGHT, "map01")
    };
    let mut app = build_screenshot_app(
        map,
        configuration,
        render_width,
        render_height,
        SCREENSHOT_DIR.to_owned(),
        screenshot_name,
    );
    run_until_screenshot_captured(&mut app);
}
