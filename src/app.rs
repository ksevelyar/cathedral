use std::collections::HashMap;
use std::env;
use std::fs::File;
use std::io::{self, Write};
use std::sync::{Arc, Mutex, MutexGuard};
use std::time::{Duration, Instant};

use avian3d::prelude::{PhysicsPlugins, SubstepCount};
use bevy::app::PluginGroup;
use bevy::asset::RenderAssetUsages;
use bevy::audio::{DefaultSpatialScale, SpatialScale};
use bevy::camera::RenderTarget;
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
        let file_guard = self.0.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
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
        .try_init()
        .ok();
}
use bevy::ecs::system::RunSystemOnce;
use bevy::mesh::MeshPlugin;
use bevy::prelude::*;
use bevy::render::{
    RenderPlugin,
    render_resource::{Extent3d, PollType, TextureDimension, TextureFormat, TextureUsages},
    renderer::RenderDevice,
    view::screenshot::{Screenshot, ScreenshotCaptured, save_to_disk},
};
use bevy::state::app::StatesPlugin;
use bevy::time::TimeUpdateStrategy;
use bevy::window::ExitCondition;
use bevy::winit::WinitPlugin;
use bevy::world_serialization::WorldSerializationPlugin;

use crate::{
    bloodmarks::BloodmarksPlugin,
    enemies::EnemiesPlugin,
    health::HealthPlugin,
    maps::{Map, MapsPlugin},
    player::{PlayerCamera, PlayerPlugin},
    ragdoll::RagdollPlugin,
    shooting::ShootingPlugin,
    state::{ColliderInspection, GameState, GameStatePlugin},
    ui::{ColliderInspectionPlugin, PlayerHudPlugin},
};

#[derive(Resource, Default)]
pub struct Configuration {
    pub show_fps: bool,
    pub show_colliders: bool,
    pub viewpoint: Option<(Vec3, Vec3)>,
}

pub fn fetch_configuration() -> Configuration {
    let values = env::args()
        .skip(1)
        .filter_map(|argument| {
            argument
                .strip_prefix("--")
                .and_then(|named_argument| named_argument.split_once('='))
                .map(|(name, value)| (name.to_owned(), value.to_owned()))
        })
        .collect::<HashMap<_, _>>();

    Configuration {
        show_fps: values.get("fps").is_some_and(|value| value.parse().unwrap()),
        show_colliders: values.get("colliders").is_some_and(|value| value.parse().unwrap()),
        viewpoint: values.get("viewpoint").map(|value| parse_viewpoint(value)),
    }
}

fn parse_viewpoint(value: &str) -> (Vec3, Vec3) {
    let coordinates = value
        .split([':', ','])
        .map(str::parse)
        .collect::<Result<Vec<f32>, _>>()
        .unwrap();

    (Vec3::from_slice(&coordinates[..3]), Vec3::from_slice(&coordinates[3..]))
}

fn make_tiling_image_sampler() -> ImageSamplerDescriptor {
    ImageSamplerDescriptor {
        address_mode_u: ImageAddressMode::Repeat,
        address_mode_v: ImageAddressMode::Repeat,
        ..ImageSamplerDescriptor::linear()
    }
    .set_anisotropic_filter(8)
    .clone()
}

pub fn build_gui_app(configuration: Configuration, map: Map) -> App {
    initialize_logging();
    let spatial_scale = SpatialScale::new(0.1);
    let mut app = App::new();
    app.insert_resource(configuration);
    app.insert_resource(map);
    app.add_plugins(DefaultPlugins.build().disable::<LogPlugin>().set(ImagePlugin {
        default_sampler: make_tiling_image_sampler(),
    }));
    app.add_plugins((
        PhysicsPlugins::default(),
        GameStatePlugin,
        MapsPlugin,
        HealthPlugin,
        BloodmarksPlugin,
        PlayerPlugin,
        EnemiesPlugin,
        RagdollPlugin,
        ShootingPlugin,
        PlayerHudPlugin,
        ColliderInspectionPlugin,
    ))
    .insert_resource(SubstepCount(30))
    .insert_resource(DefaultSpatialScale(spatial_scale))
    .add_systems(Startup, crate::player::setup_player);
    app
}

pub fn build_test_app(map: Map) -> App {
    let spatial_scale = SpatialScale::new(0.1);
    let mut app = App::new();
    app.insert_resource(map);
    app.add_plugins((
        MinimalPlugins,
        TransformPlugin,
        AssetPlugin::default(),
        InputPlugin,
        WorldSerializationPlugin,
        MeshPlugin,
        AnimationPlugin,
        GltfPlugin::default(),
        PhysicsPlugins::default(),
        StatesPlugin,
    ));
    app.insert_resource(SubstepCount(30));
    app.init_asset::<StandardMaterial>().init_asset::<Image>();
    app.add_plugins((
        GameStatePlugin,
        MapsPlugin,
        HealthPlugin,
        BloodmarksPlugin,
        PlayerPlugin,
        EnemiesPlugin,
        RagdollPlugin,
        ShootingPlugin,
    ))
    .insert_resource(GizmoConfigStore::default())
    .insert_resource(DefaultSpatialScale(spatial_scale))
    .insert_resource(TimeUpdateStrategy::ManualDuration(Duration::from_millis(16)))
    .add_systems(Startup, crate::player::setup_player);
    app.finish();
    app.cleanup();
    wait_initialization(&mut app);
    app
}

