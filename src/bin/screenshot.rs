use cathedral::app::{build_screenshot_app, fetch_configuration, request_screenshot, wait_screenshot_capture};
use cathedral::enemies::AnimationState;
use cathedral::enemies::{Dying, EnemyActivity, EnemyBehavior, EnemyKind};
use cathedral::maps::{Map, MapBuilder, build_map01};
use cathedral::player::{Player, PlayerCamera, compute_look_angles};
use cathedral::ragdoll::{OwnedByCharacter, RagdollBodyPart};

use avian3d::prelude::{Position, RigidBodyDisabled};
use bevy::input::ButtonState;
use bevy::input::mouse::{MouseButton as InputMouseButton, MouseButtonInput};
use bevy::math::EulerRot;
use bevy::prelude::*;
use bevy::time::TimeUpdateStrategy;
use bevy::world_serialization::WorldAssetRoot;

const SCREENSHOT_DIR: &str = "screenshots";
const RENDER_WIDTH: u32 = 3440;
const RENDER_HEIGHT: u32 = 1440;
const COLLIDER_RENDER_WIDTH: u32 = 1440;
const COLLIDER_RENDER_HEIGHT: u32 = 1440;

#[derive(Clone, Copy)]
enum ScreenshotScenario {
    Map01,
    GunnerColliders,
    GunnersRagdoll,
}

impl ScreenshotScenario {
    fn parse(argument: Option<String>) -> Self {
        match argument.as_deref().unwrap_or("map01") {
            "map01" => Self::Map01,
            "gunner-colliders" => Self::GunnerColliders,
            "gunners-ragdoll" => Self::GunnersRagdoll,
            scenario => panic!("unknown screenshot scenario: {scenario}"),
        }
    }
}

fn parse_screenshot_scenario() -> ScreenshotScenario {
    let scenario_argument = std::env::args().skip(1).find_map(|argument| {
        argument
            .strip_prefix("scenario=")
            .or_else(|| argument.strip_prefix("--scenario="))
            .map(str::to_owned)
    });
    ScreenshotScenario::parse(scenario_argument)
}

fn build_gunner_colliders_map() -> Map {
    let name = "gunner-colliders";
    let floor_size = Vec3::new(60.0, 0.1, 60.0);
    let player_position = Vec3::new(0.0, 1.8, 3.0);
    let player_look_at = Vec3::new(0.0, 1.0, 0.0);
    MapBuilder::new(name)
        .with_player_position(player_position, player_look_at)
        .with_floor(floor_size)
        .with_enemy(
            vec![Vec3::ZERO],
            Vec3::new(0.0, 0.0, 5.0),
            EnemyKind::Gunner,
            EnemyBehavior::Patrol,
        )
        .build()
}

fn build_gunners_ragdoll_map() -> Map {
    let name = "gunners-ragdoll";
    let floor_size = Vec3::new(60.0, 0.1, 60.0);
    let player_position = Vec3::new(0.0, 1.8, 5.0);
    let player_look_at = Vec3::new(0.0, 1.2, 0.0);
    MapBuilder::new(name)
        .with_player_position(player_position, player_look_at)
        .with_floor(floor_size)
        .with_enemy(
            vec![Vec3::new(-0.75, 0.0, 0.0)],
            Vec3::new(0.0, 0.0, 5.0),
            EnemyKind::Gunner,
            EnemyBehavior::Patrol,
        )
        .with_enemy(
            vec![Vec3::new(0.75, 0.0, 0.0)],
            Vec3::new(0.0, 0.0, 5.0),
            EnemyKind::Gunner,
            EnemyBehavior::Patrol,
        )
        .build()
}

