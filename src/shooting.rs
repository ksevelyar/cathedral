use bevy::prelude::*;

use crate::enemies::{Enemy, ENEMY_RADIUS};
use crate::player::Player;
use crate::state::GameState;

pub struct ShootingPlugin;

impl Plugin for ShootingPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<GunshotSound>()
            .add_systems(
                Update,
                (shoot, play_gunshot).run_if(in_state(GameState::Playing)),
            );
    }
}

#[derive(Component)]
pub struct Crosshair;

#[derive(Component)]
pub struct Gun;

const GUN_BARREL_RADIUS: f32 = 0.03;
const GUN_BARREL_HEIGHT: f32 = 0.25;

#[derive(Resource)]
pub struct GunshotSound {
    pub handle: Handle<AudioSource>,
}

impl FromWorld for GunshotSound {
    fn from_world(world: &mut World) -> Self {
        let asset_server = world.resource::<AssetServer>();
        GunshotSound {
            handle: asset_server.load("pistol.mp3"),
        }
    }
}

pub fn setup_crosshair(mut commands: Commands) {
    commands.spawn((
        Crosshair,
        Node {
            width: Val::Percent(100.0),
            height: Val::Percent(100.0),
            align_items: AlignItems::Center,
            justify_content: JustifyContent::Center,
            position_type: PositionType::Absolute,
            ..default()
        },
        children![
            (
                Node {
                    width: Val::Px(20.0),
                    height: Val::Px(2.0),
                    position_type: PositionType::Absolute,
                    left: Val::Percent(50.0),
                    top: Val::Percent(50.0),
                    margin: UiRect {
                        left: Val::Px(-10.0),
                        top: Val::Px(-1.0),
                        ..default()
                    },
                    ..default()
                },
                BackgroundColor(Color::WHITE),
            ),
            (
                Node {
                    width: Val::Px(2.0),
                    height: Val::Px(20.0),
                    position_type: PositionType::Absolute,
                    left: Val::Percent(50.0),
                    top: Val::Percent(50.0),
                    margin: UiRect {
                        left: Val::Px(-1.0),
                        top: Val::Px(-10.0),
                        ..default()
                    },
                    ..default()
                },
                BackgroundColor(Color::WHITE),
            ),
        ],
    ));
}

pub fn setup_gun(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    player_query: Query<Entity, With<Player>>,
) {
    let Ok(player_entity) = player_query.single() else {
        return;
    };

    let gun_barrel_mesh = meshes.add(Cylinder::new(GUN_BARREL_RADIUS, GUN_BARREL_HEIGHT));
    let gun_barrel_material = materials.add(StandardMaterial {
        base_color: Color::srgb(0.3, 0.3, 0.3),
        ..default()
    });

    commands.entity(player_entity).with_children(|parent| {
        parent.spawn((
            Gun,
            Mesh3d(gun_barrel_mesh),
            MeshMaterial3d(gun_barrel_material),
            Transform::from_xyz(0.2, -0.2, -0.4)
                .with_rotation(Quat::from_rotation_x(-90.0_f32.to_radians())),
        ));
    });
}

pub fn shoot(
    mouse_button: Res<ButtonInput<MouseButton>>,
    camera_query: Query<&Transform, With<Player>>,
    enemy_query: Query<(Entity, &Transform), With<Enemy>>,
    mut commands: Commands,
) {
    if !mouse_button.just_pressed(MouseButton::Left) {
        return;
    }

    let Ok(camera_transform) = camera_query.single() else {
        return;
    };

    let camera_position = camera_transform.translation;
    let camera_forward = camera_transform.forward().as_vec3();

    for (enemy_entity, enemy_transform) in enemy_query.iter() {
        let direction_to_enemy = enemy_transform.translation - camera_position;
        let distance_to_enemy = direction_to_enemy.length();
        let enemy_angular_radius = (ENEMY_RADIUS / distance_to_enemy).asin();
        let angle_to_enemy = camera_forward.dot(direction_to_enemy.normalize()).acos();

        if angle_to_enemy < enemy_angular_radius {
            commands.entity(enemy_entity).despawn();
            return;
        }
    }
}

pub fn play_gunshot(
    mouse_button: Res<ButtonInput<MouseButton>>,
    gunshot_sound: Res<GunshotSound>,
    mut commands: Commands,
) {
    if mouse_button.just_pressed(MouseButton::Left) {
        commands.spawn((
            AudioPlayer::new(gunshot_sound.handle.clone()),
            PlaybackSettings::DESPAWN,
        ));
    }
}
