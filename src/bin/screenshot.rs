use std::env;
use std::process;

use bevy::prelude::*;
use cathedral::app::{ScreenshotOptions, build_screenshot_app};
use cathedral::maps::CurrentMap;

const SCREENSHOT_DIR: &str = "screenshots";
const RENDER_WIDTH: u32 = 3440;
const RENDER_HEIGHT: u32 = 1440;
const COLLIDER_RENDER_WIDTH: u32 = 1440;
const COLLIDER_RENDER_HEIGHT: u32 = 1440;
const MUZZLE_FLASH_RENDER_WIDTH: u32 = 1440;
const MUZZLE_FLASH_RENDER_HEIGHT: u32 = 1440;

fn main() {
    let arguments: Vec<String> = env::args().skip(1).collect();
    std::fs::create_dir_all(SCREENSHOT_DIR).unwrap();
    let mut app = match arguments.as_slice() {
        [argument] if argument == "--colliders" => build_screenshot_app(
            CurrentMap::ColliderInspection,
            None,
            COLLIDER_RENDER_WIDTH,
            COLLIDER_RENDER_HEIGHT,
            SCREENSHOT_DIR.to_owned(),
            ScreenshotOptions {
                screenshot_name: "collider-inspection",
                show_colliders: true,
                show_revolver_muzzle_flash: false,
            },
        ),
        [argument] if argument == "--muzzle-flash" => build_screenshot_app(
            CurrentMap::Map01,
            None,
            MUZZLE_FLASH_RENDER_WIDTH,
            MUZZLE_FLASH_RENDER_HEIGHT,
            SCREENSHOT_DIR.to_owned(),
            ScreenshotOptions {
                screenshot_name: "revolver-muzzle-flash",
                show_colliders: false,
                show_revolver_muzzle_flash: true,
            },
        ),
        _ => {
            let viewpoint = parse_viewpoint_arguments(&mut arguments.into_iter());
            build_screenshot_app(
                CurrentMap::Map01,
                viewpoint,
                RENDER_WIDTH,
                RENDER_HEIGHT,
                SCREENSHOT_DIR.to_owned(),
                ScreenshotOptions {
                    screenshot_name: "map01",
                    show_colliders: false,
                    show_revolver_muzzle_flash: false,
                },
            )
        }
    };
    app.run_until_screenshot_captured();
}

fn parse_viewpoint_arguments(arguments: &mut dyn Iterator<Item = String>) -> Option<(Vec3, Vec3)> {
    let argument = arguments.next()?;
    let extra = arguments.next();
    let viewpoint = match parse_viewpoint(&argument) {
        Ok(viewpoint) => viewpoint,
        Err(error) => {
            eprintln!("{error}");
            process::exit(1);
        }
    };
    if let Some(extra) = extra {
        eprintln!("expected at most one viewpoint argument, got a second: {extra}");
        process::exit(1);
    }
    Some(viewpoint)
}

fn parse_viewpoint(argument: &str) -> Result<(Vec3, Vec3), String> {
    let Some((eye, look_target)) = argument.split_once(':') else {
        return Err(format!("expected eye:look, got {argument}"));
    };
    let eye = parse_position(eye, argument)?;
    let look_target = parse_position(look_target, argument)?;
    Ok((eye, look_target))
}

fn parse_position(text: &str, argument: &str) -> Result<Vec3, String> {
    let coordinates = text
        .split(',')
        .map(|coordinate| coordinate.trim().parse::<f32>())
        .collect::<Result<Vec<_>, _>>()
        .map_err(|_| format!("expected comma separated numbers, got {argument}"))?;
    let [x, y, z]: [f32; 3] = coordinates
        .try_into()
        .map_err(|_| format!("expected x,y,z, got {argument}"))?;
    Ok(Vec3::new(x, y, z))
}
