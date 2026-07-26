use bevy::prelude::*;

use crate::enemies::{self, Dying, Enemy, ENEMY_APPROX_RADIUS};
use crate::player::Player;
use crate::state::GameState;

pub struct ShootingPlugin;

impl Plugin for ShootingPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<GunshotSound>()
            .add_systems(Startup, (setup_gun, setup_crosshair))
            .add_systems(
                Update,
                (position_gun, shoot, play_gunshot)
                    .run_if(in_state(GameState::Playing)),
            );
    }
}

#[derive(Component)]
pub struct Crosshair;

#[derive(Component)]
pub struct Gun;

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

const GUN_OFFSET: Vec3 = Vec3::new(0.2, -0.2, -0.4);
const GUN_BASE_ROTATION: Quat = Quat::from_xyzw(0.0, 0.7071068, 0.0, 0.7071068);

pub fn setup_gun(mut commands: Commands, asset_server: Res<AssetServer>) {
    commands.spawn((
        Gun,
        WorldAssetRoot(
            asset_server.load(GltfAssetLabel::Scene(0).from_asset("guns/pistol.glb")),
        ),
        Transform {
            scale: Vec3::splat(0.15),
            rotation: GUN_BASE_ROTATION,
            ..default()
        },
    ));
}

fn position_gun(
    camera_query: Query<&Transform, (With<Player>, Without<Gun>)>,
    mut gun_query: Query<&mut Transform, (With<Gun>, Without<Player>)>,
) {
    let Ok(camera_transform) = camera_query.single() else { return };
    let Ok(mut gun_transform) = gun_query.single_mut() else { return };
    gun_transform.translation =
        camera_transform.translation + camera_transform.rotation * GUN_OFFSET;
    gun_transform.rotation = camera_transform.rotation * GUN_BASE_ROTATION;
}

pub fn shoot(
    mouse_button: Res<ButtonInput<MouseButton>>,
    camera_query: Query<&Transform, With<Player>>,
    enemy_query: Query<(Entity, &Transform), (With<Enemy>, Without<Dying>)>,
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
        let origin_to_center = camera_position - enemy_transform.translation;
        let projection = origin_to_center.dot(camera_forward);
        let radius_squared = ENEMY_APPROX_RADIUS * ENEMY_APPROX_RADIUS;
        let discriminant = projection * projection
            - (origin_to_center.dot(origin_to_center) - radius_squared);

        if discriminant >= 0.0 {
            enemies::kill_enemy(&mut commands, enemy_entity);
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
