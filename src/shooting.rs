use std::time::Duration;

use avian3d::prelude::*;
use bevy::audio::AudioPlugin;
use bevy::camera::visibility::RenderLayers;
use bevy::ecs::system::SystemParam;
use bevy::prelude::*;

mod muzzle_flash;

pub use muzzle_flash::{
    MUZZLE_FLASH_FORWARD_OFFSET_BOUNDS, MUZZLE_FLASH_LIGHT_INTENSITY_BOUNDS, MUZZLE_FLASH_LIGHT_RANGE_BOUNDS,
    MUZZLE_FLASH_SCALE_BOUNDS, MuzzleFlash,
};

pub(crate) use muzzle_flash::observe_muzzle_flash;

use crate::enemies::EnemyHit;
use crate::health::{CharacterDamaged, Health};
use crate::player::{Player, PlayerCamera};
use crate::ragdoll::{OBJECTS_GROUP, OwnedByCharacter, RAGDOLL_GROUP, RagdollBodyPart};
use crate::state::GameState;

pub struct ShootingPlugin;

#[derive(SystemSet, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct WeaponFiringSystems;

impl Plugin for ShootingPlugin {
    fn build(&self, app: &mut App) {
        muzzle_flash::configure(app);
        app.configure_sets(Update, WeaponFiringSystems)
            .init_resource::<WeaponFireCooldown>()
            .add_message::<WeaponFired>()
            .add_message::<WeaponReloadStarted>()
            .add_message::<WeaponDryFired>()
            .add_systems(Startup, (setup_gun, setup_crosshair).after(crate::player::setup_player))
            .add_systems(
                Update,
                (
                    switch_weapon,
                    start_player_reload,
                    update_weapon_reloads,
                    fire_weapon.in_set(WeaponFiringSystems),
                    shoot,
                )
                    .chain()
                    .run_if(in_state(GameState::Playing))
                    .after(crate::player::apply_mouse_look)
                    .after(crate::player::move_player),
            );
        if app.is_plugin_added::<AudioPlugin>() {
            app.add_systems(
                Update,
                (play_gunshot, play_reload_sound, play_dry_fire_sound)
                    .run_if(in_state(GameState::Playing))
                    .after(shoot),
            );
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
    pub(crate) fn get_model_asset_path(self) -> &'static str {
        match self {
            Self::Ranged(ranged_weapon) => ranged_weapon.get_model_asset_path(),
            Self::Melee(melee_weapon) => melee_weapon.get_model_asset_path(),
        }
    }

    pub(crate) fn get_collider_half_extents(self) -> Vec3 {
        match self {
            Self::Ranged(ranged_weapon) => ranged_weapon.get_collider_half_extents(),
            Self::Melee(melee_weapon) => melee_weapon.get_collider_half_extents(),
        }
    }

    pub(crate) fn get_damage(self) -> f32 {
        match self {
            Self::Ranged(ranged_weapon) => ranged_weapon.get_damage(),
            Self::Melee(melee_weapon) => melee_weapon.get_damage(),
        }
    }

    pub(crate) fn get_range(self) -> f32 {
        match self {
            Self::Ranged(ranged_weapon) => ranged_weapon.get_range(),
            Self::Melee(melee_weapon) => melee_weapon.get_range(),
        }
    }

    pub(crate) fn get_magazine_capacity(self) -> Option<u8> {
        match self {
            Self::Ranged(ranged_weapon) => Some(ranged_weapon.get_magazine_capacity()),
            Self::Melee(_) => None,
        }
    }

    pub(crate) fn get_reload_duration(self) -> Option<Duration> {
        match self {
            Self::Ranged(ranged_weapon) => Some(ranged_weapon.get_reload_duration()),
            Self::Melee(_) => None,
        }
    }

    fn get_reload_sound_path(self) -> Option<&'static str> {
        match self {
            Self::Ranged(ranged_weapon) => Some(ranged_weapon.get_reload_sound_path()),
            Self::Melee(_) => None,
        }
    }
}

#[derive(Clone, Copy)]
pub struct RangedWeapon {
    definition: &'static RangedWeaponDefinition,
}

impl PartialEq for RangedWeapon {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(self.definition, other.definition)
    }
}

impl Eq for RangedWeapon {}

