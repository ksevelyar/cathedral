use bevy::prelude::*;
use bevy::world_serialization::WorldInstanceReady;

use crate::player::Player;
use crate::state::GameState;

pub struct EnemiesPlugin;

impl Plugin for EnemiesPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, spawn_enemies)
            .add_systems(
                Update,
                (
                    move_enemies_and_check_reach,
                    play_death_animation,
                )
                    .run_if(in_state(GameState::Playing)),
            );
    }
}

#[derive(Resource)]
pub struct DeathAnimationGraph {
    pub handle: Handle<AnimationGraph>,
    pub index: AnimationNodeIndex,
}

#[derive(Component)]
pub struct Enemy;

#[derive(Component)]
pub struct Dying;

#[derive(Component)]
struct AnimationToPlay {
    graph_handle: Handle<AnimationGraph>,
    index: AnimationNodeIndex,
}

const ENEMY_MODEL_PATH: &str = "animation/Unreal-Godot/UAL1_Standard.glb";
const SPRINT_ANIMATION_INDEX: usize = 36;
const DEATH_ANIMATION_INDEX: usize = 4;
const ENEMY_MOVE_SPEED: f32 = 1.0;
pub const ENEMY_KILL_DISTANCE: f32 = 1.5;
pub const ENEMY_APPROX_RADIUS: f32 = 1.0;

pub fn spawn_enemies(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut graphs: ResMut<Assets<AnimationGraph>>,
) {
    let (sprint_graph, sprint_index) = AnimationGraph::from_clip(asset_server.load(
        GltfAssetLabel::Animation(SPRINT_ANIMATION_INDEX).from_asset(ENEMY_MODEL_PATH),
    ));
    let sprint_handle = graphs.add(sprint_graph);

    let (death_graph, death_index) = AnimationGraph::from_clip(asset_server.load(
        GltfAssetLabel::Animation(DEATH_ANIMATION_INDEX).from_asset(ENEMY_MODEL_PATH),
    ));
    let death_handle = graphs.add(death_graph);

    commands.insert_resource(DeathAnimationGraph {
        handle: death_handle,
        index: death_index,
    });

    let enemy_positions = [
        Vec3::new(-8.0, 0.0, -8.0),
        Vec3::new(8.0, 0.0, -8.0),
        Vec3::new(0.0, 0.0, -10.0),
    ];

    for position in enemy_positions {
        commands
            .spawn((
                Enemy,
                WorldAssetRoot(
                    asset_server.load(GltfAssetLabel::Scene(0).from_asset(ENEMY_MODEL_PATH)),
                ),
                AnimationToPlay {
                    graph_handle: sprint_handle.clone(),
                    index: sprint_index,
                },
                Transform::from_translation(position),
            ))
            .observe(play_enemy_animation);
    }
}

fn play_enemy_animation(
    scene_ready: On<WorldInstanceReady>,
    mut commands: Commands,
    children: Query<&Children>,
    animations_to_play: Query<&AnimationToPlay>,
    mut players: Query<&mut AnimationPlayer>,
) {
    if let Ok(animation_to_play) = animations_to_play.get(scene_ready.entity) {
        for child in children.iter_descendants(scene_ready.entity) {
            if let Ok(mut player) = players.get_mut(child) {
                player.play(animation_to_play.index).repeat();
                commands
                    .entity(child)
                    .insert(AnimationGraphHandle(animation_to_play.graph_handle.clone()));
            }
        }
    }
}

fn play_death_animation(
    children_query: Query<&Children>,
    mut players: Query<(&mut AnimationPlayer, &mut AnimationGraphHandle)>,
    dying_query: Query<Entity, Added<Dying>>,
    death_graph: Res<DeathAnimationGraph>,
) {
    for entity in dying_query.iter() {
        for child in children_query.iter_descendants(entity) {
            if let Ok((mut player, mut graph_handle)) = players.get_mut(child) {
                player.stop_all();
                *graph_handle = AnimationGraphHandle(death_graph.handle.clone());
                player.play(death_graph.index);
            }
        }
    }
}

pub fn kill_enemy(commands: &mut Commands, entity: Entity) {
    commands.entity(entity).insert(Dying);
}

pub fn despawn_enemies(mut commands: Commands, enemy_query: Query<Entity, With<Enemy>>) {
    for enemy_entity in enemy_query.iter() {
        commands.entity(enemy_entity).despawn();
    }
}

pub fn move_enemies_and_check_reach(
    time: Res<Time>,
    player_query: Query<&Transform, (With<Player>, Without<Enemy>)>,
    mut enemy_query: Query<&mut Transform, (With<Enemy>, Without<Player>, Without<Dying>)>,
    mut next_state: ResMut<NextState<GameState>>,
) {
    let Ok(player_transform) = player_query.single() else {
        return;
    };

    let player_position = player_transform.translation;

    for mut enemy_transform in enemy_query.iter_mut() {
        let diff = player_position - enemy_transform.translation;
        let distance = Vec3::new(diff.x, 0.0, diff.z).length();
        if distance < ENEMY_KILL_DISTANCE {
            next_state.set(GameState::GameOver);
            return;
        }

        let direction_to_player = Vec3::new(diff.x, 0.0, diff.z).normalize_or_zero();
        enemy_transform.translation += direction_to_player * ENEMY_MOVE_SPEED * time.delta_secs();
    }
}
