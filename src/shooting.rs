use std::time::Duration;

use avian3d::prelude::*;
use bevy::audio::AudioPlugin;
use bevy::prelude::*;

use crate::enemies::EnemyHit;
use crate::player::Player;
use crate::ragdoll::{OBJECTS_GROUP, OwnedByEnemy, RAGDOLL_GROUP};
use crate::state::GameState;

pub struct ShootingPlugin;

impl Plugin for ShootingPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<WeaponFireCooldown>()
            .add_message::<WeaponFired>()
            .add_systems(Startup, (setup_gun, setup_crosshair).after(crate::player::setup_player))
            .add_systems(
                Update,
                (switch_weapon, fire_weapon, shoot)
                    .chain()
                    .run_if(in_state(GameState::Playing))
                    .after(crate::player::apply_mouse_look)
                    .after(crate::player::move_player),
            );
        if app.is_plugin_added::<AudioPlugin>() {
            app.add_systems(Update, play_gunshot.run_if(in_state(GameState::Playing)).after(shoot));
        }
    }
}

#[derive(Component)]
pub(crate) struct Crosshair;

#[derive(Component)]
pub struct Gun;

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Weapon {
    Revolver2,
    ShotgunShortStock,
    ShotgunSawedOff,
    SubmachineGun3,
}

impl Weapon {
    fn get_model_asset_path(self) -> &'static str {
        match self {
            Self::Revolver2 => "weapons/revolver-2.glb",
            Self::ShotgunShortStock => "weapons/shotgun-short-stock.glb",
            Self::ShotgunSawedOff => "weapons/shotgun-sawed-off.glb",
            Self::SubmachineGun3 => "weapons/submachine-gun-3.glb",
        }
    }

    fn get_bullet_mass(self) -> f32 {
        match self {
            Self::Revolver2 => 0.1165,
            Self::ShotgunShortStock | Self::ShotgunSawedOff => 0.233,
            Self::SubmachineGun3 => 0.05825,
        }
    }

    fn get_bullet_speed(self) -> f32 {
        match self {
            Self::Revolver2 => 670.0,
            Self::ShotgunShortStock | Self::ShotgunSawedOff => 1340.0,
            Self::SubmachineGun3 => 335.0,
        }
    }

    fn get_sound_asset_path(self) -> &'static str {
        match self {
            Self::Revolver2 => "weapons/revolver.mp3",
            Self::ShotgunShortStock | Self::ShotgunSawedOff => "weapons/shotgun.mp3",
            Self::SubmachineGun3 => "weapons/submachine-gun.mp3",
        }
    }

    fn get_sound_start_position(self) -> Duration {
        match self {
            Self::Revolver2 => Duration::from_millis(400),
            Self::ShotgunShortStock | Self::ShotgunSawedOff => Duration::from_millis(328),
            Self::SubmachineGun3 => Duration::ZERO,
        }
    }

    fn get_fire_mode(self) -> FireMode {
        match self {
            Self::Revolver2 | Self::ShotgunShortStock | Self::ShotgunSawedOff => FireMode::SemiAutomatic,
            Self::SubmachineGun3 => FireMode::FullAutomatic,
        }
    }

    fn create_view_model_transform(self) -> Transform {
        match self {
            Self::Revolver2 => create_view_model_transform(Vec3::new(0.2, -0.2, -0.4), 0.15),
            Self::ShotgunShortStock => create_view_model_transform(Vec3::new(0.2, -0.22, -0.45), 0.15),
            Self::ShotgunSawedOff => create_view_model_transform(Vec3::new(0.2, -0.24, -0.38), 0.15),
            Self::SubmachineGun3 => create_view_model_transform(Vec3::new(0.2, -0.2, -0.45), 0.15),
        }
    }
}

