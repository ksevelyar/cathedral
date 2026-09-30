use std::fs::File;
use std::sync::Arc;
use std::time::Duration;

use avian3d::prelude::PhysicsPlugins;
use bevy::app::SubApps;
use bevy::asset::RenderAssetUsages;
use bevy::audio::AudioPlugin;
use bevy::camera::{Exposure, RenderTarget};
use bevy::diagnostic::FrameCount;
use bevy::gltf::GltfPlugin;
use bevy::image::{Image, ImageAddressMode, ImageSamplerDescriptor};
use bevy::input::InputPlugin;
use bevy::log::{BoxedLayer, LogPlugin};
use bevy::mesh::MeshPlugin;
use bevy::prelude::*;
use bevy::render::{
    RenderPlugin,
    render_resource::{Extent3d, PollType, TextureDimension, TextureFormat, TextureUsages},
    renderer::RenderDevice,
    view::screenshot::{Screenshot, save_to_disk},
};
use bevy::state::app::StatesPlugin;
use bevy::time::TimeUpdateStrategy;
use bevy::window::ExitCondition;
use bevy::winit::WinitPlugin;
use bevy::world_serialization::WorldSerializationPlugin;

use crate::{
    GamePlugin,
    enemies::EnemiesPlugin,
    maps::{CurrentMap, MapsPlugin, PlayerStartOverride},
    player::PlayerPlugin,
    ragdoll::RagdollPlugin,
    shooting::ShootingPlugin,
    state::GameStatePlugin,
};

const FIRST_CAPTURE_FRAME: u32 = 16;
const FRAMES_BETWEEN_CAPTURES: u32 = 3;

fn tiling_image_sampler() -> ImageSamplerDescriptor {
    ImageSamplerDescriptor {
        address_mode_u: ImageAddressMode::Repeat,
        address_mode_v: ImageAddressMode::Repeat,
        ..ImageSamplerDescriptor::linear()
    }
    .set_anisotropic_filter(8)
    .clone()
}

fn log_file_layer() -> Option<BoxedLayer> {
    let file = File::create("last-run.log").ok()?;
    Some(Box::new(
        bevy::log::tracing_subscriber::fmt::layer()
            .with_writer(Arc::new(file))
            .with_ansi(false),
    ))
}

pub fn build_game_app() -> App {
    let mut app = App::new();
    app.add_plugins((
        DefaultPlugins
            .set(LogPlugin {
                custom_layer: |_| log_file_layer(),
                ..default()
            })
            .set(ImagePlugin {
                default_sampler: tiling_image_sampler(),
            }),
        GamePlugin,
    ));
    app
}

pub fn build_headless_app(map: CurrentMap, player_start: Option<Vec3>) -> App {
    let mut app = App::new();
    app.insert_resource(map);
    if let Some(player_start) = player_start {
        app.insert_resource(PlayerStartOverride(player_start));
    }
    app.add_plugins((
        MinimalPlugins,
        TransformPlugin,
        AssetPlugin::default(),
        InputPlugin,
        AudioPlugin::default(),
        WorldSerializationPlugin,
        MeshPlugin,
        AnimationPlugin,
        GltfPlugin::default(),
        PhysicsPlugins::default(),
        StatesPlugin,
    ));
    app.add_plugins((
        GameStatePlugin,
        MapsPlugin,
        PlayerPlugin,
        EnemiesPlugin,
        RagdollPlugin,
        ShootingPlugin,
    ))
    .init_asset::<StandardMaterial>()
    .init_asset::<Image>()
    .insert_resource(GizmoConfigStore::default())
    .insert_resource(TimeUpdateStrategy::ManualDuration(Duration::from_millis(16)))
    .add_systems(Startup, crate::player::setup_player);
    app.finish();
    app.cleanup();
    app
}

pub struct ScreenshotApp(SubApps);

