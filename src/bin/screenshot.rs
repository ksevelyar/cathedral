use cathedral::app::{build_screenshot_app, fetch_configuration, run_until_screenshot_captured};
use cathedral::maps::CurrentMap;

const SCREENSHOT_DIR: &str = "screenshots";
const RENDER_WIDTH: u32 = 3440;
const RENDER_HEIGHT: u32 = 1440;
const COLLIDER_RENDER_WIDTH: u32 = 1440;
const COLLIDER_RENDER_HEIGHT: u32 = 1440;

fn main() {
    let configuration = fetch_configuration();
    let show_colliders = configuration.show_colliders;
    std::fs::create_dir_all(SCREENSHOT_DIR).unwrap();
    let (map, render_width, render_height, screenshot_name) = if show_colliders {
        (
            CurrentMap::ColliderInspection,
            COLLIDER_RENDER_WIDTH,
            COLLIDER_RENDER_HEIGHT,
            "collider-inspection",
        )
    } else {
        (CurrentMap::Map01, RENDER_WIDTH, RENDER_HEIGHT, "map01")
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
