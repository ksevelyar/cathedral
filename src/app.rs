use std::fs::File;
use std::io::{self, Write};
use std::sync::{Arc, Mutex, MutexGuard};
use std::time::Duration;

use avian3d::prelude::PhysicsPlugins;
use bevy::app::{PluginGroup, SubApps};
use bevy::asset::RenderAssetUsages;
use bevy::audio::AudioPlugin;
use bevy::camera::{Exposure, RenderTarget};
use bevy::diagnostic::FrameCount;
use bevy::gltf::GltfPlugin;
use bevy::image::{Image, ImageAddressMode, ImageSamplerDescriptor};
use bevy::input::InputPlugin;
use bevy::log::{
    LogPlugin,
    tracing_subscriber::{
        self, EnvFilter, fmt::MakeWriter, fmt::format::Writer, fmt::time::FormatTime, layer::SubscriberExt,
        util::SubscriberInitExt,
    },
};
use jiff::Zoned;

struct LocalTime;

impl FormatTime for LocalTime {
    fn format_time(&self, writer: &mut Writer<'_>) -> std::fmt::Result {
        write!(writer, "{}", Zoned::now().strftime("%Y-%m-%d %H:%M:%S%.1f"))
    }
}

struct LogFile(Arc<Mutex<Option<File>>>);

impl LogFile {
    fn open() -> Self {
        Self(Arc::new(Mutex::new(File::create("last-run.log").ok())))
    }
}

impl<'a> MakeWriter<'a> for LogFile {
    type Writer = LogFileLine<'a>;

    fn make_writer(&'a self) -> Self::Writer {
        let file_guard = self.0.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
        LogFileLine(file_guard)
    }
}

struct LogFileLine<'a>(MutexGuard<'a, Option<File>>);

impl Write for LogFileLine<'_> {
    fn write(&mut self, buffer: &[u8]) -> io::Result<usize> {
        match self.0.as_mut() {
            Some(file) => file.write(buffer),
            None => Ok(buffer.len()),
        }
    }

    fn flush(&mut self) -> io::Result<()> {
        match self.0.as_mut() {
            Some(file) => file.flush(),
            None => Ok(()),
        }
    }
}

fn make_default_filter() -> EnvFilter {
    EnvFilter::new(
        "info,wgpu=error,naga=warn,symphonia_bundle_mp3::demuxer=warn,symphonia_format_caf::demuxer=warn,symphonia_format_isompf4::demuxer=warn,symphonia_format_ogg::demuxer=warn,symphonia_format_riff::demuxer=warn,symphonia_format_wav::demuxer=warn,calloop::loop_logic=error,calloop::sources=debug",
    )
}

fn initialize_logging() {
    let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| make_default_filter());
    let log_file = LogFile::open();
    tracing_subscriber::registry()
        .with(filter)
        .with(
            tracing_subscriber::fmt::layer()
                .with_timer(LocalTime)
                .with_ansi(true)
                .with_writer(std::io::stdout),
        )
        .with(
            tracing_subscriber::fmt::layer()
                .with_timer(LocalTime)
                .with_ansi(false)
                .with_writer(log_file),
        )
        .init();
}
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

fn make_tiling_image_sampler() -> ImageSamplerDescriptor {
    ImageSamplerDescriptor {
        address_mode_u: ImageAddressMode::Repeat,
        address_mode_v: ImageAddressMode::Repeat,
        ..ImageSamplerDescriptor::linear()
    }
    .set_anisotropic_filter(8)
    .clone()
}

pub fn build_game_app() -> App {
    initialize_logging();
    let mut app = App::new();
    app.add_plugins((
        DefaultPlugins.build().disable::<LogPlugin>().set(ImagePlugin {
            default_sampler: make_tiling_image_sampler(),
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
            .build()
            .disable::<LogPlugin>()
            .set(window_plugin)
            .set(render_plugin)
            .disable::<WinitPlugin>()
            .set(ImagePlugin {
                default_sampler: make_tiling_image_sampler(),
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
    pub fn create_render_target(&mut self, width: u32, height: u32) -> RenderTarget {
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
    let Some((eye, look_target)) = compute_viewpoint(&viewpoints, frame_count.0) else {
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

fn compute_viewpoint(viewpoints: &Viewpoints, frame: u32) -> Option<(Vec3, Vec3)> {
    if frame + 1 < FIRST_CAPTURE_FRAME {
        return None;
    }
    let frame_after_warmup = frame.checked_sub(FIRST_CAPTURE_FRAME - 1)?;
    let viewpoint_index = frame_after_warmup / FRAMES_BETWEEN_CAPTURES;
    viewpoints.0.get(viewpoint_index as usize).copied()
}
