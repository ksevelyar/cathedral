use bevy::diagnostic::{DiagnosticsStore, FrameTimeDiagnosticsPlugin};
use bevy::prelude::*;
use bevy::window::{CursorGrabMode, CursorOptions};

use crate::shooting::{Crosshair, Gun};
use crate::state::GameState;

pub struct UiPlugin {
    pub show_fps: bool,
}

impl Plugin for UiPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(OnEnter(GameState::Playing), lock_cursor)
            .add_systems(OnEnter(GameState::Paused), show_cursor)
            .add_systems(OnEnter(GameState::Paused), setup_pause_menu)
            .add_systems(OnEnter(GameState::GameOver), setup_game_over_menu)
            .add_systems(Update, update_crosshair_and_gun_visibility);
        if self.show_fps {
            app.add_plugins(FrameTimeDiagnosticsPlugin::default())
                .add_systems(Startup, setup_fps_display)
                .add_systems(Update, update_fps_display);
        }
    }
}

#[derive(Component)]
struct FpsDisplay;

fn setup_fps_display(mut commands: Commands) {
    let fps_text_color = Color::srgb(0.55, 0.8, 1.0);
    commands.spawn((
        FpsDisplay,
        Text::new("FPS --"),
        TextFont {
            font_size: FontSize::Px(18.0),
            ..default()
        },
        TextColor(fps_text_color),
        Node {
            position_type: PositionType::Absolute,
            top: px(12),
            right: px(12),
            ..default()
        },
    ));
}

fn update_fps_display(diagnostics: Res<DiagnosticsStore>, mut fps_displays: Query<&mut Text, With<FpsDisplay>>) {
    let Some(fps) = diagnostics.get(&FrameTimeDiagnosticsPlugin::FPS) else {
        return;
    };
    let Some(fps_value) = fps.smoothed() else {
        return;
    };
    let fps_text = format!("FPS {fps_value:.0}");
    for mut fps_display in &mut fps_displays {
        fps_display.0.clone_from(&fps_text);
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