fn main() {
    let configuration = fetch_configuration();
    let screenshot_scenario = parse_screenshot_scenario();
    std::fs::create_dir_all(SCREENSHOT_DIR).unwrap();
    match screenshot_scenario {
        ScreenshotScenario::Map01 => {
            run_regular_screenshot(build_map01(), configuration, RENDER_WIDTH, RENDER_HEIGHT, "map01");
        }
        ScreenshotScenario::GunnerColliders => run_regular_screenshot(
            build_gunner_colliders_map(),
            configuration,
            COLLIDER_RENDER_WIDTH,
            COLLIDER_RENDER_HEIGHT,
            "gunner-colliders",
        ),
        ScreenshotScenario::GunnersRagdoll => run_gunners_ragdoll_screenshot(configuration),
    }
}

fn run_regular_screenshot(
    map: Map,
    configuration: cathedral::app::Configuration,
    render_width: u32,
    render_height: u32,
    screenshot_name: &'static str,
) {
    let mut app = build_screenshot_app(
        map,
        configuration,
        render_width,
        render_height,
        SCREENSHOT_DIR.to_owned(),
        screenshot_name,
    );
    request_screenshot(app.world_mut());
    wait_screenshot_capture(&mut app);
}

fn run_gunners_ragdoll_screenshot(configuration: cathedral::app::Configuration) {
    let mut app = build_screenshot_app(
        build_gunners_ragdoll_map(),
        configuration,
        COLLIDER_RENDER_WIDTH,
        COLLIDER_RENDER_HEIGHT,
        SCREENSHOT_DIR.to_owned(),
        "gunners-ragdoll",
    );
    make_simulation_time_deterministic(&mut app);
    let (left_gunner, right_gunner) = wait_for_gunners_readiness(&mut app);
    wait_for_shooting_pose(&mut app, left_gunner);
    let right_gunner_head = find_ragdoll_body(&mut app, right_gunner, RagdollBodyPart::Head);
    aim_player_at(&mut app, right_gunner_head);
    fire_player_weapon(&mut app);
    wait_for_ragdoll_activation(&mut app, right_gunner, right_gunner_head);
    advance_ragdoll_frames(&mut app, 10);
    request_screenshot(app.world_mut());
    wait_screenshot_capture(&mut app);
}

fn wait_for_ragdoll_activation(app: &mut App, gunner: Entity, head: Entity) {
    let maximum_activation_updates = 10_000;
    for _ in 0..maximum_activation_updates {
        app.update();
        let gunner_is_dying = app.world().get::<Dying>(gunner).is_some();
        let head_is_active = app.world().get::<RigidBodyDisabled>(head).is_none();
        if gunner_is_dying && head_is_active {
            return;
        }
        std::thread::yield_now();
    }
    panic!("gunner ragdoll should activate within {maximum_activation_updates} updates");
}

fn advance_ragdoll_frames(app: &mut App, frame_count: usize) {
    for _ in 0..frame_count {
        app.update();
    }
}

fn make_simulation_time_deterministic(app: &mut App) {
    let frame_duration = core::time::Duration::from_millis(16);
    app.insert_resource(Time::<Fixed>::from_duration(frame_duration));
    app.insert_resource(TimeUpdateStrategy::ManualDuration(frame_duration));
}

type GunnerPair = (Entity, Entity);

fn wait_for_gunners_readiness(app: &mut App) -> GunnerPair {
    let maximum_warmup_updates = 10_000;
    let required_stable_updates = 64;
    let mut stable_updates = 0;
    for _ in 0..maximum_warmup_updates {
        app.update();
        let gunners = find_gunner_entities(app);
        let readiness = gunners
            .iter()
            .map(|gunner| count_gunner_readiness(app, *gunner))
            .sum::<usize>();
        stable_updates = if readiness == gunners.len() * 2 {
            stable_updates + 1
        } else {
            0
        };
        if stable_updates == required_stable_updates {
            let gunner_positions = gunners
                .iter()
                .map(|gunner| app.world().get::<Transform>(*gunner).unwrap().translation)
                .collect::<Vec<_>>();
            let right_gunner = gunner_positions
                .iter()
                .enumerate()
                .max_by(|(_, left_position), (_, right_position)| left_position.x.total_cmp(&right_position.x))
                .map(|(gunner_index, _)| gunner_index)
                .unwrap();
            let left_gunner_index = 1 - right_gunner;
            return (gunners[left_gunner_index], gunners[right_gunner]);
        }
        std::thread::yield_now();
    }
    panic!("gunners should load hair and ragdoll colliders within {maximum_warmup_updates} updates");
}

