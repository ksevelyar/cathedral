use bevy::prelude::*;

use crate::player::Player;
use crate::state::GameState;

pub struct EnemiesPlugin;

impl Plugin for EnemiesPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, spawn_enemies)
            .add_systems(
                Update,
                move_enemies_and_check_reach.run_if(in_state(GameState::Playing)),
            );
    }
}

#[derive(Component)]
pub struct Enemy;

pub const ENEMY_RADIUS: f32 = 0.5;
const ENEMY_MOVE_SPEED: f32 = 1.0;
pub const ENEMY_KILL_DISTANCE: f32 = 0.8;

pub fn spawn_enemies(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let enemy_mesh = meshes.add(Sphere::new(ENEMY_RADIUS));
    let enemy_material = materials.add(StandardMaterial {
        base_color: Color::srgb(0.8, 0.1, 0.1),
        ..default()
    });

    let enemy_positions = [
        Vec3::new(-3.0, ENEMY_RADIUS, -3.0),
        Vec3::new(3.0, ENEMY_RADIUS, -3.0),
        Vec3::new(0.0, ENEMY_RADIUS, -4.0),
    ];

    for position in enemy_positions {
        commands.spawn((
            Enemy,
            Mesh3d(enemy_mesh.clone()),
            MeshMaterial3d(enemy_material.clone()),
            Transform::from_translation(position),
        ));
    }
}

pub fn despawn_enemies(mut commands: Commands, enemy_query: Query<Entity, With<Enemy>>) {
    for enemy_entity in enemy_query.iter() {
        commands.entity(enemy_entity).despawn();
    }
}

pub fn move_enemies_and_check_reach(
    time: Res<Time>,
    player_query: Query<&Transform, (With<Player>, Without<Enemy>)>,
    mut enemy_query: Query<&mut Transform, (With<Enemy>, Without<Player>)>,
    mut next_state: ResMut<NextState<GameState>>,
) {
    let Ok(player_transform) = player_query.single() else {
        return;
    };

    let player_position = player_transform.translation;

    for mut enemy_transform in enemy_query.iter_mut() {
        let distance = player_position.distance(enemy_transform.translation);
        if distance < ENEMY_KILL_DISTANCE {
            next_state.set(GameState::GameOver);
            return;
        }

        let direction_to_player = (player_position - enemy_transform.translation).normalize();
        enemy_transform.translation += direction_to_player * ENEMY_MOVE_SPEED * time.delta_secs();
    }
}
