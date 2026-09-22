use avian3d::prelude::{Gravity, LinearVelocity, MoveAndSlide, SpatialQueryFilter};
use bevy::input::mouse::AccumulatedMouseMotion;
use bevy::prelude::*;

use crate::collision::{integrate_fall, slide_translation, snapped_ground_height};
use crate::maps::PlayerStartPosition;
use crate::state::GameState;

pub struct PlayerPlugin;

impl Plugin for PlayerPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(ClearColor(Color::srgb(0.05, 0.05, 0.05)))
            .insert_resource(GlobalAmbientLight {
                brightness: 0.0,
                ..default()
            })
            .add_systems(
                Update,
                (
                    mouse_look.run_if(not(in_state(GameState::Paused))),
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

pub fn setup_player(mut commands: Commands, start_position: Option<Res<PlayerStartPosition>>) {
    commands.spawn((
        Camera3d::default(),
        Transform::from_translation(
            start_position
                .map(|start_position| start_position.position)
                .unwrap_or(Vec3::ZERO),
        ),
        CameraState::default(),
        LinearVelocity::default(),
        Player,
    ));
}

pub fn reset_player(
    mut player_query: Query<(&mut Transform, &mut CameraState), With<Player>>,
    start_position: &PlayerStartPosition,
) {
    let Ok((mut transform, mut camera_state)) = player_query.single_mut() else {
        return;
    };

    transform.translation = start_position.position;
    camera_state.yaw = 0.0;
    camera_state.pitch = 0.0;
    transform.rotation = Quat::IDENTITY;
}

pub fn settle_player_on_startup(
    mut player: Query<(&mut Transform, &mut LinearVelocity), Added<Player>>,
    move_and_slide: MoveAndSlide,
) {
    for (mut transform, mut velocity) in &mut player {
        if let Some(ground_height) = snapped_ground_height(
            &move_and_slide.spatial_query,
            transform.translation,
            &SpatialQueryFilter::default(),
        ) {
            transform.translation.y = ground_height + PLAYER_EYE_HEIGHT;
            velocity.y = 0.0;
        }
    }
}

pub fn mouse_look(
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

    if movement_direction != Vec3::ZERO {
        let desired_translation = movement_direction.normalize() * PLAYER_MOVE_SPEED * time.delta_secs();
        let position = transform.translation;
        transform.translation += slide_translation(
            &move_and_slide,
            position,
            PLAYER_COLLISION_RADIUS,
            0.0,
            &SpatialQueryFilter::default(),
            desired_translation,
            time.delta_secs(),
        );
    }
    let ground_height = snapped_ground_height(
        &move_and_slide.spatial_query,
        transform.translation,
        &SpatialQueryFilter::default(),
    );
    integrate_fall(
        &mut velocity,
        &mut transform.translation.y,
        ground_height,
        PLAYER_EYE_HEIGHT,
        time.delta_secs(),
        gravity.0.y,
    );
}
