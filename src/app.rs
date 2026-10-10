use std::fs::File;
use std::io::{self, Write};
use std::sync::{Arc, Mutex, MutexGuard};
use std::time::Duration;

use avian3d::prelude::{PhysicsDebugPlugin, PhysicsGizmos, PhysicsPlugins, SubstepCount};
use bevy::app::{PluginGroup, SubApps};
use bevy::asset::RenderAssetUsages;
use bevy::camera::RenderTarget;
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
    view::screenshot::{Screenshot, ScreenshotCaptured, save_to_disk},
};
use bevy::state::app::StatesPlugin;
use bevy::time::TimeUpdateStrategy;
use bevy::window::ExitCondition;
use bevy::winit::WinitPlugin;
use bevy::world_serialization::WorldSerializationPlugin;
use bevy_hanabi::HanabiPlugin;

use crate::{
    enemies::EnemiesPlugin,
    maps::{CurrentMap, MapsPlugin, PlayerPosition},
    player::{CameraState, Player, PlayerPlugin},
    ragdoll::RagdollPlugin,
    shooting::{Gun, RevolverMuzzleFlashSystems, ShootingPlugin, WeaponFired},
    state::GameStatePlugin,
    ui::UiPlugin,
};

const FIRST_CAPTURE_FRAME: u32 = 200;

fn make_tiling_image_sampler() -> ImageSamplerDescriptor {
    ImageSamplerDescriptor {
        address_mode_u: ImageAddressMode::Repeat,
        address_mode_v: ImageAddressMode::Repeat,
        ..ImageSamplerDescriptor::linear()
    }
    .set_anisotropic_filter(8)
    .clone()
}

pub fn build_gui_app() -> App {
    initialize_logging();
    let mut app = App::new();
    app.add_plugins(DefaultPlugins.build().disable::<LogPlugin>().set(ImagePlugin {
        default_sampler: make_tiling_image_sampler(),
    }));
    app.add_plugins(HanabiPlugin);
    app.add_plugins((
        PhysicsPlugins::default(),
        PhysicsDebugPlugin,
        GameStatePlugin,
        MapsPlugin,
        PlayerPlugin,
        EnemiesPlugin,
        RagdollPlugin,
        ShootingPlugin,
        UiPlugin,
    ))
    .insert_resource(SubstepCount(30))
    .add_systems(Startup, crate::player::setup_player)
    .insert_gizmo_config(
        PhysicsGizmos::default(),
        GizmoConfig {
            enabled: false,
            ..default()
        },
    );
    app
}