struct RangedWeaponDefinition {
    model_asset_path: &'static str,
    collider_half_extents: Vec3,
    bullet_mass: f32,
    bullet_speed: f32,
    caliber: f32,
    damage: f32,
    range: f32,
    attack_sound_path: &'static str,
    attack_sound_start_position: Duration,
    fire_mode: FireMode,
    magazine_capacity: u8,
    reload_duration: Duration,
    reload_sound_path: &'static str,
}

const RELOAD_DURATION: Duration = Duration::from_secs(2);
const WEAPON_COLLIDER_HALF_EXTENTS: Vec3 = Vec3::new(0.88, 0.15, 0.1);

const REVOLVER_2_DEFINITION: RangedWeaponDefinition = RangedWeaponDefinition {
    model_asset_path: "weapons/revolver-2-muzzle.glb",
    collider_half_extents: WEAPON_COLLIDER_HALF_EXTENTS,
    bullet_mass: 0.1165,
    bullet_speed: 670.0,
    caliber: 0.0091,
    damage: 40.0,
    range: 100.0,
    attack_sound_path: "weapons/revolver.mp3",
    attack_sound_start_position: Duration::from_millis(450),
    fire_mode: FireMode::SingleShot,
    magazine_capacity: 8,
    reload_duration: RELOAD_DURATION,
    reload_sound_path: "weapons/revolver-reload.mp3",
};

const SHOTGUN_SHORT_STOCK_DEFINITION: RangedWeaponDefinition = RangedWeaponDefinition {
    model_asset_path: "weapons/shotgun-short-stock-muzzle.glb",
    collider_half_extents: WEAPON_COLLIDER_HALF_EXTENTS,
    bullet_mass: 0.233,
    bullet_speed: 1340.0,
    caliber: 0.0185,
    damage: 30.0,
    range: 100.0,
    attack_sound_path: "weapons/shotgun.mp3",
    attack_sound_start_position: Duration::from_millis(328),
    fire_mode: FireMode::SingleShot,
    magazine_capacity: 6,
    reload_duration: RELOAD_DURATION,
    reload_sound_path: "weapons/pump-shotgun-reload.mp3",
};

const SHOTGUN_SAWED_OFF_DEFINITION: RangedWeaponDefinition = RangedWeaponDefinition {
    model_asset_path: "weapons/shotgun-sawed-off-muzzle.glb",
    collider_half_extents: WEAPON_COLLIDER_HALF_EXTENTS,
    bullet_mass: 0.233,
    bullet_speed: 1340.0,
    caliber: 0.0185,
    damage: 30.0,
    range: 100.0,
    attack_sound_path: "weapons/shotgun.mp3",
    attack_sound_start_position: Duration::from_millis(328),
    fire_mode: FireMode::SingleShot,
    magazine_capacity: 2,
    reload_duration: RELOAD_DURATION,
    reload_sound_path: "weapons/rifle-reload.mp3",
};

const SUBMACHINE_GUN_3_DEFINITION: RangedWeaponDefinition = RangedWeaponDefinition {
    model_asset_path: "weapons/submachine-gun-3-muzzle.glb",
    collider_half_extents: WEAPON_COLLIDER_HALF_EXTENTS,
    bullet_mass: 0.05825,
    bullet_speed: 335.0,
    caliber: 0.009,
    damage: 12.0,
    range: 100.0,
    attack_sound_path: "weapons/submachine-gun.mp3",
    attack_sound_start_position: Duration::ZERO,
    fire_mode: FireMode::Automatic,
    magazine_capacity: 24,
    reload_duration: RELOAD_DURATION,
    reload_sound_path: "weapons/rifle-reload.mp3",
};

const PISTOL_DEFINITION: RangedWeaponDefinition = RangedWeaponDefinition {
    model_asset_path: "weapons/pistol-muzzle.glb",
    collider_half_extents: WEAPON_COLLIDER_HALF_EXTENTS,
    bullet_mass: 0.1165,
    bullet_speed: 670.0,
    caliber: 0.009,
    damage: 25.0,
    range: 100.0,
    attack_sound_path: "weapons/pistol.mp3",
    attack_sound_start_position: Duration::from_millis(400),
    fire_mode: FireMode::SingleShot,
    magazine_capacity: 9,
    reload_duration: RELOAD_DURATION,
    reload_sound_path: "weapons/pistol-reload.mp3",
};