fn count_gunner_readiness(app: &mut App, gunner: Entity) -> usize {
    let hair_is_loaded = app.world().get::<Children>(gunner).is_some_and(|children| {
        children
            .iter()
            .any(|child| app.world().get::<WorldAssetRoot>(child).is_some())
    });
    let head_exists = find_ragdoll_body_in(app, gunner, RagdollBodyPart::Head).is_some();
    usize::from(hair_is_loaded) + usize::from(head_exists)
}

fn find_gunner_entities(app: &mut App) -> Vec<Entity> {
    app.world_mut()
        .query_filtered::<Entity, With<EnemyActivity>>()
        .iter(app.world())
        .collect()
}

fn wait_for_shooting_pose(app: &mut App, gunner: Entity) {
    let maximum_pose_updates = 10_000;
    for _ in 0..maximum_pose_updates {
        app.update();
        let gunner_is_attacking = app
            .world()
            .get::<EnemyActivity>(gunner)
            .is_some_and(|activity| activity.state == AnimationState::Attacking);
        if gunner_is_attacking {
            return;
        }
        std::thread::yield_now();
    }
    panic!("gunner should enter the attacking pose within {maximum_pose_updates} updates");
}

fn find_ragdoll_body(app: &mut App, gunner: Entity, body_part: RagdollBodyPart) -> Entity {
    find_ragdoll_body_in(app, gunner, body_part)
        .unwrap_or_else(|| panic!("gunner {gunner:?} should have a {body_part:?} ragdoll body"))
}

fn find_ragdoll_body_in(app: &mut App, gunner: Entity, body_part: RagdollBodyPart) -> Option<Entity> {
    app.world_mut()
        .query::<(Entity, &OwnedByCharacter, &RagdollBodyPart)>()
        .iter(app.world())
        .find(|(_, body_owner, part)| body_owner.get_owner() == gunner && **part == body_part)
        .map(|(body_entity, _, _)| body_entity)
}

fn aim_player_at(app: &mut App, target_body: Entity) {
    let target_position = app
        .world()
        .get::<Position>(target_body)
        .expect("target body position")
        .0;
    let mut player_transform = app
        .world_mut()
        .query_filtered::<&mut Transform, With<Player>>()
        .single_mut(app.world_mut())
        .expect("one player should spawn");
    let (yaw, pitch) = compute_look_angles(player_transform.translation, target_position);
    player_transform.rotation = Quat::from_euler(EulerRot::YXZ, yaw, 0.0, 0.0);
    let mut camera_transform = app
        .world_mut()
        .query_filtered::<&mut Transform, With<PlayerCamera>>()
        .single_mut(app.world_mut())
        .expect("one player camera should spawn");
    camera_transform.rotation = Quat::from_euler(EulerRot::YXZ, 0.0, pitch, 0.0);
}

fn fire_player_weapon(app: &mut App) {
    let pressed_message = MouseButtonInput {
        button: InputMouseButton::Left,
        state: ButtonState::Pressed,
        window: Entity::PLACEHOLDER,
    };
    let released_message = MouseButtonInput {
        button: InputMouseButton::Left,
        state: ButtonState::Released,
        window: Entity::PLACEHOLDER,
    };
    app.world_mut()
        .resource_mut::<Messages<MouseButtonInput>>()
        .write(pressed_message);
    app.update();
    app.world_mut()
        .resource_mut::<Messages<MouseButtonInput>>()
        .write(released_message);
    app.update();
}
