use std::time::Duration;

use avian3d::prelude::*;
use bevy::audio::AudioPlugin;
use bevy::prelude::*;
use bevy_hanabi::prelude::*;

use crate::enemies::EnemyHit;
use crate::player::Player;
use crate::ragdoll::{OBJECTS_GROUP, OwnedByEnemy, RAGDOLL_GROUP};
use crate::state::GameState;

pub struct ShootingPlugin;

impl Plugin for ShootingPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<WeaponFireCooldown>()
            .add_message::<WeaponFired>()
            .configure_sets(Update, RevolverMuzzleFlashSystems)
            .add_systems(
                Startup,
                (setup_gun, setup_crosshair, setup_revolver_muzzle_flash).after(crate::player::setup_player),
            )
            .add_systems(
                Update,
                (
                    switch_weapon,
                    fire_weapon,
                    spawn_revolver_muzzle_flash.in_set(RevolverMuzzleFlashSystems),
                    update_muzzle_flashes,
                    shoot,
                )
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

impl WeaponFired {
    pub fn revolver() -> Self {
        Self(Weapon::Revolver2)
    }
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

#[derive(Component)]
struct MuzzleFlashLifetime(Timer);

#[derive(Component)]
struct RevolverMuzzleFlash;

#[derive(SystemSet, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) struct RevolverMuzzleFlashSystems;

#[derive(Resource)]
struct RevolverMuzzleFlashAssets {
    flame_effect: Handle<EffectAsset>,
    texture: Handle<Image>,
}

fn setup_revolver_muzzle_flash(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    effects: Option<ResMut<Assets<EffectAsset>>>,
) {
    let Some(mut effects) = effects else {
        return;
    };
    let expression_writer = ExprWriter::new();
    let particle_age = expression_writer.lit(0.0).expr();
    let particle_lifetime = (expression_writer.lit(0.025)
        + expression_writer.lit(0.045) * expression_writer.rand(ScalarType::Float))
        .expr();
    let particle_rotation = (expression_writer.rand(ScalarType::Float)
        * expression_writer.lit(std::f32::consts::TAU))
        .expr();
    let particle_speed = (expression_writer.lit(4.0)
        + expression_writer.lit(5.0) * expression_writer.rand(ScalarType::Float))
        .expr();
    let particle_texture_slot = expression_writer.lit(0u32).expr();
    let mut color_gradient = bevy_hanabi::Gradient::new();
    color_gradient.add_key(0.0, Vec4::new(3.0, 2.0, 0.4, 1.0));
    color_gradient.add_key(0.3, Vec4::new(2.0, 0.5, 0.04, 0.85));
    color_gradient.add_key(1.0, Vec4::ZERO);
    let mut size_gradient = bevy_hanabi::Gradient::new();
    size_gradient.add_key(0.0, Vec3::splat(0.025));
    size_gradient.add_key(0.35, Vec3::splat(0.08));
    size_gradient.add_key(1.0, Vec3::splat(0.015));
    let particle_position = SetPositionCone3dModifier {
        base_radius: expression_writer.lit(0.015).expr(),
        top_radius: expression_writer.lit(0.12).expr(),
        height: expression_writer.lit(0.28).expr(),
        dimension: ShapeDimension::Volume,
    };
    let particle_velocity = SetVelocityCircleModifier {
        center: expression_writer.lit(Vec3::Y).expr(),
        axis: expression_writer.lit(Vec3::Y).expr(),
        speed: particle_speed,
    };
    let particle_age_initializer = SetAttributeModifier::new(Attribute::AGE, particle_age);
    let particle_lifetime_initializer = SetAttributeModifier::new(Attribute::LIFETIME, particle_lifetime);
    let particle_rotation_initializer = SetAttributeModifier::new(Attribute::F32_0, particle_rotation);
    let particle_rotation_attribute = expression_writer.attr(Attribute::F32_0).expr();
    let mut expression_module = expression_writer.finish();
    expression_module.add_texture_slot("flame");
    let flame_effect = effects.add(
        EffectAsset::new(48, SpawnerSettings::once(10.0.into()), expression_module)
            .with_name("revolver_muzzle_flame")
            .with_alpha_mode(bevy_hanabi::AlphaMode::Add)
            .init(particle_position)
            .init(particle_velocity)
            .init(particle_age_initializer)
            .init(particle_lifetime_initializer)
            .init(particle_rotation_initializer)
            .render(ParticleTextureModifier {
                texture_slot: particle_texture_slot,
                sample_mapping: ImageSampleMapping::ModulateOpacityFromR,
            })
            .render(OrientModifier {
                mode: OrientMode::FaceCameraPosition,
                rotation: Some(particle_rotation_attribute),
            })
            .render(ColorOverLifetimeModifier::new(color_gradient))
            .render(SizeOverLifetimeModifier {
                gradient: size_gradient,
                screen_space_size: false,
            }),
    );
    let flame_texture_path = "effects/revolver-muzzle-flame.png";
    commands.insert_resource(RevolverMuzzleFlashAssets {
        flame_effect,
        texture: asset_server.load(flame_texture_path),
    });
}

fn spawn_revolver_muzzle_flash(
    mut commands: Commands,
    mut weapon_fired: MessageReader<WeaponFired>,
    revolver_muzzle_flash_assets: Option<Res<RevolverMuzzleFlashAssets>>,
    guns: Query<(Entity, &WeaponSlot), With<Gun>>,
) {
    let Some(revolver_muzzle_flash_assets) = revolver_muzzle_flash_assets else {
        return;
    };
    let revolver_muzzle_position = Vec3::new(2.11, 0.5, 0.0);
    let muzzle_light_position = Vec3::new(2.35, 0.5, 0.0);
    let flash_forward_rotation = Quat::from_rotation_y(std::f32::consts::FRAC_PI_2);
    let flash_duration = Duration::from_millis(33);
    let muzzle_light_color = Color::srgb(1.0, 0.82, 0.58);
    let muzzle_light_intensity = 18_000.0;
    let muzzle_light_range = 3.5;
    let muzzle_light_radius = 0.08;

    for WeaponFired(weapon) in weapon_fired.read() {
        if *weapon != Weapon::Revolver2 {
            continue;
        }
        let Some((revolver_entity, _)) = guns.iter().find(|(_, weapon_slot)| weapon_slot.0 == Weapon::Revolver2) else {
            continue;
        };
        commands.entity(revolver_entity).with_children(|revolver| {
            revolver.spawn((
                RevolverMuzzleFlash,
                MuzzleFlashLifetime(Timer::new(flash_duration, TimerMode::Once)),
                ParticleEffect::new(revolver_muzzle_flash_assets.flame_effect.clone()),
                EffectMaterial {
                    images: vec![revolver_muzzle_flash_assets.texture.clone()],
                },
                Transform::from_translation(revolver_muzzle_position)
                    .with_rotation(flash_forward_rotation),
            ));
            revolver.spawn((
                RevolverMuzzleFlash,
                MuzzleFlashLifetime(Timer::new(flash_duration, TimerMode::Once)),
                PointLight {
                    color: muzzle_light_color,
                    intensity: muzzle_light_intensity,
                    range: muzzle_light_range,
                    radius: muzzle_light_radius,
                    ..default()
                },
                Transform::from_translation(muzzle_light_position),
            ));
        });
    }
}

fn update_muzzle_flashes(
    time: Res<Time>,
    mut commands: Commands,
    mut muzzle_flashes: Query<(Entity, &mut MuzzleFlashLifetime)>,
) {
    for (muzzle_flash_entity, mut muzzle_flash_lifetime) in &mut muzzle_flashes {
        muzzle_flash_lifetime.0.tick(time.delta());
        if muzzle_flash_lifetime.0.is_finished() {
            commands.entity(muzzle_flash_entity).despawn();
        }
    }
}

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