fn wait_initialization(app: &mut App) {
    let maximum_initialization_updates = 10_000;
    for _ in 0..maximum_initialization_updates {
        app.update();
        if matches!(app.world().resource::<State<GameState>>().get(), GameState::Playing) {
            return;
        }
        std::thread::yield_now();
    }
    panic!("test application should enter the playing state within {maximum_initialization_updates} updates");
}

#[derive(Resource)]
struct ScreenshotDir(String);

#[derive(Resource)]
struct ScreenshotName(&'static str);

#[derive(Resource, Default)]
pub struct CapturedScreenshots(pub Vec<Image>);

pub fn build_screenshot_app(
    map: Map,
    configuration: Configuration,
    render_width: u32,
    render_height: u32,
    screenshot_dir: String,
    screenshot_name: &'static str,
) -> App {
    initialize_logging();
    let viewpoint = configuration.viewpoint;
    let show_colliders = configuration.show_colliders;
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
    app.insert_resource(configuration);
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
    );
    app.add_plugins((
        PhysicsPlugins::default(),
        GameStatePlugin,
        MapsPlugin,
        HealthPlugin,
        BloodmarksPlugin,
        PlayerPlugin,
        EnemiesPlugin,
        RagdollPlugin,
        ShootingPlugin,
        PlayerHudPlugin,
        ColliderInspectionPlugin,
    ))
    .insert_resource(SubstepCount(30))
    .add_systems(Startup, crate::player::setup_player)
    .insert_resource(map)
    .insert_resource(ColliderInspection(show_colliders))
    .insert_resource(ScreenshotDir(screenshot_dir))
    .insert_resource(ScreenshotName(screenshot_name))
    .insert_resource(CapturedScreenshots::default())
    .insert_resource(Viewpoint(viewpoint))
    .add_systems(Update, attach_screenshot_camera);
    let render_target = create_render_target(&mut app, render_width, render_height);
    app.insert_resource(ScreenshotRenderTarget(render_target));

    app.finish();
    app.cleanup();
    wait_initialization(&mut app);

    app
}

fn create_render_target(app: &mut App, render_width: u32, render_height: u32) -> Handle<Image> {
    let mut target = Image::new_uninit(
        Extent3d {
            width: render_width,
            height: render_height,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        TextureFormat::Rgba8UnormSrgb,
        RenderAssetUsages::RENDER_WORLD,
    );
    target.texture_descriptor.usage |= TextureUsages::RENDER_ATTACHMENT;
    app.world_mut().resource_mut::<Assets<Image>>().add(target)
}

#[derive(Resource)]
struct ScreenshotRenderTarget(Handle<Image>);

fn attach_screenshot_camera(
    screenshot_target: Res<ScreenshotRenderTarget>,
    viewpoint: Res<Viewpoint>,
    player_camera: Query<Entity, (With<PlayerCamera>, Without<ScreenshotCamera>)>,
    mut commands: Commands,
) {
    let Ok(player_camera) = player_camera.single() else {
        return;
    };
    commands
        .entity(player_camera)
        .insert((ScreenshotCamera, RenderTarget::from(screenshot_target.0.clone())));
    if let Some((eye, look_target)) = viewpoint.0 {
        let direction = (look_target - eye).normalize();
        let pitch = direction.y.asin();
        let yaw = (-direction.x).atan2(-direction.z);
        let camera_rotation = Quat::from_rotation_y(yaw) * Quat::from_rotation_x(pitch);
        let camera_transform = Transform::from_translation(eye).with_rotation(camera_rotation);
        commands
            .entity(player_camera)
            .remove::<ChildOf>()
            .insert(camera_transform);
    }
}

pub fn wait_screenshot_capture(app: &mut App) -> &CapturedScreenshots {
    let screenshot_timeout = Duration::from_secs(10);
    let screenshot_deadline = Instant::now() + screenshot_timeout;
    loop {
        app.update();
        if !app.world().resource::<CapturedScreenshots>().0.is_empty() {
            break;
        }
        let remaining_timeout = screenshot_deadline
            .checked_duration_since(Instant::now())
            .unwrap_or_else(|| panic!("screenshot capture did not finish within {screenshot_timeout:?}"));
        app.world()
            .resource::<RenderDevice>()
            .wgpu_device()
            .poll(PollType::Wait {
                submission_index: None,
                timeout: Some(remaining_timeout),
            })
            .unwrap_or_else(|error| panic!("screenshot capture did not finish within {screenshot_timeout:?}: {error}"));
    }

    app.world().resource::<CapturedScreenshots>()
}

#[derive(Resource)]
struct Viewpoint(Option<(Vec3, Vec3)>);

#[derive(Component)]
struct ScreenshotCamera;

pub fn request_screenshot(world: &mut World) {
    world
        .run_system_once(spawn_screenshot_capture)
        .expect("screenshot capture system should run");
}

fn spawn_screenshot_capture(
    screenshot_dir: Res<ScreenshotDir>,
    screenshot_name: Res<ScreenshotName>,
    screenshot_target: Res<ScreenshotRenderTarget>,
    mut commands: Commands,
) {
    commands
        .spawn(Screenshot::image(screenshot_target.0.clone()))
        .observe(save_to_disk(format!(
            "{}/{}-{}.png",
            screenshot_dir.0,
            env!("CARGO_PKG_VERSION"),
            screenshot_name.0
        )))
        .observe(collect_captured_screenshot);
}

fn collect_captured_screenshot(screenshot_captured: On<ScreenshotCaptured>, mut captured: ResMut<CapturedScreenshots>) {
    captured.0.push(screenshot_captured.image.clone());
}
