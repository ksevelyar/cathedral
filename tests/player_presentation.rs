use bevy::camera::visibility::RenderLayers;
use bevy::input::ButtonState;
use bevy::input::keyboard::{Key, KeyboardInput, NativeKey};
use bevy::input::mouse::MouseButtonInput;
use bevy::light::NotShadowReceiver;
use bevy::prelude::*;
use cathedral::app::build_test_app;
use cathedral::maps::{Map, MapBuilder};
use cathedral::player::{Player, PlayerCamera};
use cathedral::shooting::{Gun, ShadowGun, Weapon, WeaponFired};

fn build_map() -> Map {
    let floor_size = Vec3::new(60.0, 0.1, 60.0);
    MapBuilder::new("player_presentation").with_floor(floor_size).build()
}

fn send_key(app: &mut App, key_code: KeyCode, state: ButtonState) {
    app.world_mut()
        .resource_mut::<Messages<KeyboardInput>>()
        .write(KeyboardInput {
            key_code,
            logical_key: Key::Unidentified(NativeKey::Unidentified),
            state,
            text: None,
            repeat: false,
            window: Entity::PLACEHOLDER,
        });
}

fn send_mouse_button(app: &mut App, state: ButtonState) {
    app.world_mut()
        .resource_mut::<Messages<MouseButtonInput>>()
        .write(MouseButtonInput {
            button: MouseButton::Left,
            state,
            window: Entity::PLACEHOLDER,
        });
}

fn find_ancestor_named(app: &App, entity: Entity, wanted_name: &str) -> Option<Entity> {
    let mut current_entity = entity;
    loop {
        let entity_reference = app.world().get_entity(current_entity).ok()?;
        if entity_reference
            .get::<Name>()
            .is_some_and(|name| name.as_str() == wanted_name)
        {
            return Some(current_entity);
        }
        current_entity = entity_reference.get::<ChildOf>()?.parent();
    }
}

fn find_ancestor_with<T: Component>(app: &App, entity: Entity) -> Option<Entity> {
    let mut current_entity = entity;
    loop {
        let entity_reference = app.world().get_entity(current_entity).ok()?;
        if entity_reference.contains::<T>() {
            return Some(current_entity);
        }
        current_entity = entity_reference.get::<ChildOf>()?.parent();
    }
}

fn find_playing_animation_node(app: &App, animation_player: Entity) -> AnimationNodeIndex {
    app.world()
        .get::<AnimationPlayer>(animation_player)
        .expect("player armature should animate")
        .playing_animations()
        .next()
        .map(|(animation_node, _)| *animation_node)
        .expect("player armature should play an animation")
}

#[test]
fn player_presentation_uses_camera_view_models_and_hand_shadow_proxies() {
    let mut app = build_test_app(build_map());
    let maximum_setup_updates = 10_000;
    let (visible_gun, shadow_gun, player_animation_player) = (0..maximum_setup_updates)
        .find_map(|_| {
            app.update();
            let visible_gun = app
                .world_mut()
                .query_filtered::<(Entity, &Visibility), With<Gun>>()
                .iter(app.world())
                .find(|(_, visibility)| **visibility != Visibility::Hidden)
                .map(|(entity, _)| entity)?;
            let visible_weapon = *app.world().get::<Weapon>(visible_gun)?;
            let shadow_gun = app
                .world_mut()
                .query_filtered::<(Entity, &Weapon, &Visibility), With<ShadowGun>>()
                .iter(app.world())
                .find(|(_, weapon, visibility)| **weapon == visible_weapon && **visibility != Visibility::Hidden)
                .map(|(entity, _, _)| entity)?;
            let right_hand = find_ancestor_named(&app, shadow_gun, "hand_r")?;
            let player_animation_player = find_ancestor_with::<AnimationPlayer>(&app, right_hand)?;
            Some((visible_gun, shadow_gun, player_animation_player))
        })
        .expect("player should load visible and shadow weapon representations");

    assert!(find_ancestor_with::<PlayerCamera>(&app, visible_gun).is_some());
    assert!(find_ancestor_named(&app, shadow_gun, "hand_r").is_some());
    assert!(app.world().get::<NotShadowReceiver>(visible_gun).is_some());

    let player_camera = app
        .world_mut()
        .query_filtered::<Entity, With<PlayerCamera>>()
        .single(app.world())
        .expect("one player camera should spawn");
    let player_camera_layers = app
        .world()
        .get::<RenderLayers>(player_camera)
        .cloned()
        .unwrap_or_default();
    let shadow_gun_layers = app
        .world()
        .get::<RenderLayers>(shadow_gun)
        .expect("shadow gun should have shadow-only render layers");
    assert!(!player_camera_layers.intersects(shadow_gun_layers));
    let player_body = app
        .world_mut()
        .query_filtered::<Entity, With<WorldAssetRoot>>()
        .iter(app.world())
        .find(|entity| {
            app.world().get::<RenderLayers>(*entity).is_some() && app.world().get::<ShadowGun>(*entity).is_none()
        })
        .expect("player body should have a shadow-only asset root");
    let player_body_layers = app
        .world()
        .get::<RenderLayers>(player_body)
        .expect("player body should have shadow-only render layers");
    assert!(!player_camera_layers.intersects(player_body_layers));

    let idle_animation = find_playing_animation_node(&app, player_animation_player);
    send_key(&mut app, KeyCode::KeyW, ButtonState::Pressed);
    app.update();
    let moving_animation = find_playing_animation_node(&app, player_animation_player);
    assert_ne!(idle_animation, moving_animation);

    send_key(&mut app, KeyCode::KeyW, ButtonState::Released);
    send_mouse_button(&mut app, ButtonState::Pressed);
    app.update();
    send_mouse_button(&mut app, ButtonState::Released);
    app.update();
    send_key(&mut app, KeyCode::KeyR, ButtonState::Pressed);
    app.update();
    let reloading_animation = find_playing_animation_node(&app, player_animation_player);
    assert_ne!(moving_animation, reloading_animation);
    send_key(&mut app, KeyCode::KeyR, ButtonState::Released);
    let reload_completion_updates = 140;
    for _ in 0..reload_completion_updates {
        app.update();
    }

    let player = app
        .world_mut()
        .query_filtered::<Entity, With<Player>>()
        .single(app.world())
        .expect("one player should spawn");
    let player_camera = app
        .world_mut()
        .query_filtered::<Entity, With<PlayerCamera>>()
        .single(app.world())
        .expect("one player camera should spawn");
    let yaw = 0.4;
    let pitch = -0.2;
    app.world_mut()
        .get_mut::<Transform>(player)
        .expect("player transform")
        .rotation = Quat::from_euler(EulerRot::YXZ, yaw, 0.0, 0.0);
    app.world_mut()
        .get_mut::<Transform>(player_camera)
        .expect("player camera transform")
        .rotation = Quat::from_euler(EulerRot::YXZ, 0.0, pitch, 0.0);
    let mut weapon_fired_cursor = app.world().resource::<Messages<WeaponFired>>().get_cursor_current();
    send_mouse_button(&mut app, ButtonState::Pressed);
    app.update();
    let weapon_fired = weapon_fired_cursor
        .read(app.world().resource::<Messages<WeaponFired>>())
        .next()
        .expect("firing should emit a weapon ray");
    let expected_direction = Quat::from_euler(EulerRot::YXZ, yaw, pitch, 0.0) * Vec3::NEG_Z;
    assert!(
        weapon_fired
            .direction
            .as_vec3()
            .abs_diff_eq(expected_direction, 0.000_1)
    );
}