pub fn build_test_app(map: CurrentMap, player_position: Option<PlayerPosition>) -> App {
    let mut app = App::new();
    app.insert_resource(map);
    if let Some(player_position) = player_position {
        app.insert_resource(player_position);
    }
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

pub struct ScreenshotApp(pub SubApps);

#[derive(Resource)]
struct ScreenshotDir(String);

#[derive(Resource)]
struct ScreenshotName(&'static str);

#[derive(Resource, Default)]
pub struct CapturedScreenshots(pub Vec<Image>);

pub struct ScreenshotOptions {
    pub screenshot_name: &'static str,
    pub show_colliders: bool,
    pub show_revolver_muzzle_flash: bool,
}

pub fn build_screenshot_app(
    map: CurrentMap,
    viewpoint: Option<(Vec3, Vec3)>,
    render_width: u32,
    render_height: u32,
    screenshot_dir: String,
    screenshot_options: ScreenshotOptions,
) -> ScreenshotApp {
    let render_plugin = RenderPlugin {
        synchronous_pipeline_compilation: true,
        ..default()
    };
    let window_plugin = WindowPlugin {
        primary_window: None,
        exit_condition: ExitCondition::DontExit,
        ..default()
    };
    let physics_gizmos = if screenshot_options.show_colliders {
        PhysicsGizmos::colliders(Color::srgb(1.0, 0.2, 0.2))
    } else {
        PhysicsGizmos::default()
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
    );
    app.add_plugins(HanabiPlugin);
    app.add_plugins((
        PhysicsPlugins::default(),
        PhysicsDebugPlugin,
        GameStatePlugin,
        MapsPlugin,
        PlayerPlugin,
        EnemiesPlugin,
        RagdollPlugin,
        ShootingPlugin,
        UiPlugin,
    ))
    .insert_resource(SubstepCount(30))
    .add_systems(Startup, crate::player::setup_player)
    .insert_gizmo_config(
        physics_gizmos,
        GizmoConfig {
            enabled: screenshot_options.show_colliders,
            ..default()
        },
    )
    .insert_resource(map)
    .insert_resource(ScreenshotDir(screenshot_dir))
    .insert_resource(ScreenshotName(screenshot_options.screenshot_name))
    .insert_resource(CapturedScreenshots::default())
    .insert_resource(Viewpoint(viewpoint))
    .add_systems(
        PreUpdate,
        trigger_screenshot_revolver_muzzle_flash.run_if(resource_exists::<ScreenshotRevolverMuzzleFlash>),
    )
    .add_systems(
        Update,
        (
            attach_screenshot_camera,
            capture_screenshots.after(RevolverMuzzleFlashSystems),
        ),
    );
    if screenshot_options.show_colliders {
        app.add_systems(Update, hide_screenshot_weapons);
    }
    let render_target = create_render_target(&mut app, render_width, render_height);
    app.insert_resource(ScreenshotRenderTarget(render_target));
    if screenshot_options.show_revolver_muzzle_flash {
        app.insert_resource(ScreenshotRevolverMuzzleFlash);
    }

    app.finish();
    app.cleanup();

    ScreenshotApp(std::mem::take(app.sub_apps_mut()))
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

#[derive(Resource)]
struct ScreenshotRevolverMuzzleFlash;

fn trigger_screenshot_revolver_muzzle_flash(
    frame_count: Res<FrameCount>,
    mut weapon_fired: MessageWriter<WeaponFired>,
) {
    let first_trigger_frame = FIRST_CAPTURE_FRAME - 5;
    let trigger_interval = 2;
    let trigger_frame = FIRST_CAPTURE_FRAME - 1;
    if (first_trigger_frame..=trigger_frame).contains(&frame_count.0)
        && (frame_count.0 - first_trigger_frame).is_multiple_of(trigger_interval)
    {
        weapon_fired.write(WeaponFired::revolver());
    }
}

fn attach_screenshot_camera(
    screenshot_target: Res<ScreenshotRenderTarget>,
    viewpoint: Res<Viewpoint>,
    player_camera: Query<Entity, (With<Player>, Without<ScreenshotCamera>)>,
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
        commands
            .entity(player_camera)
            .insert((CameraState { yaw, pitch }, Transform::from_translation(eye)));
    }
}

impl ScreenshotApp {
    pub fn run_until_screenshot_captured(&mut self) -> &CapturedScreenshots {
        loop {
            self.update();
            if !self.0.main.world().resource::<CapturedScreenshots>().0.is_empty() {
                break;
            }
        }

        self.0.main.world().resource::<CapturedScreenshots>()
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
struct Viewpoint(Option<(Vec3, Vec3)>);

#[derive(Component)]
struct ScreenshotCamera;

fn hide_screenshot_weapons(mut weapons: Query<&mut Visibility, With<Gun>>) {
    for mut visibility in &mut weapons {
        *visibility = Visibility::Hidden;
    }
}

fn capture_screenshots(
    frame_count: Res<FrameCount>,
    screenshot_dir: Res<ScreenshotDir>,
    screenshot_name: Res<ScreenshotName>,
    screenshot_target: Res<ScreenshotRenderTarget>,
    mut commands: Commands,
) {
    if frame_count.0 != FIRST_CAPTURE_FRAME {
        return;
    }
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
