use bevy::input::mouse::AccumulatedMouseMotion;
use bevy::prelude::*;

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
                (mouse_look.run_if(not(in_state(GameState::Paused))), move_player).run_if(in_state(GameState::Playing)),
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
        Self {
            yaw: 0.0,
            pitch: 0.0,
        }
    }
}

const MOUSE_SENSITIVITY: f32 = 0.003;
const PLAYER_MOVE_SPEED: f32 = 5.0;
const CAMERA_PITCH_LIMIT: f32 = std::f32::consts::FRAC_PI_2 - 0.01;

pub const PLAYER_START_POSITION: Vec3 = Vec3::new(0.0, 1.5, 12.0);

pub fn setup_player(mut commands: Commands) {
    commands.spawn((
        Camera3d::default(),
        Transform::from_xyz(0.0, 1.5, 12.0),
        CameraState::default(),
        Player,
    ));
}

pub fn reset_player(mut player_query: Query<(&mut Transform, &mut CameraState), With<Player>>) {
    let Ok((mut transform, mut camera_state)) = player_query.single_mut() else {
        return;
    };

    transform.translation = PLAYER_START_POSITION;
    camera_state.yaw = 0.0;
    camera_state.pitch = 0.0;
    transform.rotation = Quat::IDENTITY;
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
    camera_state.pitch = (camera_state.pitch - mouse_delta.y * MOUSE_SENSITIVITY)
        .clamp(-CAMERA_PITCH_LIMIT, CAMERA_PITCH_LIMIT);

    transform.rotation =
        Quat::from_euler(EulerRot::YXZ, camera_state.yaw, camera_state.pitch, 0.0);
}

pub fn move_player(
    keyboard: Res<ButtonInput<KeyCode>>,
    time: Res<Time>,
    mut query: Query<(&mut Transform, &CameraState), With<Player>>,
) {
    let Ok((mut transform, camera_state)) = query.single_mut() else {
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
        transform.translation +=
            movement_direction.normalize() * PLAYER_MOVE_SPEED * time.delta_secs();
    }
}
