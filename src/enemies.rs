use bevy::prelude::*;
use bevy::world_serialization::WorldInstanceReady;

use crate::player::Player;
use crate::ragdoll::setup_ragdoll;
use crate::state::GameState;

pub struct EnemiesPlugin;

impl Plugin for EnemiesPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<EnemyBehaviorEnabled>()
            .add_systems(
                Update,
                move_enemies_and_check_reach.run_if(in_state(GameState::Playing)),
            )
            .add_systems(PostUpdate, stop_enemy_animation);
    }
}

#[derive(Resource, Default)]
struct EnemyBehaviorEnabled;

#[derive(Component)]
pub struct Enemy;

#[derive(Component)]
pub struct Dying;

const ENEMY_MODEL_PATH: &str = "animation/Unreal-Godot/UAL1_Standard.glb";
const SPRINT_ANIMATION_INDEX: usize = 36;
const ENEMY_MOVE_SPEED: f32 = 1.0;
const ENEMY_REACH_DISTANCE: f32 = 1.5;

#[derive(Component)]
struct SprintAnimation(Handle<AnimationClip>);

#[derive(Clone, Copy)]
pub enum EnemyKind {
    Standard,
}

impl EnemyKind {
    fn model_path(self) -> &'static str {
        match self {
            Self::Standard => ENEMY_MODEL_PATH,
        }
    }
}

#[derive(Clone, Copy)]
pub enum EnemyLifeState {
    Alive,
    Dead,
}

pub struct EnemySpawn {
    pub kind: EnemyKind,
    pub transform: Transform,
    pub life_state: EnemyLifeState,
}

pub fn spawn_enemy(commands: &mut Commands, asset_server: &AssetServer, spawn: EnemySpawn) -> Entity {
    let mut enemy = commands.spawn((
        Enemy,
        WorldAssetRoot(asset_server.load(GltfAssetLabel::Scene(0).from_asset(spawn.kind.model_path()))),
        spawn.transform,
    ));
    match spawn.life_state {
        EnemyLifeState::Alive => {
            enemy.insert(SprintAnimation(asset_server.load(
                GltfAssetLabel::Animation(SPRINT_ANIMATION_INDEX).from_asset(spawn.kind.model_path()),
            )));
        }
        EnemyLifeState::Dead => {
            enemy.insert(Dying);
        }
    }
    enemy.observe(setup_ragdoll).observe(play_sprint_animation).id()
}

fn play_sprint_animation(
    scene_ready: On<WorldInstanceReady>,
    mut commands: Commands,
    children: Query<&Children>,
    sprint_animations: Query<&SprintAnimation>,
    mut players: Query<&mut AnimationPlayer>,
    mut graphs: ResMut<Assets<AnimationGraph>>,
    behavior_enabled: Option<Res<EnemyBehaviorEnabled>>,
) {
    if behavior_enabled.is_none() {
        return;
    }
    let Ok(sprint_animation) = sprint_animations.get(scene_ready.entity) else {
        return;
    };
    let (graph, animation) = AnimationGraph::from_clip(sprint_animation.0.clone());
    let graph = graphs.add(graph);

    for descendant in children.iter_descendants(scene_ready.entity) {
        if let Ok(mut player) = players.get_mut(descendant) {
            player.play(animation).repeat();
            commands.entity(descendant).insert(AnimationGraphHandle(graph.clone()));
        }
    }
}

fn stop_enemy_animation(
    children: Query<&Children>,
    dying_enemies: Query<Entity, Added<Dying>>,
    mut players: Query<&mut AnimationPlayer>,
) {
    for enemy in &dying_enemies {
        for descendant in children.iter_descendants(enemy) {
            if let Ok(mut player) = players.get_mut(descendant) {
                player.stop_all();
            }
        }
    }
}

type AliveEnemy = (With<Enemy>, Without<Dying>, Without<Player>);

fn move_enemies_and_check_reach(
    time: Res<Time>,
    player: Query<&Transform, (With<Player>, Without<Enemy>)>,
    mut enemies: Query<&mut Transform, AliveEnemy>,
    mut next_state: ResMut<NextState<GameState>>,
) {
    let Ok(player_transform) = player.single() else {
        return;
    };

    for mut enemy_transform in &mut enemies {
        let offset = player_transform.translation - enemy_transform.translation;
        let planar_offset = Vec3::new(offset.x, 0.0, offset.z);
        let distance = planar_offset.length();
        if distance <= ENEMY_REACH_DISTANCE {
            next_state.set(GameState::GameOver);
            return;
        }

        let direction = planar_offset / distance;
        enemy_transform.look_to(-direction, Vec3::Y);
        let available_distance = distance - ENEMY_REACH_DISTANCE;
        let movement = (ENEMY_MOVE_SPEED * time.delta_secs()).min(available_distance);
        enemy_transform.translation += direction * movement;

        if movement >= available_distance {
            next_state.set(GameState::GameOver);
            return;
        }
    }
}
