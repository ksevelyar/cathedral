use avian3d::prelude::*;
use bevy::prelude::*;

use crate::enemies::EnemyHit;
use crate::player::Player;
use crate::ragdoll::{OBJECTS_GROUP, OwnedByEnemy, RAGDOLL_GROUP};
use crate::state::GameState;

pub struct ShootingPlugin;

impl Plugin for ShootingPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<GunshotSound>()
            .add_systems(Startup, (setup_gun, setup_crosshair).after(crate::player::setup_player))
            .add_systems(
                Update,
                (shoot, play_gunshot)
                    .run_if(in_state(GameState::Playing))
                    .after(crate::player::apply_mouse_look)
                    .after(crate::player::move_player),
            );
    }
}

#[derive(Component)]
pub(crate) struct Crosshair;

#[derive(Component)]
pub(crate) struct Gun;

#[derive(Resource)]
pub(crate) struct GunshotSound {
    handle: Handle<AudioSource>,
}

impl FromWorld for GunshotSound {
    fn from_world(world: &mut World) -> Self {
        let asset_server = world.resource::<AssetServer>();
        GunshotSound {
            handle: asset_server.load("weapon/pistol.mp3"),
        }
    }
}

fn setup_crosshair(mut commands: Commands) {
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

fn setup_gun(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    player_camera: Query<Entity, (With<Player>, With<Camera3d>)>,
) {
    let Ok(player_camera) = player_camera.single() else {
        return;
    };
    let gun = commands
        .spawn((
            Gun,
            WorldAssetRoot(asset_server.load(GltfAssetLabel::Scene(0).from_asset("weapon/pistol.glb"))),
            Transform {
                translation: GUN_OFFSET,
                scale: Vec3::splat(0.15),
                rotation: GUN_BASE_ROTATION,
            },
        ))
        .id();
    commands.entity(player_camera).add_children(&[gun]);
}

const SHOT_DISTANCE: f32 = 100.0;
const SHOT_IMPULSE: f32 = 100.0;

pub fn shoot(
    mouse_button: Res<ButtonInput<MouseButton>>,
    camera_query: Query<&Transform, With<Player>>,
    spatial_query: SpatialQuery,
    owners: Query<&OwnedByEnemy>,
    collider_parents: Query<&ChildOf>,
    mut prop_bodies: Query<Forces>,
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
        &SpatialQueryFilter::from_mask(RAGDOLL_GROUP | OBJECTS_GROUP),
    ) else {
        return;
    };

    if let Ok(owner) = owners.get(hit.entity) {
        commands.entity(owner.0).trigger(|entity| EnemyHit {
            entity,
            body: hit.entity,
            impulse: direction.as_vec3() * SHOT_IMPULSE,
            point: origin + direction.as_vec3() * hit.distance,
        });
        return;
    }

    let impulse = direction.as_vec3() * SHOT_IMPULSE;
    let point = origin + direction.as_vec3() * hit.distance;
    if let Some(body_entity) = find_dynamic_body_ancestor(hit.entity, &collider_parents, &prop_bodies)
        && let Ok(mut forces) = prop_bodies.get_mut(body_entity)
    {
        forces.apply_linear_impulse_at_point(impulse, point);
    }
}

fn find_dynamic_body_ancestor(
    mut entity: Entity,
    collider_parents: &Query<&ChildOf>,
    prop_bodies: &Query<Forces>,
) -> Option<Entity> {
    loop {
        if prop_bodies.contains(entity) {
            return Some(entity);
        }
        let Ok(collider_parent) = collider_parents.get(entity) else {
            return None;
        };
        entity = collider_parent.parent();
    }
}

fn play_gunshot(mouse_button: Res<ButtonInput<MouseButton>>, gunshot_sound: Res<GunshotSound>, mut commands: Commands) {
    if mouse_button.just_pressed(MouseButton::Left) {
        commands.spawn((
            AudioPlayer::new(gunshot_sound.handle.clone()),
            PlaybackSettings::DESPAWN,
        ));
    }
}