pub fn build_screenshot_app(viewpoints: Vec<(Vec3, Vec3)>) -> ScreenshotApp {
    let render_plugin = RenderPlugin {
        synchronous_pipeline_compilation: true,
        ..default()
    };
    let window_plugin = WindowPlugin {
        primary_window: None,
        exit_condition: ExitCondition::DontExit,
        ..default()
    };

    let mut app = App::new();
    app.add_plugins(
        DefaultPlugins
            .set(window_plugin)
            .set(render_plugin)
            .disable::<WinitPlugin>()
            .set(ImagePlugin {
                default_sampler: tiling_image_sampler(),
            }),
    )
    .add_plugins(GamePlugin)
    .insert_resource(Viewpoints(viewpoints))
    .add_systems(Update, (aim_camera, capture_screenshots));

    app.finish();
    app.cleanup();

    ScreenshotApp(std::mem::take(app.sub_apps_mut()))
}

impl ScreenshotApp {
    pub fn new_render_target(&mut self, width: u32, height: u32) -> RenderTarget {
        let mut target = Image::new_uninit(
            Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
            TextureDimension::D2,
            TextureFormat::Rgba8UnormSrgb,
            RenderAssetUsages::RENDER_WORLD,
        );
        target.texture_descriptor.usage |= TextureUsages::RENDER_ATTACHMENT;
        self.0
            .main
            .world_mut()
            .resource_mut::<Assets<Image>>()
            .add(target)
            .into()
    }

    pub fn spawn_camera(&mut self, target: RenderTarget) {
        let image_handle = target.as_image().unwrap().clone();
        self.0.main.world_mut().spawn((
            ScreenshotCamera,
            Camera3d::default(),
            Exposure::INDOOR,
            target,
            Transform::from_xyz(0.0, 1.9, 6.0),
        ));
        self.0.main.world_mut().insert_resource(CaptureTarget(image_handle));
    }

    pub fn update(&mut self) {
        self.0.update();
        self.0
            .main
            .world()
            .resource::<RenderDevice>()
            .wgpu_device()
            .poll(PollType::Wait {
                submission_index: None,
                timeout: None,
            })
            .unwrap();
    }
}

#[derive(Resource)]
struct Viewpoints(Vec<(Vec3, Vec3)>);

#[derive(Resource)]
struct CaptureTarget(Handle<Image>);

#[derive(Component)]
struct ScreenshotCamera;

fn aim_camera(
    frame_count: Res<FrameCount>,
    viewpoints: Res<Viewpoints>,
    mut camera: Query<&mut Transform, (With<Camera3d>, With<ScreenshotCamera>)>,
) {
    let Ok(mut transform) = camera.single_mut() else {
        return;
    };
    let Some((eye, look_target)) = viewpoint(&viewpoints, frame_count.0) else {
        return;
    };
    eprintln!("aiming frame {} at {eye:?} -> {look_target:?}", frame_count.0);
    *transform = Transform::from_translation(eye).looking_at(look_target, Vec3::Y);
}

fn capture_screenshots(
    frame_count: Res<FrameCount>,
    viewpoints: Res<Viewpoints>,
    capture_target: Res<CaptureTarget>,
    mut commands: Commands,
) {
    let Some(capture_index) = frame_count
        .0
        .checked_sub(FIRST_CAPTURE_FRAME)
        .and_then(|frame| frame.checked_div(FRAMES_BETWEEN_CAPTURES))
    else {
        return;
    };
    if frame_count.0 % FRAMES_BETWEEN_CAPTURES != FIRST_CAPTURE_FRAME % FRAMES_BETWEEN_CAPTURES
        || capture_index >= viewpoints.0.len() as u32
    {
        return;
    }
    eprintln!("scheduling screenshot {capture_index}");
    commands
        .spawn(Screenshot::image(capture_target.0.clone()))
        .observe(save_to_disk(format!("/tmp/cathedral_view{capture_index}.png")));
}

fn viewpoint(viewpoints: &Viewpoints, frame: u32) -> Option<(Vec3, Vec3)> {
    if frame + 1 < FIRST_CAPTURE_FRAME {
        return None;
    }
    let frame_after_warmup = frame.checked_sub(FIRST_CAPTURE_FRAME - 1)?;
    let viewpoint_index = frame_after_warmup / FRAMES_BETWEEN_CAPTURES;
    viewpoints.0.get(viewpoint_index as usize).copied()
}
