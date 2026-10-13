use std::time::Duration;

use bevy::prelude::*;
use bevy::world_serialization::WorldInstanceReady;

use super::{Weapon, WeaponFired};
use crate::state::GameState;

#[derive(Component)]
pub struct MuzzleFlash {
    weapon_entity: Entity,
    timer: Timer,
}

pub const MUZZLE_FLASH_FORWARD_OFFSET_BOUNDS: std::ops::RangeInclusive<f32> = 0.07..=0.11;
pub const MUZZLE_FLASH_SCALE_BOUNDS: std::ops::RangeInclusive<f32> = 0.9..=1.16;
pub const MUZZLE_FLASH_LIGHT_INTENSITY_BOUNDS: std::ops::RangeInclusive<f32> = 1_350.0..=1_950.0;
pub const MUZZLE_FLASH_LIGHT_RANGE_BOUNDS: std::ops::RangeInclusive<f32> = 1.0..=1.35;

pub(super) fn configure(app: &mut App) {
    app.add_systems(
        Update,
        (activate_muzzle_flash, update_muzzle_flash)
            .chain()
            .run_if(in_state(GameState::Playing))
            .after(super::fire_weapon),
    );
}

pub(crate) fn observe_muzzle_flash(commands: &mut Commands, weapon_entity: Entity) {
    commands.entity(weapon_entity).observe(setup_muzzle_flash);
}

fn setup_muzzle_flash(
    scene_ready: On<WorldInstanceReady>,
    mut commands: Commands,
    weapon_entities: Query<&Weapon>,
    children: Query<&Children>,
    names: Query<&Name>,
) {
    let Ok(Weapon::Ranged(_)) = weapon_entities.get(scene_ready.entity) else {
        return;
    };
    let Some(muzzle_entity) = find_named_descendant(scene_ready.entity, "Muzzle", &children, &names) else {
        return;
    };
    let muzzle_flash_duration = Duration::from_millis(60);
    commands.entity(muzzle_entity).with_child((
        MuzzleFlash {
            weapon_entity: scene_ready.entity,
            timer: Timer::new(muzzle_flash_duration, TimerMode::Once),
        },
        PointLight {
            color: Color::linear_rgb(1.0, 0.3, 0.03),
            intensity: *MUZZLE_FLASH_LIGHT_INTENSITY_BOUNDS.start(),
            range: *MUZZLE_FLASH_LIGHT_RANGE_BOUNDS.start(),
            shadow_maps_enabled: false,
            ..default()
        },
        Visibility::Hidden,
    ));
}

fn find_named_descendant(
    entity: Entity,
    expected_name: &str,
    children: &Query<&Children>,
    names: &Query<&Name>,
) -> Option<Entity> {
    if names.get(entity).is_ok_and(|name| name.as_str() == expected_name) {
        return Some(entity);
    }
    let Ok(entity_children) = children.get(entity) else {
        return None;
    };
    entity_children
        .iter()
        .find_map(|child| find_named_descendant(child, expected_name, children, names))
}

fn activate_muzzle_flash(
    mut weapon_fired: MessageReader<WeaponFired>,
    mut muzzle_flashes: Query<(&mut MuzzleFlash, &mut PointLight, &mut Visibility)>,
) {
    for weapon_fired_message in weapon_fired.read() {
        for (mut muzzle_flash, mut point_light, mut visibility) in &mut muzzle_flashes {
            if muzzle_flash.weapon_entity != weapon_fired_message.weapon_entity {
                continue;
            }
            muzzle_flash.timer.reset();
            point_light.intensity = *MUZZLE_FLASH_LIGHT_INTENSITY_BOUNDS.start();
            point_light.range = *MUZZLE_FLASH_LIGHT_RANGE_BOUNDS.start();
            *visibility = Visibility::Visible;
        }
    }
}

fn update_muzzle_flash(time: Res<Time>, mut muzzle_flashes: Query<(&mut MuzzleFlash, &mut Visibility)>) {
    for (mut muzzle_flash, mut visibility) in &mut muzzle_flashes {
        if *visibility == Visibility::Hidden {
            continue;
        }
        muzzle_flash.timer.tick(time.delta());
        if muzzle_flash.timer.is_finished() {
            *visibility = Visibility::Hidden;
        }
    }
}
