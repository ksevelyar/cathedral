use bevy::input::ButtonState;
use bevy::input::keyboard::{Key, KeyboardInput, NativeKey};
use bevy::input::mouse::MouseButtonInput;
use bevy::prelude::*;
use cathedral::app::build_test_app;
use cathedral::maps::{Map, MapBuilder};
use cathedral::player::Player;
use cathedral::shooting::Gun;

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

fn find_selected_gun(app: &mut App) -> Entity {
    app.world_mut()
        .query_filtered::<(Entity, &Visibility), With<Gun>>()
        .iter(app.world())
        .find(|(_, visibility)| **visibility != Visibility::Hidden)
        .map(|(entity, _)| entity)
        .expect("selected player gun should be visible")
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

fn find_ancestor_animation_player(app: &App, entity: Entity) -> Option<Entity> {
    let mut current_entity = entity;
    loop {
        let entity_reference = app.world().get_entity(current_entity).ok()?;
        if entity_reference.contains::<AnimationPlayer>() {
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
fn player_gun_and_animations_follow_player_actions() {
    let mut app = build_test_app(build_map());
    let maximum_setup_updates = 10_000;
    let (selected_gun, player_animation_player) = (0..maximum_setup_updates)
        .find_map(|_| {
            app.update();
            let selected_gun = find_selected_gun(&mut app);
            let right_hand = find_ancestor_named(&app, selected_gun, "hand_r")?;
            let player_animation_player = find_ancestor_animation_player(&app, right_hand)?;
            Some((selected_gun, player_animation_player))
        })
        .expect("selected player gun should attach to the animated right hand");
    let idle_animation = find_playing_animation_node(&app, player_animation_player);

    let player = app
        .world_mut()
        .query_filtered::<Entity, With<Player>>()
        .single(app.world())
        .expect("one player should spawn");
    let starting_position = app
        .world()
        .get::<Transform>(player)
        .expect("player transform")
        .translation;
    send_key(&mut app, KeyCode::KeyW, ButtonState::Pressed);
    app.update();
    let moving_animation = find_playing_animation_node(&app, player_animation_player);
    let ending_position = app
        .world()
        .get::<Transform>(player)
        .expect("player transform")
        .translation;
    assert_ne!(
        idle_animation, moving_animation,
        "moving should select a different animation"
    );
    assert!(ending_position.z < starting_position.z, "player should move forward");

    send_key(&mut app, KeyCode::KeyW, ButtonState::Released);
    send_mouse_button(&mut app, ButtonState::Pressed);
    app.update();
    send_mouse_button(&mut app, ButtonState::Released);
    app.update();
    send_key(&mut app, KeyCode::KeyR, ButtonState::Pressed);
    app.update();
    app.update();
    let reloading_animation = find_playing_animation_node(&app, player_animation_player);
    assert_ne!(
        moving_animation, reloading_animation,
        "reloading should select a different animation"
    );
    assert!(
        app.world().get_entity(selected_gun).is_ok(),
        "selected gun should remain attached while reloading"
    );
}
