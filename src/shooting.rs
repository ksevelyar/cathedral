use avian3d::prelude::*;
use bevy::prelude::*;

use crate::enemies::{Dying, Enemy};
use crate::player::Player;
use crate::ragdoll::{OwnedByEnemy, PendingRagdollImpact, RagdollBodyPart};
use crate::state::GameState;

pub struct ShootingPlugin;

impl Plugin for ShootingPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<GunshotSound>()
            .add_systems(Startup, (setup_gun, setup_crosshair))
            .add_systems(
                Update,
                (position_gun, shoot, play_gunshot)
                    .run_if(in_state(GameState::Playing))
                    .after(crate::player::mouse_look),
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
const GUN_BASE_ROTATION: Quat = Quat::from_xyzw(
    0.0,
    std::f32::consts::FRAC_1_SQRT_2,
    0.0,
    std::f32::consts::FRAC_1_SQRT_2,
);

pub fn setup_gun(mut commands: Commands, asset_server: Res<AssetServer>) {
    commands.spawn((
        Gun,
        WorldAssetRoot(asset_server.load(GltfAssetLabel::Scene(0).from_asset("guns/pistol.glb"))),
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
    let Ok(camera_transform) = camera_query.single() else {
        return;
    };
    let Ok(mut gun_transform) = gun_query.single_mut() else {
        return;
    };
    gun_transform.translation = camera_transform.translation + camera_transform.rotation * GUN_OFFSET;
    gun_transform.rotation = camera_transform.rotation * GUN_BASE_ROTATION;
}

type AliveEnemy = (With<Enemy>, Without<Dying>);

const SHOT_DISTANCE: f32 = 100.0;
const SHOT_IMPULSE: f32 = 100.0;

pub fn shoot(
    mouse_button: Res<ButtonInput<MouseButton>>,
    camera_query: Query<&Transform, With<Player>>,
    spatial_query: SpatialQuery,
    owners: Query<&OwnedByEnemy>,
    alive_enemies: Query<(), AliveEnemy>,
    ragdoll_bodies: Query<(Entity, &OwnedByEnemy), With<RagdollBodyPart>>,
    mut commands: Commands,
) {
    if !mouse_button.just_pressed(MouseButton::Left) {
        return;
    }

    let Ok(camera_transform) = camera_query.single() else {
        return;
    };

    let origin = camera_transform.translation;
    let direction = camera_transform.forward();
    let Some(hit) = spatial_query.cast_ray(
        origin,
        direction,
        SHOT_DISTANCE,
        true,
        &SpatialQueryFilter::from_mask(0b10),
    ) else {
        return;
    };
    let Ok(owner) = owners.get(hit.entity) else {
        return;
    };
    if alive_enemies.get(owner.0).is_err() {
        return;
    }
    for (body, body_owner) in &ragdoll_bodies {
        if body_owner.0 == owner.0 {
            commands.entity(body).remove::<RigidBodyDisabled>();
        }
    }
    commands.entity(owner.0).insert(Dying);
    commands.entity(hit.entity).insert(PendingRagdollImpact {
        impulse: direction.as_vec3() * SHOT_IMPULSE,
        point: origin + direction.as_vec3() * hit.distance,
    });
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
