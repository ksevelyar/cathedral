use bevy::prelude::*;
use bevy::window::{CursorGrabMode, CursorOptions};

use crate::shooting::{Crosshair, Gun};
use crate::state::GameState;

pub struct UiPlugin;

impl Plugin for UiPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(OnEnter(GameState::Playing), lock_cursor)
            .add_systems(OnEnter(GameState::Paused), show_cursor)
            .add_systems(OnEnter(GameState::Paused), setup_pause_menu)
            .add_systems(OnEnter(GameState::GameOver), setup_game_over_menu)
            .add_systems(Update, update_crosshair_and_gun_visibility);
    }
}

fn lock_cursor(mut query: Query<&mut CursorOptions, With<Window>>) {
    let Ok(mut cursor_options) = query.single_mut() else {
        return;
    };
    cursor_options.grab_mode = CursorGrabMode::Locked;
    cursor_options.visible = false;
}

fn show_cursor(mut query: Query<&mut CursorOptions, With<Window>>) {
    let Ok(mut cursor_options) = query.single_mut() else {
        return;
    };
    cursor_options.grab_mode = CursorGrabMode::None;
    cursor_options.visible = true;
}

pub fn setup_pause_menu(mut commands: Commands) {
    commands.spawn((
        DespawnOnExit(GameState::Paused),
        Node {
            width: Val::Percent(100.0),
            height: Val::Percent(100.0),
            justify_content: JustifyContent::Center,
            align_items: AlignItems::Center,
            ..default()
        },
        children![(
            Text::new("Press Esc to continue"),
            TextFont {
                font_size: FontSize::Px(48.0),
                ..default()
            },
            TextColor(Color::WHITE),
        )],
    ));
}

fn setup_game_over_menu(mut commands: Commands) {
    commands.spawn((
        DespawnOnExit(GameState::GameOver),
        Node {
            width: Val::Percent(100.0),
            height: Val::Percent(100.0),
            flex_direction: FlexDirection::Column,
            justify_content: JustifyContent::Center,
            align_items: AlignItems::Center,
            ..default()
        },
        children![
            (
                Text::new("skill issue!"),
                TextFont {
                    font_size: FontSize::Px(64.0),
                    ..default()
                },
                TextColor(Color::srgb(0.9, 0.2, 0.2)),
            ),
            (
                Text::new("Press Space to restart"),
                TextFont {
                    font_size: FontSize::Px(32.0),
                    ..default()
                },
                TextColor(Color::srgb(0.7, 0.7, 0.7)),
            ),
        ],
    ));
}

fn update_crosshair_and_gun_visibility(
    state: Res<State<GameState>>,
    mut crosshair_query: Query<&mut Visibility, With<Crosshair>>,
    mut gun_query: Query<&mut Visibility, (With<Gun>, Without<Crosshair>)>,
) {
    let should_be_visible = matches!(state.get(), GameState::Playing);

    if let Ok(mut crosshair_visibility) = crosshair_query.single_mut() {
        *crosshair_visibility = if should_be_visible {
            Visibility::Visible
        } else {
            Visibility::Hidden
        };
    }

    if let Ok(mut gun_visibility) = gun_query.single_mut() {
        *gun_visibility = if should_be_visible {
            Visibility::Visible
        } else {
            Visibility::Hidden
        };
    }
}
