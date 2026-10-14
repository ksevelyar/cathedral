use std::time::Duration;

use avian3d::prelude::*;
use bevy::audio::AudioPlugin;
use bevy::ecs::system::SystemParam;
use bevy::prelude::*;

mod muzzle_flash;

pub use muzzle_flash::{
    MUZZLE_FLASH_FORWARD_OFFSET_BOUNDS, MUZZLE_FLASH_LIGHT_INTENSITY_BOUNDS, MUZZLE_FLASH_LIGHT_RANGE_BOUNDS,
    MUZZLE_FLASH_SCALE_BOUNDS, MuzzleFlash,
};

pub(crate) use muzzle_flash::observe_muzzle_flash;

use crate::enemies::EnemyHit;
use crate::player::PlayerCamera;
use crate::ragdoll::{OBJECTS_GROUP, OwnedByCharacter, RAGDOLL_GROUP, RagdollBodyPart};
use crate::state::GameState;

pub struct ShootingPlugin;

impl Plugin for ShootingPlugin {
    fn build(&self, app: &mut App) {
        muzzle_flash::configure(app);
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

#[derive(Component, Clone, Copy, PartialEq, Eq)]
pub enum Weapon {
    Ranged(RangedWeapon),
    Melee(MeleeWeapon),
}

impl Weapon {
    pub fn get_model_asset_path(self) -> &'static str {
        match self {
            Self::Ranged(ranged_weapon) => ranged_weapon.get_model_asset_path(),
            Self::Melee(melee_weapon) => melee_weapon.get_model_asset_path(),
        }
    }

    pub fn get_collider_half_extents(self) -> Vec3 {
        match self {
            Self::Ranged(ranged_weapon) => ranged_weapon.get_collider_half_extents(),
            Self::Melee(melee_weapon) => melee_weapon.get_collider_half_extents(),
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum RangedWeapon {
    Revolver2,
    ShotgunShortStock,
    ShotgunSawedOff,
    SubmachineGun3,
    Pistol,
}

impl RangedWeapon {
    fn get_model_asset_path(self) -> &'static str {
        match self {
            Self::Revolver2 => "weapons/revolver-2-muzzle.glb",
            Self::ShotgunShortStock => "weapons/shotgun-short-stock-muzzle.glb",
            Self::ShotgunSawedOff => "weapons/shotgun-sawed-off-muzzle.glb",
            Self::SubmachineGun3 => "weapons/submachine-gun-3-muzzle.glb",
            Self::Pistol => "weapons/pistol-muzzle.glb",
        }
    }

    fn get_collider_half_extents(self) -> Vec3 {
        match self {
            Self::Pistol => Vec3::new(0.88, 0.15, 0.1),
            Self::Revolver2 | Self::ShotgunShortStock | Self::ShotgunSawedOff | Self::SubmachineGun3 => {
                Vec3::new(0.88, 0.15, 0.1)
            }
        }
    }

    fn get_bullet_mass(self) -> f32 {
        match self {
            Self::Revolver2 | Self::Pistol => 0.1165,
            Self::ShotgunShortStock | Self::ShotgunSawedOff => 0.233,
            Self::SubmachineGun3 => 0.05825,
        }
    }

    fn get_bullet_speed(self) -> f32 {
        match self {
            Self::Revolver2 | Self::Pistol => 670.0,
            Self::ShotgunShortStock | Self::ShotgunSawedOff => 1340.0,
            Self::SubmachineGun3 => 335.0,
        }
    }

    fn get_attack_sound_path(self) -> &'static str {
        match self {
            Self::Revolver2 | Self::Pistol => "weapons/revolver.mp3",
            Self::ShotgunShortStock | Self::ShotgunSawedOff => "weapons/shotgun.mp3",
            Self::SubmachineGun3 => "weapons/submachine-gun.mp3",
        }
    }

    fn get_attack_sound_start_position(self) -> Duration {
        match self {
            Self::Revolver2 | Self::Pistol => Duration::from_millis(400),
            Self::ShotgunShortStock | Self::ShotgunSawedOff => Duration::from_millis(328),
            Self::SubmachineGun3 => Duration::ZERO,
        }
    }

    fn get_fire_mode(self) -> FireMode {
        match self {
            Self::Revolver2 | Self::ShotgunShortStock | Self::ShotgunSawedOff | Self::Pistol => FireMode::SingleShot,
            Self::SubmachineGun3 => FireMode::Automatic,
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum MeleeWeapon {
    Katana,
}

impl MeleeWeapon {
    fn get_model_asset_path(self) -> &'static str {
        match self {
            Self::Katana => "weapons/katana.glb",
        }
    }

    fn get_collider_half_extents(self) -> Vec3 {
        match self {
            Self::Katana => Vec3::new(0.01, 0.04, 0.35),
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
    SingleShot,
    Automatic,
}

#[derive(Message, Clone, Copy)]
pub struct WeaponFired {
    pub(crate) shooter_entity: Option<Entity>,
    pub(crate) weapon_entity: Entity,
    pub(crate) weapon: Weapon,
    pub(crate) origin: Vec3,
    pub(crate) direction: Dir3,
}

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
    player_camera: Query<Entity, (With<PlayerCamera>, With<Camera3d>)>,
) {
    let Ok(player_camera) = player_camera.single() else {
        return;
    };
    let selectable_weapons = [
        (
            Weapon::Ranged(RangedWeapon::Revolver2),
            create_view_model_transform(Vec3::new(0.2, -0.2, -0.4), 0.15),
        ),
        (
            Weapon::Ranged(RangedWeapon::ShotgunShortStock),
            create_view_model_transform(Vec3::new(0.2, -0.22, -0.45), 0.15),
        ),
        (
            Weapon::Ranged(RangedWeapon::ShotgunSawedOff),
            create_view_model_transform(Vec3::new(0.2, -0.24, -0.38), 0.15),
        ),
        (
            Weapon::Ranged(RangedWeapon::SubmachineGun3),
            create_view_model_transform(Vec3::new(0.2, -0.2, -0.45), 0.15),
        ),
    ];
    let mut weapon_entities = Vec::with_capacity(selectable_weapons.len());
    for (weapon, weapon_transform) in selectable_weapons {
        let mut weapon_entity = commands.spawn((
            Gun,
            weapon,
            WeaponSlot(weapon),
            Visibility::Hidden,
            WorldAssetRoot(asset_server.load(GltfAssetLabel::Scene(0).from_asset(weapon.get_model_asset_path()))),
            weapon_transform,
        ));
        let weapon_entity_id = weapon_entity.id();
        if weapon == Weapon::Ranged(RangedWeapon::Revolver2) {
            weapon_entity.insert(Visibility::Inherited);
        }
        observe_muzzle_flash(&mut commands, weapon_entity_id);
        weapon_entities.push(weapon_entity_id);
    }
    commands.insert_resource(SelectedWeapon(Weapon::Ranged(RangedWeapon::Revolver2)));
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
        (KeyCode::Digit1, Weapon::Ranged(RangedWeapon::Revolver2)),
        (KeyCode::Digit2, Weapon::Ranged(RangedWeapon::ShotgunShortStock)),
        (KeyCode::Digit3, Weapon::Ranged(RangedWeapon::ShotgunSawedOff)),
        (KeyCode::Digit4, Weapon::Ranged(RangedWeapon::SubmachineGun3)),
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
    weapon_slots: Query<(Entity, &WeaponSlot), With<Gun>>,
    player_camera: Query<(Entity, &ChildOf, &GlobalTransform), With<PlayerCamera>>,
    time: Res<Time>,
    mut weapon_fire_cooldown: ResMut<WeaponFireCooldown>,
    mut weapon_fired: MessageWriter<WeaponFired>,
) {
    let Some((weapon_entity, weapon_slot)) = weapon_slots
        .iter()
        .find(|(_, weapon_slot)| weapon_slot.0 == **selected_weapon)
    else {
        return;
    };
    let Weapon::Ranged(ranged_weapon) = weapon_slot.0 else {
        return;
    };
    let Ok((_, camera_parent, player_camera_transform)) = player_camera.single() else {
        return;
    };
    let weapon_fired_message = WeaponFired {
        shooter_entity: Some(camera_parent.0),
        weapon_entity,
        weapon: weapon_slot.0,
        origin: player_camera_transform.translation(),
        direction: Dir3::new(*player_camera_transform.forward()).unwrap(),
    };
    match ranged_weapon.get_fire_mode() {
        FireMode::SingleShot => {
            if mouse_button.just_pressed(MouseButton::Left) {
                weapon_fired.write(weapon_fired_message);
            }
        }
        FireMode::Automatic => {
            if mouse_button.just_pressed(MouseButton::Left) {
                weapon_fired.write(weapon_fired_message);
                weapon_fire_cooldown.0.reset();
                return;
            }
            if !mouse_button.pressed(MouseButton::Left) {
                weapon_fire_cooldown.0.reset();
                return;
            }
            weapon_fire_cooldown.0.tick(time.delta());
            for _ in 0..weapon_fire_cooldown.0.times_finished_this_tick() {
                weapon_fired.write(weapon_fired_message);
            }
        }
    }
}

const SHOT_DISTANCE: f32 = 100.0;

fn exclude_shooter_bodies(
    shooter_entity: Option<Entity>,
    owned_bodies: &Query<(Entity, &OwnedByCharacter)>,
    character_children: &Query<&Children>,
) -> SpatialQueryFilter {
    let Some(shooter_entity) = shooter_entity else {
        return SpatialQueryFilter::from_mask(RAGDOLL_GROUP | OBJECTS_GROUP);
    };
    let mut character_entities = vec![shooter_entity];
    if let Ok(children) = character_children.get(shooter_entity) {
        character_entities.extend(children.iter());
    }
    SpatialQueryFilter::from_mask(RAGDOLL_GROUP | OBJECTS_GROUP).with_excluded_entities(
        owned_bodies
            .iter()
            .filter(|(_, owner)| character_entities.contains(&owner.0))
            .map(|(body_entity, _)| body_entity),
    )
}

fn compute_bullet_momentum(weapon: Weapon) -> Option<f32> {
    let Weapon::Ranged(ranged_weapon) = weapon else {
        return None;
    };
    Some(ranged_weapon.get_bullet_mass() * ranged_weapon.get_bullet_speed())
}

#[derive(SystemParam)]
pub(crate) struct HitContext<'w, 's> {
    owners: Query<'w, 's, (Entity, &'static OwnedByCharacter)>,
    ragdoll_bodies: Query<'w, 's, (&'static OwnedByCharacter, &'static RagdollBodyPart)>,
    names: Query<'w, 's, &'static Name>,
}

pub(crate) fn shoot(
    mut weapon_fired: MessageReader<WeaponFired>,
    spatial_query: SpatialQuery,
    hit_context: HitContext,
    character_children: Query<&Children>,
    collider_parents: Query<&ChildOf>,
    mut prop_bodies: Query<Forces>,
    mut commands: Commands,
) {
    for weapon_fired_message in weapon_fired.read() {
        let Some(bullet_momentum) = compute_bullet_momentum(weapon_fired_message.weapon) else {
            continue;
        };
        let origin = weapon_fired_message.origin;
        let direction = weapon_fired_message.direction;
        let ray_filter = exclude_shooter_bodies(
            weapon_fired_message.shooter_entity,
            &hit_context.owners,
            &character_children,
        );
        let Some(hit) = spatial_query.cast_ray(origin, direction, SHOT_DISTANCE, true, &ray_filter) else {
            continue;
        };
        if let Ok((hit_owner, body_part)) = hit_context.ragdoll_bodies.get(hit.entity)
            && Some(hit_owner.0) != weapon_fired_message.shooter_entity
        {
            let shooter_name = weapon_fired_message
                .shooter_entity
                .and_then(|shooter_entity| hit_context.names.get(shooter_entity).ok())
                .map(|name| name.to_string())
                .unwrap_or_else(|| "unknown shooter".to_string());
            info!("player {body_part:?} damaged by {shooter_name}");
        }
        if let Ok((_, owner)) = hit_context.owners.get(hit.entity)
            && Some(owner.0) != weapon_fired_message.shooter_entity
        {
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

fn get_attack_sound_path(weapon: Weapon) -> &'static str {
    match weapon {
        Weapon::Ranged(ranged_weapon) => ranged_weapon.get_attack_sound_path(),
        Weapon::Melee(_) => unreachable!("melee weapons cannot fire"),
    }
}

fn get_attack_sound_start_position(weapon: Weapon) -> Duration {
    match weapon {
        Weapon::Ranged(ranged_weapon) => ranged_weapon.get_attack_sound_start_position(),
        Weapon::Melee(_) => unreachable!("melee weapons cannot fire"),
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
    for weapon_fired_message in weapon_fired.read() {
        commands.spawn((
            AudioPlayer::new(asset_server.load(get_attack_sound_path(weapon_fired_message.weapon))),
            PlaybackSettings::DESPAWN
                .with_start_position(get_attack_sound_start_position(weapon_fired_message.weapon))
                .with_spatial(true),
            Transform::from_translation(weapon_fired_message.origin),
        ));
    }
}
