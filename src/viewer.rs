use avian3d::prelude::{PhysicsPlugins, SubstepCount};
use bevy::input::mouse::{AccumulatedMouseMotion, MouseScrollUnit, MouseWheel};
use bevy::prelude::*;
use bevy::window::WindowPlugin;

use crate::enemies::{EnemiesPlugin, Enemy, EnemyBehavior, EnemyKind, ManualAnimationControl};
use crate::health::HealthPlugin;
use crate::maps::{MapBuilder, MapsPlugin};
use crate::ragdoll::{BoneMap, RagdollPlugin};
use crate::shooting::Weapon;
use crate::state::GameStatePlugin;
use crate::ui::{ColliderInspectionPlugin, toggle_collider_gizmos};

pub struct ViewerHudPlugin;

impl Plugin for ViewerHudPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Update,
            (
                update_orbit,
                switch_animation,
                toggle_animation_pause,
                toggle_collider_gizmos,
                update_mount_display,
            ),
        );
    }
}

#[derive(Resource)]
struct OrbitCamera {
    target: Vec3,
    yaw: f32,
    pitch: f32,
    distance: f32,
}

#[derive(Component)]
struct ViewerCamera;

#[derive(Component)]
struct MountDisplay;

#[derive(Component)]
struct ViewerAnimations {
    armature: Entity,
    nodes: Vec<AnimationNodeIndex>,
    current: usize,
    paused: bool,
}

type ViewerGunnerQuery<'world, 'state> =
    Query<'world, 'state, (Entity, &'static BoneMap), (With<Enemy>, Without<ViewerAnimations>)>;

pub fn run() {
    let map = MapBuilder::new("viewer")
        .with_floor(Vec3::new(12.0, 0.1, 12.0))
        .with_enemy(
            vec![Vec3::ZERO],
            Vec3::ZERO + Vec3::NEG_Z,
            EnemyKind::Gunner,
            EnemyBehavior::Patrol,
        )
        .build();
    App::new()
        .insert_resource(map)
        .insert_resource(OrbitCamera {
            target: Vec3::new(0.0, 0.9, 0.0),
            yaw: 0.0,
            pitch: 0.15,
            distance: 2.1,
        })
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "Cathedral viewer".to_owned(),
                ..default()
            }),
            ..default()
        }))
        .add_plugins((
            PhysicsPlugins::default(),
            GameStatePlugin,
            MapsPlugin,
            HealthPlugin,
            EnemiesPlugin,
            RagdollPlugin,
            ColliderInspectionPlugin,
            ViewerHudPlugin,
        ))
        .insert_resource(SubstepCount(30))
        .add_systems(Startup, setup_viewer)
        .add_systems(Update, (setup_animations,))
        .run();
}

fn setup_viewer(mut commands: Commands) {
    commands.spawn((
        DirectionalLight {
            illuminance: 15_000.0,
            shadow_maps_enabled: true,
            ..default()
        },
        Transform::from_rotation(Quat::from_euler(EulerRot::ZYX, 0.5, -0.7, -0.8)),
    ));
    commands.spawn((Camera3d::default(), ViewerCamera));
    commands.spawn((
        MountDisplay,
        Node {
            position_type: PositionType::Absolute,
            top: px(12),
            left: px(12),
            ..default()
        },
        Text::new("Loading viewer..."),
        TextFont {
            font_size: FontSize::Px(18.0),
            ..default()
        },
        TextColor(Color::WHITE),
    ));
}

fn setup_animations(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    gunner_bones: ViewerGunnerQuery,
    mut animation_players: Query<&mut AnimationPlayer>,
    mut graphs: ResMut<Assets<AnimationGraph>>,
) {
    let animation_asset_path = "characters/animations/ual1-standard.glb";
    let animation_count = 43;
    for (gunner, bones) in &gunner_bones {
        let Some(armature) = bones.find_bone("Armature") else {
            continue;
        };
        let Ok(mut player) = animation_players.get_mut(armature) else {
            continue;
        };
        let first_clip = asset_server.load(GltfAssetLabel::Animation(0).from_asset(animation_asset_path));
        let (mut graph, first_node) = AnimationGraph::from_clip(first_clip);
        let mut nodes = vec![first_node];
        for animation_index in 1..animation_count {
            let clip = asset_server.load(GltfAssetLabel::Animation(animation_index).from_asset(animation_asset_path));
            nodes.push(graph.add_clip(clip, 1.0, graph.root));
        }
        player.stop_all();
        player.start(first_node).repeat();
        commands
            .entity(armature)
            .insert(AnimationGraphHandle(graphs.add(graph)));
        commands.entity(gunner).insert((
            ManualAnimationControl,
            ViewerAnimations {
                armature,
                nodes,
                current: 0,
                paused: false,
            },
        ));
    }
}

fn switch_animation(
    keyboard: Res<ButtonInput<KeyCode>>,
    mut animations: Query<&mut ViewerAnimations>,
    mut players: Query<&mut AnimationPlayer>,
) {
    let direction = if keyboard.just_pressed(KeyCode::ArrowLeft) {
        Some(-1_isize)
    } else if keyboard.just_pressed(KeyCode::ArrowRight) {
        Some(1_isize)
    } else {
        None
    };
    let Some(direction) = direction else {
        return;
    };
    for mut animation in &mut animations {
        let node_count = animation.nodes.len();
        let next_index = (animation.current.cast_signed() + direction).rem_euclid(node_count.cast_signed());
        animation.current = usize::try_from(next_index).expect("animation index should stay within node count");
        let Ok(mut player) = players.get_mut(animation.armature) else {
            continue;
        };
        player.stop_all();
        let active_animation = player.start(animation.nodes[animation.current]).repeat();
        if animation.paused {
            active_animation.pause();
        }
    }
}