impl RangedWeapon {
    pub const REVOLVER_2: Self = Self {
        definition: &REVOLVER_2_DEFINITION,
    };
    pub const SHOTGUN_SHORT_STOCK: Self = Self {
        definition: &SHOTGUN_SHORT_STOCK_DEFINITION,
    };
    pub const SHOTGUN_SAWED_OFF: Self = Self {
        definition: &SHOTGUN_SAWED_OFF_DEFINITION,
    };
    pub const SUBMACHINE_GUN_3: Self = Self {
        definition: &SUBMACHINE_GUN_3_DEFINITION,
    };
    pub const PISTOL: Self = Self {
        definition: &PISTOL_DEFINITION,
    };

    fn get_model_asset_path(self) -> &'static str {
        self.definition.model_asset_path
    }

    fn get_collider_half_extents(self) -> Vec3 {
        self.definition.collider_half_extents
    }

    fn get_bullet_mass(self) -> f32 {
        self.definition.bullet_mass
    }

    fn get_bullet_speed(self) -> f32 {
        self.definition.bullet_speed
    }

    pub fn get_caliber(self) -> f32 {
        self.definition.caliber
    }

    fn get_damage(self) -> f32 {
        self.definition.damage
    }

    fn get_range(self) -> f32 {
        self.definition.range
    }

    fn get_attack_sound_path(self) -> &'static str {
        self.definition.attack_sound_path
    }

    fn get_attack_sound_start_position(self) -> Duration {
        self.definition.attack_sound_start_position
    }

    fn get_fire_mode(self) -> FireMode {
        self.definition.fire_mode
    }

    fn get_magazine_capacity(self) -> u8 {
        self.definition.magazine_capacity
    }

    fn get_reload_duration(self) -> Duration {
        self.definition.reload_duration
    }

    fn get_reload_sound_path(self) -> &'static str {
        self.definition.reload_sound_path
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

    pub fn get_damage(self) -> f32 {
        match self {
            Self::Katana => 40.0,
        }
    }

    pub fn get_range(self) -> f32 {
        match self {
            Self::Katana => 2.0,
        }
    }
}

fn create_view_model_transform(weapon: Weapon) -> Transform {
    let (translation, scale) = if weapon == Weapon::Ranged(RangedWeapon::SHOTGUN_SHORT_STOCK) {
        (Vec3::new(0.2, -0.22, -0.45), 0.15)
    } else if weapon == Weapon::Ranged(RangedWeapon::SHOTGUN_SAWED_OFF) {
        (Vec3::new(0.2, -0.24, -0.38), 0.15)
    } else if weapon == Weapon::Ranged(RangedWeapon::SUBMACHINE_GUN_3) {
        (Vec3::new(0.2, -0.2, -0.45), 0.15)
    } else {
        (Vec3::new(0.2, -0.2, -0.4), 0.15)
    };
    let rotation = Quat::from_rotation_y(std::f32::consts::FRAC_PI_2);
    Transform {
        translation,
        rotation,
        scale: Vec3::splat(scale),
    }
}

