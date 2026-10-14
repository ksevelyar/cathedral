mod map01;
mod meshes;
mod objects;

use bevy::light::FogVolume;
use bevy::prelude::*;
use bevy::transform::TransformSystems;

use crate::enemies::{EnemyKind, spawn_enemy};
use crate::player;
use objects::{Object, ObjectSpawnContext};

pub struct MapsPlugin;

impl Plugin for MapsPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, spawn_map.before(player::setup_player))
            .add_systems(
                FixedPostUpdate,
                objects::update_lamp_wires.after(TransformSystems::Propagate),
            );
    }
}

pub fn build_map01() -> Map {
    map01::build_map()
}

#[derive(Clone)]
pub struct MapBuilder {
    name: &'static str,
    ambient: GlobalAmbientLight,
    clear_color: ClearColor,
    fog_volume: Option<FogVolumeSettings>,
    player_position: PlayerPosition,
    enemies: Vec<EnemySpawn>,
    objects: Vec<Object>,
}

impl MapBuilder {
    pub fn new(name: &'static str) -> Self {
        Self {
            name,
            ambient: GlobalAmbientLight::default(),
            clear_color: ClearColor(Color::srgb(0.05, 0.05, 0.05)),
            fog_volume: None,
            player_position: PlayerPosition::new(Vec3::new(0.0, 0.5, 0.0)),
            enemies: vec![],
            objects: vec![],
        }
    }

    pub fn with_player_position(self, position: Vec3, look_at: Vec3) -> Self {
        Self {
            player_position: PlayerPosition { position, look_at },
            ..self
        }
    }

    pub fn with_floor(self, size: Vec3) -> Self {
        self.with_object(Object::Floor {
            transform: Transform::IDENTITY,
            size,
        })
    }

    pub fn with_wall(self, position: Vec3, size: Vec3) -> Self {
        self.with_object(Object::Wall {
            transform: Transform::from_translation(position),
            size,
        })
    }

    pub fn with_enemy(mut self, position: Vec3, kind: EnemyKind) -> Self {
        self.enemies.push(EnemySpawn::new(position, kind));
        self
    }

    fn with_object(mut self, object: Object) -> Self {
        self.objects.push(object);
        self
    }

    pub fn build(self) -> Map {
        Map {
            name: self.name,
            ambient: self.ambient,
            clear_color: self.clear_color,
            fog_volume: self.fog_volume,
            player_position: self.player_position,
            enemies: self.enemies,
            objects: self.objects,
        }
    }
}

#[derive(Resource, Clone, Copy)]
pub struct PlayerPosition {
    pub position: Vec3,
    pub look_at: Vec3,
}

impl PlayerPosition {
    pub fn new(position: Vec3) -> Self {
        Self {
            position,
            look_at: position + Vec3::NEG_Z,
        }
    }
}

#[derive(Resource, Clone)]
pub struct Map {
    pub(crate) name: &'static str,
    pub(crate) ambient: GlobalAmbientLight,
    pub(crate) clear_color: ClearColor,
    pub(crate) fog_volume: Option<FogVolumeSettings>,
    pub(crate) player_position: PlayerPosition,
    pub(crate) enemies: Vec<EnemySpawn>,
    pub(crate) objects: Vec<Object>,
}

#[derive(Clone, Copy)]
pub(crate) struct FogVolumeSettings {
    pub center: Vec3,
    pub size: Vec3,
    pub color: Color,
    pub density: f32,
    pub absorption: f32,
    pub scattering: f32,
    pub scattering_asymmetry: f32,
}

#[derive(Clone, Copy)]
pub(crate) struct EnemySpawn {
    pub position: Vec3,
    pub kind: EnemyKind,
}

impl EnemySpawn {
    pub fn new(position: Vec3, kind: EnemyKind) -> Self {
        Self { position, kind }
    }
}

fn spawn_map(
    map: Res<Map>,
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    info!("spawning map {}", map.name);
    commands.insert_resource(map.player_position);
    commands.insert_resource(map.ambient.clone());
    commands.insert_resource(map.clear_color.clone());
    if let Some(fog_volume) = map.fog_volume {
        commands.spawn((
            FogVolume {
                fog_color: fog_volume.color,
                density_factor: fog_volume.density,
                absorption: fog_volume.absorption,
                scattering: fog_volume.scattering,
                scattering_asymmetry: fog_volume.scattering_asymmetry,
                ..default()
            },
            Transform::from_translation(fog_volume.center).with_scale(fog_volume.size),
        ));
    }
    {
        let mut object_spawn_context =
            ObjectSpawnContext::new(&mut commands, &asset_server, &mut meshes, &mut materials);
        for object in map.objects.clone() {
            object.spawn(&mut object_spawn_context);
        }
    }
    spawn_enemies(&mut commands, &asset_server, &map.enemies);
}

fn spawn_enemies(commands: &mut Commands, asset_server: &AssetServer, enemies: &[EnemySpawn]) {
    for (enemy_index, enemy_spawn) in enemies.iter().enumerate() {
        spawn_enemy(
            commands,
            asset_server,
            enemy_spawn.kind,
            enemy_index,
            Transform::from_translation(enemy_spawn.position),
            true,
        );
    }
}

pub(crate) fn restart(
    map: &mut Map,
    player_position: &mut PlayerPosition,
    commands: &mut Commands,
    asset_server: &AssetServer,
) {
    *map = build_map01();
    *player_position = map.player_position;
    spawn_enemies(commands, asset_server, &map.enemies);
}