fn toggle_animation_pause(
    keyboard: Res<ButtonInput<KeyCode>>,
    mut animations: Query<&mut ViewerAnimations>,
    mut players: Query<&mut AnimationPlayer>,
) {
    if !keyboard.just_pressed(KeyCode::KeyP) {
        return;
    }
    for mut animation in &mut animations {
        animation.paused = !animation.paused;
        let Ok(mut player) = players.get_mut(animation.armature) else {
            continue;
        };
        if animation.paused {
            player.pause_all();
        } else {
            player.resume_all();
        }
    }
}

fn update_orbit(
    mouse_buttons: Res<ButtonInput<MouseButton>>,
    mouse_motion: Res<AccumulatedMouseMotion>,
    mut mouse_wheels: MessageReader<MouseWheel>,
    mut orbit_camera: ResMut<OrbitCamera>,
    mut cameras: Query<&mut Transform, With<ViewerCamera>>,
) {
    if mouse_buttons.pressed(MouseButton::Left) {
        let mouse_delta = mouse_motion.delta;
        orbit_camera.yaw -= mouse_delta.x * 0.01;
        orbit_camera.pitch += mouse_delta.y * 0.01;
    }
    for mouse_wheel in mouse_wheels.read() {
        let scroll_lines = match mouse_wheel.unit {
            MouseScrollUnit::Line => mouse_wheel.y,
            MouseScrollUnit::Pixel => mouse_wheel.y * 0.02,
        };
        orbit_camera.distance = (orbit_camera.distance * (-scroll_lines * 0.1).exp()).clamp(1.7, 12.0);
    }
    let minimum_height = 0.2;
    let minimum_pitch = ((minimum_height - orbit_camera.target.y) / orbit_camera.distance).asin();
    orbit_camera.pitch = orbit_camera
        .pitch
        .clamp(minimum_pitch, std::f32::consts::FRAC_PI_2 - 0.05);
    let horizontal_distance = orbit_camera.distance * orbit_camera.pitch.cos();
    let offset = Vec3::new(
        horizontal_distance * orbit_camera.yaw.sin(),
        orbit_camera.distance * orbit_camera.pitch.sin(),
        horizontal_distance * orbit_camera.yaw.cos(),
    );
    for mut camera in &mut cameras {
        *camera = Transform::from_translation(orbit_camera.target + offset).looking_at(orbit_camera.target, Vec3::Y);
    }
}

fn update_mount_display(
    weapons: Query<&Transform, With<Weapon>>,
    animations: Query<&ViewerAnimations>,
    mut displays: Query<&mut Text, With<MountDisplay>>,
) {
    let Ok(weapon) = weapons.single() else {
        return;
    };
    let (yaw, pitch, roll) = weapon.rotation.to_euler(EulerRot::YXZ);
    let animation_index = animations.single().map_or(0, |animation| animation.current);
    let animation_name = get_animation_name(animation_index);
    let display = format!(
        "Weapon mount\ntranslation: {:.4}, {:.4}, {:.4}\nrotation YXZ: {:.4}, {:.4}, {:.4}\nscale: {:.4}, {:.4}, {:.4}\n\nAnimation: {animation_index}/42 {animation_name}\nArrow keys: previous and next animation\nP: pause animation\nC: toggle colliders and bones",
        weapon.translation.x,
        weapon.translation.y,
        weapon.translation.z,
        yaw,
        pitch,
        roll,
        weapon.scale.x,
        weapon.scale.y,
        weapon.scale.z,
    );
    for mut text in &mut displays {
        text.0.clone_from(&display);
    }
}

fn get_animation_name(index: usize) -> &'static str {
    match index {
        0 => "A_TPose",
        1 => "Crouch_Fwd_Loop",
        2 => "Crouch_Idle_Loop",
        3 => "Dance_Loop",
        4 => "Death01",
        5 => "Driving_Loop",
        6 => "Fixing_Kneeling",
        7 => "Hit_Chest",
        8 => "Hit_Head",
        9 => "Idle_Loop",
        10 => "Idle_Talking_Loop",
        11 => "Idle_Torch_Loop",
        12 => "Interact",
        13 => "Jog_Fwd_Loop",
        14 => "Jump_Land",
        15 => "Jump_Loop",
        16 => "Jump_Start",
        17 => "PickUp_Table",
        18 => "Pistol_Aim_Down",
        19 => "Pistol_Aim_Neutral",
        20 => "Pistol_Aim_Up",
        21 => "Pistol_Idle_Loop",
        22 => "Pistol_Reload",
        23 => "Pistol_Shoot",
        24 => "Punch_Cross",
        25 => "Punch_Jab",
        26 => "Push_Loop",
        27 => "Roll",
        28 => "Sitting_Enter",
        29 => "Sitting_Exit",
        30 => "Sitting_Idle_Loop",
        31 => "Sitting_Talking_Loop",
        32 => "Spell_Simple_Enter",
        33 => "Spell_Simple_Exit",
        34 => "Spell_Simple_Idle_Loop",
        35 => "Spell_Simple_Shoot",
        36 => "Sprint_Loop",
        37 => "Swim_Fwd_Loop",
        38 => "Swim_Idle_Loop",
        39 => "Sword_Attack",
        40 => "Sword_Idle",
        41 => "Walk_Formal_Loop",
        42 => "Walk_Loop",
        _ => "Unknown",
    }
}