fn create_view_model_transform(model_translation: Vec3, model_scale: f32) -> Transform {
    let model_rotation = Quat::from_xyzw(
        0.0,
        std::f32::consts::FRAC_1_SQRT_2,
        0.0,
        std::f32::consts::FRAC_1_SQRT_2,
    );
    Transform {
        translation: model_translation,
        rotation: model_rotation,
        scale: Vec3::splat(model_scale),
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum FireMode {
    SemiAutomatic,
    FullAutomatic,
}

#[derive(Message)]
pub struct WeaponFired(Weapon);

#[derive(Resource)]
struct WeaponFireCooldown(Timer);

impl Default for WeaponFireCooldown {
    fn default() -> Self {
        let shots_per_second = 10.0;
        Self(Timer::from_seconds(1.0 / shots_per_second, TimerMode::Repeating))
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

#[derive(Resource, Deref)]
pub struct SelectedWeapon(pub Weapon);

#[derive(Component)]
pub struct WeaponSlot(pub Weapon);

fn setup_gun(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    player_camera: Query<Entity, (With<Player>, With<Camera3d>)>,
) {
    let Ok(player_camera) = player_camera.single() else {
        return;
    };
    let selectable_weapons = [
        Weapon::Revolver2,
        Weapon::ShotgunShortStock,
        Weapon::ShotgunSawedOff,
        Weapon::SubmachineGun3,
    ];
    let mut weapon_entities = Vec::with_capacity(selectable_weapons.len());
    for weapon in selectable_weapons {
        let mut weapon_entity = commands.spawn((
            Gun,
            WeaponSlot(weapon),
            Visibility::Hidden,
            WorldAssetRoot(asset_server.load(GltfAssetLabel::Scene(0).from_asset(weapon.get_model_asset_path()))),
            weapon.create_view_model_transform(),
        ));
        if weapon == Weapon::Revolver2 {
            weapon_entity.insert(Visibility::Inherited);
        }
        weapon_entities.push(weapon_entity.id());
    }
    commands.insert_resource(SelectedWeapon(Weapon::Revolver2));
    commands.entity(player_camera).add_children(&weapon_entities);
}

fn switch_weapon(
    keyboard_input: Res<ButtonInput<KeyCode>>,
    mut selected_weapon: ResMut<SelectedWeapon>,
    mut weapon_fire_cooldown: ResMut<WeaponFireCooldown>,
    mut weapon_slots: Query<(&WeaponSlot, &mut Visibility)>,
) {
    let mut requested_weapon = None;
    for (key, weapon) in [
        (KeyCode::Digit1, Weapon::Revolver2),
        (KeyCode::Digit2, Weapon::ShotgunShortStock),
        (KeyCode::Digit3, Weapon::ShotgunSawedOff),
        (KeyCode::Digit4, Weapon::SubmachineGun3),
    ] {
        if keyboard_input.just_pressed(key) {
            requested_weapon = Some(weapon);
        }
    }
    let Some(requested_weapon) = requested_weapon else {
        return;
    };
    if requested_weapon == selected_weapon.0 {
        return;
    }
    selected_weapon.0 = requested_weapon;
    weapon_fire_cooldown.0.reset();
    for (weapon_slot, mut weapon_visibility) in &mut weapon_slots {
        if weapon_slot.0 == requested_weapon {
            *weapon_visibility = Visibility::Inherited;
        } else {
            *weapon_visibility = Visibility::Hidden;
        }
    }
}

fn fire_weapon(
    mouse_button: Res<ButtonInput<MouseButton>>,
    selected_weapon: Res<SelectedWeapon>,
    time: Res<Time>,
    mut weapon_fire_cooldown: ResMut<WeaponFireCooldown>,
    mut weapon_fired: MessageWriter<WeaponFired>,
) {
    match selected_weapon.get_fire_mode() {
        FireMode::SemiAutomatic => {
            if mouse_button.just_pressed(MouseButton::Left) {
                weapon_fired.write(WeaponFired(**selected_weapon));
            }
        }
        FireMode::FullAutomatic => {
            if mouse_button.just_pressed(MouseButton::Left) {
                weapon_fired.write(WeaponFired(**selected_weapon));
                weapon_fire_cooldown.0.reset();
                return;
            }
            if !mouse_button.pressed(MouseButton::Left) {
                weapon_fire_cooldown.0.reset();
                return;
            }
            weapon_fire_cooldown.0.tick(time.delta());
            for _ in 0..weapon_fire_cooldown.0.times_finished_this_tick() {
                weapon_fired.write(WeaponFired(**selected_weapon));
            }
        }
    }
}

const SHOT_DISTANCE: f32 = 100.0;

fn compute_bullet_momentum(weapon: Weapon) -> f32 {
    weapon.get_bullet_mass() * weapon.get_bullet_speed()
}

pub fn shoot(
    mut weapon_fired: MessageReader<WeaponFired>,
    camera_query: Query<&Transform, With<Player>>,
    spatial_query: SpatialQuery,
    owners: Query<&OwnedByEnemy>,
    collider_parents: Query<&ChildOf>,
    mut prop_bodies: Query<Forces>,
    mut commands: Commands,
) {
    let Ok(camera_transform) = camera_query.single() else {
        return;
    };

    let origin = camera_transform.translation;
    let direction = camera_transform.forward();
    for WeaponFired(weapon) in weapon_fired.read() {
        let Some(hit) = spatial_query.cast_ray(
            origin,
            direction,
            SHOT_DISTANCE,
            true,
            &SpatialQueryFilter::from_mask(RAGDOLL_GROUP | OBJECTS_GROUP),
        ) else {
            continue;
        };

        let bullet_momentum = compute_bullet_momentum(*weapon);
        if let Ok(owner) = owners.get(hit.entity) {
            commands.entity(owner.0).trigger(|entity| EnemyHit {
                entity,
                body: hit.entity,
                impulse: direction.as_vec3() * bullet_momentum,
                point: origin + direction.as_vec3() * hit.distance,
            });
            continue;
        }

        let point = origin + direction.as_vec3() * hit.distance;
        if let Some(body_entity) = find_dynamic_body_ancestor(hit.entity, &collider_parents, &prop_bodies)
            && let Ok(mut forces) = prop_bodies.get_mut(body_entity)
        {
            let impulse = direction.as_vec3() * bullet_momentum;
            forces.apply_linear_impulse_at_point(impulse, point);
        }
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

fn play_gunshot(mut weapon_fired: MessageReader<WeaponFired>, asset_server: Res<AssetServer>, mut commands: Commands) {
    for WeaponFired(weapon) in weapon_fired.read() {
        commands.spawn((
            AudioPlayer::new(asset_server.load(weapon.get_sound_asset_path())),
            PlaybackSettings::DESPAWN.with_start_position(weapon.get_sound_start_position()),
        ));
    }
}