fn create_shadow_gun_transform() -> Transform {
    Transform {
        translation: Vec3::new(-0.05, 0.0, 0.0),
        rotation: Quat::from_euler(EulerRot::YXZ, 1.5240, 0.0217, 1.3841),
        scale: Vec3::splat(0.15),
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum FireMode {
    SingleShot,
    Automatic,
}

#[derive(Message, Clone, Copy)]
pub struct WeaponFired {
    pub shooter_entity: Option<Entity>,
    pub weapon_entity: Entity,
    pub weapon: Weapon,
    pub damage_coefficient: f32,
    pub origin: Vec3,
    pub direction: Dir3,
}

#[derive(Message, Clone, Copy)]
pub(crate) struct WeaponReloadStarted(pub(crate) Weapon);

#[derive(Message, Clone, Copy)]
pub(crate) struct WeaponDryFired;

#[derive(Component)]
pub(crate) struct Magazine {
    current_ammunition: u8,
}

impl Magazine {
    pub(crate) fn create_full(weapon: Weapon) -> Self {
        Self {
            current_ammunition: weapon.get_magazine_capacity().unwrap_or_default(),
        }
    }

    pub(crate) fn get_current_ammunition(&self) -> u8 {
        self.current_ammunition
    }

    pub(crate) fn consume_round(&mut self) -> bool {
        if self.current_ammunition == 0 {
            return false;
        }
        self.current_ammunition -= 1;
        true
    }

    pub(crate) fn refill(&mut self, weapon: Weapon) {
        self.current_ammunition = weapon.get_magazine_capacity().unwrap_or_default();
    }

    fn is_full(&self, weapon: Weapon) -> bool {
        self.current_ammunition == weapon.get_magazine_capacity().unwrap_or_default()
    }
}

#[derive(Component)]
pub(crate) struct Reloading(Timer);

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
pub(crate) struct SelectedWeapon(pub Weapon);

#[derive(Component)]
pub(crate) struct WeaponSlot(pub Weapon);

#[derive(Component)]
pub struct ShadowGun;

fn setup_gun(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    players: Query<Entity, With<Player>>,
    player_cameras: Query<Entity, With<PlayerCamera>>,
) {
    let Ok(player_entity) = players.single() else {
        return;
    };
    let Ok(player_camera_entity) = player_cameras.single() else {
        return;
    };
    spawn_player_guns(&mut commands, &asset_server, player_entity, player_camera_entity);
}

pub(crate) fn spawn_player_guns(
    commands: &mut Commands,
    asset_server: &AssetServer,
    player_entity: Entity,
    player_camera_entity: Entity,
) {
    let selectable_weapons = [
        Weapon::Ranged(RangedWeapon::REVOLVER_2),
        Weapon::Ranged(RangedWeapon::SHOTGUN_SHORT_STOCK),
        Weapon::Ranged(RangedWeapon::SHOTGUN_SAWED_OFF),
        Weapon::Ranged(RangedWeapon::SUBMACHINE_GUN_3),
    ];
    let shadow_gun_transform = create_shadow_gun_transform();
    let mut visible_gun_entities = Vec::with_capacity(selectable_weapons.len());
    let mut shadow_gun_entities = Vec::with_capacity(selectable_weapons.len());
    for weapon in selectable_weapons {
        let visibility = if weapon == Weapon::Ranged(RangedWeapon::REVOLVER_2) {
            Visibility::Inherited
        } else {
            Visibility::Hidden
        };
        let mut visible_gun_entity = commands.spawn((
            Gun,
            weapon,
            WeaponSlot(weapon),
            visibility,
            WorldAssetRoot(asset_server.load(GltfAssetLabel::Scene(0).from_asset(weapon.get_model_asset_path()))),
            create_view_model_transform(weapon),
        ));
        if weapon.get_magazine_capacity().is_some() {
            visible_gun_entity.insert(Magazine::create_full(weapon));
        }
        let visible_gun_entity = visible_gun_entity.id();
        observe_muzzle_flash(commands, visible_gun_entity);
        visible_gun_entities.push(visible_gun_entity);

        let shadow_gun_entity = commands
            .spawn((
                ShadowGun,
                weapon,
                WeaponSlot(weapon),
                visibility,
                RenderLayers::layer(1),
                WorldAssetRoot(asset_server.load(GltfAssetLabel::Scene(0).from_asset(weapon.get_model_asset_path()))),
                shadow_gun_transform,
            ))
            .id();
        shadow_gun_entities.push(shadow_gun_entity);
    }
    commands.insert_resource(SelectedWeapon(Weapon::Ranged(RangedWeapon::REVOLVER_2)));
    commands
        .entity(player_camera_entity)
        .add_children(&visible_gun_entities);
    commands.entity(player_entity).add_children(&shadow_gun_entities);
}

fn switch_weapon(
    keyboard_input: Res<ButtonInput<KeyCode>>,
    mut selected_weapon: ResMut<SelectedWeapon>,
    mut weapon_fire_cooldown: ResMut<WeaponFireCooldown>,
    mut weapon_slots: Query<(&WeaponSlot, &mut Visibility)>,
) {
    let mut requested_weapon = None;
    for (key, weapon) in [
        (KeyCode::Digit1, Weapon::Ranged(RangedWeapon::REVOLVER_2)),
        (KeyCode::Digit2, Weapon::Ranged(RangedWeapon::SHOTGUN_SHORT_STOCK)),
        (KeyCode::Digit3, Weapon::Ranged(RangedWeapon::SHOTGUN_SAWED_OFF)),
        (KeyCode::Digit4, Weapon::Ranged(RangedWeapon::SUBMACHINE_GUN_3)),
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

fn start_player_reload(
    keyboard_input: Res<ButtonInput<KeyCode>>,
    selected_weapon: Res<SelectedWeapon>,
    weapon_slots: Query<(Entity, &WeaponSlot, &Magazine, Option<&Reloading>), With<Gun>>,
    mut commands: Commands,
    mut weapon_reload_started: MessageWriter<WeaponReloadStarted>,
) {
    if !keyboard_input.just_pressed(KeyCode::KeyR) {
        return;
    }
    let Some((weapon_entity, weapon_slot, magazine, reloading)) = weapon_slots
        .iter()
        .find(|(_, weapon_slot, _, _)| weapon_slot.0 == **selected_weapon)
    else {
        return;
    };
    if reloading.is_some() || magazine.is_full(weapon_slot.0) {
        return;
    }
    let Some(reload_duration) = weapon_slot.0.get_reload_duration() else {
        return;
    };
    commands
        .entity(weapon_entity)
        .insert(Reloading(Timer::new(reload_duration, TimerMode::Once)));
    weapon_reload_started.write(WeaponReloadStarted(weapon_slot.0));
}

fn update_weapon_reloads(
    time: Res<Time>,
    mut reloading_weapons: Query<(Entity, &Weapon, &mut Magazine, &mut Reloading)>,
    mut commands: Commands,
) {
    for (weapon_entity, weapon, mut magazine, mut reloading) in &mut reloading_weapons {
        reloading.0.tick(time.delta());
        if reloading.0.is_finished() {
            magazine.refill(*weapon);
            commands.entity(weapon_entity).remove::<Reloading>();
        }
    }
}

type PlayerAimQuery<'w, 's> = Query<'w, 's, (Entity, &'static Transform), With<Player>>;
type PlayerCameraAimQuery<'w, 's> = Query<'w, 's, &'static Transform, (With<PlayerCamera>, Without<Player>)>;

#[derive(SystemParam)]
struct WeaponAimContext<'w, 's> {
    player: PlayerAimQuery<'w, 's>,
    player_camera: PlayerCameraAimQuery<'w, 's>,
}

#[derive(SystemParam)]
struct PlayerWeaponMessageContext<'w> {
    weapon_fired: MessageWriter<'w, WeaponFired>,
    weapon_dry_fired: MessageWriter<'w, WeaponDryFired>,
}

fn fire_weapon(
    mouse_button: Res<ButtonInput<MouseButton>>,
    selected_weapon: Res<SelectedWeapon>,
    mut weapon_slots: Query<(Entity, &WeaponSlot, &mut Magazine, Option<&Reloading>), With<Gun>>,
    aim_context: WeaponAimContext,
    time: Res<Time>,
    mut weapon_fire_cooldown: ResMut<WeaponFireCooldown>,
    mut weapon_messages: PlayerWeaponMessageContext,
) {
    let Some((weapon_entity, weapon_slot, mut magazine, reloading)) = weapon_slots
        .iter_mut()
        .find(|(_, weapon_slot, _, _)| weapon_slot.0 == **selected_weapon)
    else {
        return;
    };
    let Weapon::Ranged(ranged_weapon) = weapon_slot.0 else {
        return;
    };
    let Ok((shooter_entity, player_transform)) = aim_context.player.single() else {
        return;
    };
    let Ok(camera_local_transform) = aim_context.player_camera.single() else {
        return;
    };
    let camera_rotation = player_transform.rotation * camera_local_transform.rotation;
    let camera_translation =
        player_transform.translation + player_transform.rotation * camera_local_transform.translation;
    let weapon_fired_message = WeaponFired {
        shooter_entity: Some(shooter_entity),
        weapon_entity,
        weapon: weapon_slot.0,
        damage_coefficient: 1.0,
        origin: camera_translation,
        direction: Dir3::new(camera_rotation * Vec3::NEG_Z).unwrap(),
    };
    if reloading.is_some() {
        return;
    }
    match ranged_weapon.get_fire_mode() {
        FireMode::SingleShot => {
            if mouse_button.just_pressed(MouseButton::Left) {
                if magazine.consume_round() {
                    weapon_messages.weapon_fired.write(weapon_fired_message);
                } else {
                    weapon_messages.weapon_dry_fired.write(WeaponDryFired);
                }
            }
        }
        FireMode::Automatic => {
            if mouse_button.just_pressed(MouseButton::Left) {
                if magazine.consume_round() {
                    weapon_messages.weapon_fired.write(weapon_fired_message);
                } else {
                    weapon_messages.weapon_dry_fired.write(WeaponDryFired);
                }
                weapon_fire_cooldown.0.reset();
                return;
            }
            if !mouse_button.pressed(MouseButton::Left) {
                weapon_fire_cooldown.0.reset();
                return;
            }
            weapon_fire_cooldown.0.tick(time.delta());
            for _ in 0..weapon_fire_cooldown.0.times_finished_this_tick() {
                if !magazine.consume_round() {
                    break;
                }
                weapon_messages.weapon_fired.write(weapon_fired_message);
            }
        }
    }
}

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
    characters: Query<'w, 's, (&'static Name, Option<&'static Health>)>,
    character_children: Query<'w, 's, &'static Children>,
    collider_parents: Query<'w, 's, &'static ChildOf>,
}

pub(crate) fn shoot(
    mut weapon_fired: MessageReader<WeaponFired>,
    mut character_damaged: MessageWriter<CharacterDamaged>,
    spatial_query: SpatialQuery,
    hit_context: HitContext,
    mut prop_bodies: Query<Forces>,
    mut commands: Commands,
) {
    for weapon_fired_message in weapon_fired.read() {
        let Some(bullet_momentum) = compute_bullet_momentum(weapon_fired_message.weapon) else {
            continue;
        };
        let Weapon::Ranged(ranged_weapon) = weapon_fired_message.weapon else {
            continue;
        };
        let origin = weapon_fired_message.origin;
        let direction = weapon_fired_message.direction;
        let ray_filter = exclude_shooter_bodies(
            weapon_fired_message.shooter_entity,
            &hit_context.owners,
            &hit_context.character_children,
        );
        let shot_distance = weapon_fired_message.weapon.get_range();
        let Some(hit) = spatial_query.cast_ray(origin, direction, shot_distance, true, &ray_filter) else {
            continue;
        };
        if let Ok((hit_owner, body_part)) = hit_context.ragdoll_bodies.get(hit.entity)
            && Some(hit_owner.0) != weapon_fired_message.shooter_entity
        {
            let victim = find_health_owner(hit_owner.0, &hit_context.characters, &hit_context.collider_parents);
            let shot_damage = weapon_fired_message.weapon.get_damage() * weapon_fired_message.damage_coefficient;
            character_damaged.write(CharacterDamaged {
                character: victim,
                amount: shot_damage,
                attacker: weapon_fired_message.shooter_entity,
                body_part: Some(*body_part),
            });
        }
        if let Ok((_, owner)) = hit_context.owners.get(hit.entity)
            && Some(owner.0) != weapon_fired_message.shooter_entity
        {
            commands.entity(owner.0).trigger(|target| EnemyHit {
                target,
                collider: hit.entity,
                shot_origin: origin,
                shot_direction: direction,
                impact_distance: hit.distance,
                impulse: direction.as_vec3() * bullet_momentum,
                weapon_caliber: ranged_weapon.get_caliber(),
            });
            continue;
        }

        let point = origin + direction.as_vec3() * hit.distance;
        if let Some(body_entity) = find_dynamic_body_ancestor(hit.entity, &hit_context.collider_parents, &prop_bodies)
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

fn find_health_owner(
    mut character: Entity,
    characters: &Query<'_, '_, (&'static Name, Option<&'static Health>)>,
    parents: &Query<&ChildOf>,
) -> Entity {
    loop {
        if characters.get(character).is_ok_and(|(_, health)| health.is_some()) {
            return character;
        }
        let Ok(parent) = parents.get(character) else {
            return character;
        };
        character = parent.parent();
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

fn play_reload_sound(
    mut weapon_reload_started: MessageReader<WeaponReloadStarted>,
    asset_server: Res<AssetServer>,
    mut commands: Commands,
) {
    for WeaponReloadStarted(weapon) in weapon_reload_started.read() {
        let Some(reload_sound_path) = weapon.get_reload_sound_path() else {
            continue;
        };
        commands.spawn((
            AudioPlayer::new(asset_server.load(reload_sound_path)),
            PlaybackSettings::DESPAWN,
        ));
    }
}

fn play_dry_fire_sound(
    mut weapon_dry_fired: MessageReader<WeaponDryFired>,
    asset_server: Res<AssetServer>,
    mut commands: Commands,
) {
    for WeaponDryFired in weapon_dry_fired.read() {
        commands.spawn((
            AudioPlayer::new(asset_server.load("weapons/dry-fire.mp3")),
            PlaybackSettings::DESPAWN,
        ));
    }
}
