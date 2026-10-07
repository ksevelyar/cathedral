use avian3d::prelude::{Gravity, LinearVelocity, MoveAndSlide, SpatialQueryFilter};
use bevy::camera::Exposure;
use bevy::input::mouse::AccumulatedMouseMotion;
use bevy::prelude::*;

use crate::maps::PlayerPosition;
use crate::movement::PhysicsWorld;
use crate::state::GameState;

pub struct PlayerPlugin;

impl Plugin for PlayerPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Update,
            (
                apply_mouse_look.run_if(not(in_state(GameState::Paused))),
                settle_player_on_startup,
                move_player,
            )
                .run_if(in_state(GameState::Playing)),
        );
    }
}

#[derive(Component)]
pub struct Player;

#[derive(Component)]
pub struct CameraState {
    pub yaw: f32,
    pub pitch: f32,
}

impl Default for CameraState {
    fn default() -> Self {
        Self { yaw: 0.0, pitch: 0.0 }
    }
}

const MOUSE_SENSITIVITY: f32 = 0.003;
const PLAYER_MOVE_SPEED: f32 = 5.0;
const CAMERA_PITCH_LIMIT: f32 = std::f32::consts::FRAC_PI_2 - 0.01;
const PLAYER_COLLISION_RADIUS: f32 = 0.4;
const PLAYER_EYE_HEIGHT: f32 = 1.8;

pub fn setup_player(mut commands: Commands, start_position: Option<Res<PlayerPosition>>) {
    let player_position = start_position
        .map(|start_position| *start_position)
        .unwrap_or_else(|| PlayerPosition::new(Vec3::ZERO));
    let (yaw, pitch) = compute_look_angles(player_position.position, player_position.look_at);
    commands.spawn((
        Camera3d::default(),
        Exposure::INDOOR,
        Transform::from_translation(player_position.position).with_rotation(Quat::from_euler(
            EulerRot::YXZ,
            yaw,
            pitch,
            0.0,
        )),
        CameraState { yaw, pitch },
        LinearVelocity::default(),
        Player,
    ));
}

pub fn compute_look_angles(eye: Vec3, look_at: Vec3) -> (f32, f32) {
    let direction = (look_at - eye).normalize();
    ((-direction.x).atan2(-direction.z), direction.y.asin())
}

pub fn reset_player(
    mut player_query: Query<(&mut Transform, &mut CameraState), With<Player>>,
    start_position: &PlayerPosition,
) {
    let Ok((mut transform, mut camera_state)) = player_query.single_mut() else {
        return;
    };

    let (yaw, pitch) = compute_look_angles(start_position.position, start_position.look_at);
    transform.translation = start_position.position;
    transform.rotation = Quat::from_euler(EulerRot::YXZ, yaw, pitch, 0.0);
    camera_state.yaw = yaw;
    camera_state.pitch = pitch;
}

pub fn settle_player_on_startup(
    mut player: Query<(&mut Transform, &mut LinearVelocity), Added<Player>>,
    move_and_slide: MoveAndSlide,
) {
    for (mut transform, mut velocity) in &mut player {
        let filter = SpatialQueryFilter::default();
        let physics_world = PhysicsWorld::new(
            &move_and_slide,
            0.0,
            0.0,
            &filter,
            PLAYER_COLLISION_RADIUS,
            0.0,
            PLAYER_EYE_HEIGHT,
        );
        if let Some(ground_height) = physics_world.find_ground_height(transform.translation) {
            transform.translation.y = ground_height + PLAYER_EYE_HEIGHT;
            velocity.y = 0.0;
        }
    }
}

pub fn apply_mouse_look(
    mouse_motion: Res<AccumulatedMouseMotion>,
    mut query: Query<(&mut Transform, &mut CameraState), With<Player>>,
) {
    let Ok((mut transform, mut camera_state)) = query.single_mut() else {
        return;
    };

    let mouse_delta = mouse_motion.delta;
    if mouse_delta == Vec2::ZERO {
        return;
    }

    camera_state.yaw -= mouse_delta.x * MOUSE_SENSITIVITY;
    camera_state.pitch =
        (camera_state.pitch - mouse_delta.y * MOUSE_SENSITIVITY).clamp(-CAMERA_PITCH_LIMIT, CAMERA_PITCH_LIMIT);

    transform.rotation = Quat::from_euler(EulerRot::YXZ, camera_state.yaw, camera_state.pitch, 0.0);
}

pub fn move_player(
    keyboard: Res<ButtonInput<KeyCode>>,
    time: Res<Time>,
    move_and_slide: MoveAndSlide,
    gravity: Res<Gravity>,
    mut query: Query<(&mut Transform, &CameraState, &mut LinearVelocity), With<Player>>,
) {
    let Ok((mut transform, camera_state, mut velocity)) = query.single_mut() else {
        return;
    };

    let forward_direction = Vec3::new(-camera_state.yaw.sin(), 0.0, -camera_state.yaw.cos());
    let right_direction = Vec3::new(camera_state.yaw.cos(), 0.0, -camera_state.yaw.sin());

    let mut movement_direction = Vec3::ZERO;
    if keyboard.pressed(KeyCode::KeyW) {
        movement_direction += forward_direction;
    }
    if keyboard.pressed(KeyCode::KeyS) {
        movement_direction -= forward_direction;
    }
    if keyboard.pressed(KeyCode::KeyD) {
        movement_direction += right_direction;
    }
    if keyboard.pressed(KeyCode::KeyA) {
        movement_direction -= right_direction;
    }

    let filter = SpatialQueryFilter::default();
    let physics_world = PhysicsWorld::new(
        &move_and_slide,
        gravity.0.y,
        time.delta_secs(),
        &filter,
        PLAYER_COLLISION_RADIUS,
        0.0,
        PLAYER_EYE_HEIGHT,
    );

    if movement_direction != Vec3::ZERO {
        let desired_translation = movement_direction.normalize() * PLAYER_MOVE_SPEED * time.delta_secs();
        let position = transform.translation;
        transform.translation += physics_world.slide(position, desired_translation);
    }
    physics_world.integrate_fall(&mut velocity, &mut transform.translation);
}
