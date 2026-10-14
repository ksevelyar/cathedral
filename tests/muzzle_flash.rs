use bevy::input::ButtonState;
use bevy::input::mouse::MouseButtonInput;
use bevy::prelude::*;
use cathedral::app::build_test_app;
use cathedral::enemies::{Enemy, EnemyKind};
use cathedral::maps::{Map, MapBuilder};
use cathedral::shooting::{Gun, MUZZLE_FLASH_LIGHT_INTENSITY_BOUNDS, MUZZLE_FLASH_LIGHT_RANGE_BOUNDS, MuzzleFlash};

pub mod support;

const FLOOR_SIZE: Vec3 = Vec3::new(60.0, 0.1, 60.0);

fn build_map_with_gunner_and_fighter() -> Map {
    let gunner_position = Vec3::new(-2.0, 0.5, 0.0);
    let fighter_position = Vec3::new(2.0, 0.5, 0.0);
    MapBuilder::new("muzzle_flash")
        .with_floor(FLOOR_SIZE)
        .with_enemy(gunner_position, EnemyKind::Gunner)
        .with_enemy(fighter_position, EnemyKind::Fighter)
        .build()
}

fn build_empty_map() -> Map {
    MapBuilder::new("muzzle_flash").with_floor(FLOOR_SIZE).build()
}

fn has_visible_gun_ancestor(app: &App, mut entity: Entity) -> bool {
    loop {
        let entity_reference = app.world().entity(entity);
        if entity_reference.contains::<Gun>() {
            return entity_reference
                .get::<Visibility>()
                .is_some_and(|visibility| *visibility != Visibility::Hidden);
        }
        let Some(parent) = entity_reference.get::<ChildOf>() else {
            return false;
        };
        entity = parent.parent();
    }
}

#[test]
fn firing_player_weapon_shows_and_hides_muzzle_flash() {
    let mut app = build_test_app(build_empty_map());
    let maximum_setup_updates = 10_000;
    let mut muzzle_flash = None;
    for _ in 0..maximum_setup_updates {
        app.update();
        if let Some(selected_muzzle_flash) = app
            .world_mut()
            .query_filtered::<(Entity, &ChildOf), With<MuzzleFlash>>()
            .iter(app.world())
            .find(|(_, muzzle_flash_parent)| has_visible_gun_ancestor(&app, muzzle_flash_parent.parent()))
        {
            muzzle_flash = Some(selected_muzzle_flash.0);
            break;
        }
        std::thread::yield_now();
    }
    let muzzle_flash = muzzle_flash.expect("selected player weapon should have a muzzle flash");
    assert_eq!(app.world().get::<Visibility>(muzzle_flash), Some(&Visibility::Hidden));
    assert!(app.world().get::<PointLight>(muzzle_flash).is_some());

    let mut mouse_button_messages = app.world_mut().resource_mut::<Messages<MouseButtonInput>>();
    mouse_button_messages.write(MouseButtonInput {
        button: MouseButton::Left,
        state: ButtonState::Pressed,
        window: Entity::PLACEHOLDER,
    });
    app.update();
    assert_eq!(app.world().get::<Visibility>(muzzle_flash), Some(&Visibility::Visible));
    let muzzle_flash_light = app
        .world()
        .get::<PointLight>(muzzle_flash)
        .expect("muzzle flash point light");
    assert!(MUZZLE_FLASH_LIGHT_INTENSITY_BOUNDS.contains(&muzzle_flash_light.intensity));
    assert!(MUZZLE_FLASH_LIGHT_RANGE_BOUNDS.contains(&muzzle_flash_light.range));

    let muzzle_flash_lifetime_updates = 7;
    for _ in 0..muzzle_flash_lifetime_updates {
        app.update();
    }
    assert_eq!(app.world().get::<Visibility>(muzzle_flash), Some(&Visibility::Hidden));
    assert!(app.world().get::<PointLight>(muzzle_flash).is_some());
}

fn count_enemy_muzzle_flashes(app: &mut App) -> usize {
    app.world_mut()
        .query_filtered::<&ChildOf, With<MuzzleFlash>>()
        .iter(app.world())
        .filter(|muzzle_flash_parent| has_enemy_ancestor(app, muzzle_flash_parent.parent()))
        .count()
}

fn has_enemy_ancestor(app: &App, mut entity: Entity) -> bool {
    loop {
        let entity_reference = app.world().entity(entity);
        if entity_reference.contains::<Enemy>() {
            return true;
        }
        let Some(parent) = entity_reference.get::<ChildOf>() else {
            return false;
        };
        entity = parent.parent();
    }
}

fn count_weapon_entities(app: &mut App) -> usize {
    app.world_mut()
        .query_filtered::<Entity, With<Name>>()
        .iter(app.world())
        .filter(|entity| {
            app.world()
                .get::<Name>(*entity)
                .is_some_and(|name| name.as_str() == "Weapon")
        })
        .count()
}

#[test]
fn gunner_weapon_has_muzzle_flash_but_fighter_weapon_does_not() {
    let mut app = build_test_app(build_map_with_gunner_and_fighter());
    let maximum_setup_updates = 10_000;
    for _ in 0..maximum_setup_updates {
        app.update();
        let weapon_count = count_weapon_entities(&mut app);
        let enemy_muzzle_flash_count = count_enemy_muzzle_flashes(&mut app);
        if weapon_count == 2 && enemy_muzzle_flash_count == 1 {
            break;
        }
        std::thread::yield_now();
    }

    let weapon_count = count_weapon_entities(&mut app);
    let enemy_muzzle_flash_count = count_enemy_muzzle_flashes(&mut app);
    let enemy_point_light_count = app
        .world_mut()
        .query_filtered::<(&ChildOf, &PointLight), With<MuzzleFlash>>()
        .iter(app.world())
        .filter(|(muzzle_flash_parent, _)| has_enemy_ancestor(&app, muzzle_flash_parent.parent()))
        .count();
    assert_eq!(weapon_count, 2);
    assert_eq!(enemy_muzzle_flash_count, 1);
    assert_eq!(enemy_point_light_count, 1);
}
