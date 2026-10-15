mod gltf;
mod hanging_lamp;
mod map01;
mod meshes;
mod structure;

use bevy::light::FogVolume;
use bevy::prelude::*;
use bevy::transform::TransformSystems;

use crate::enemies::{EnemyKind, spawn_enemy};
use crate::player;

pub use gltf::{CardboardBox, Desk, Jerrycan, Laptop, PlasticBottleGallon, PowerBox, Stool, StorageRack};
pub use hanging_lamp::HangingLamp;
pub use structure::{Ceiling, Floor, Wall};

pub struct MapsPlugin;

impl Plugin for MapsPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, spawn_map.before(player::setup_player))
            .add_systems(
                FixedPostUpdate,
                hanging_lamp::update_lamp_wires.after(TransformSystems::Propagate),
            );
    }
}

pub fn build_map01() -> Map {
    map01::build_map()
}

pub trait MapObject: Send + Sync {
    fn spawn(&self, context: &mut SpawnContext, transform: Transform);
}

pub struct SpawnContext<'commands, 'world, 'state> {
    pub(crate) commands: &'commands mut Commands<'world, 'state>,
    pub(crate) asset_server: &'commands AssetServer,
    pub(crate) meshes: &'commands mut Assets<Mesh>,
    pub(crate) materials: &'commands mut Assets<StandardMaterial>,
}

impl<'commands, 'world, 'state> SpawnContext<'commands, 'world, 'state> {
    pub fn new(
        commands: &'commands mut Commands<'world, 'state>,
        asset_server: &'commands AssetServer,
        meshes: &'commands mut Assets<Mesh>,
        materials: &'commands mut Assets<StandardMaterial>,
    ) -> Self {
        Self {
            commands,
            asset_server,
            meshes,
            materials,
        }
    }
}

pub struct ObjectSpawn {
    pub transform: Transform,
    pub object: Box<dyn MapObject>,
}

impl ObjectSpawn {
    pub fn new(transform: Transform, object: impl MapObject + 'static) -> Self {
        Self {
            transform,
            object: Box::new(object),
        }
    }
}

pub struct MapBuilder {
    name: &'static str,
    ambient: GlobalAmbientLight,
    clear_color: ClearColor,
    fog_volume: Option<FogVolumeSettings>,
    player_position: PlayerPosition,
    enemies: Vec<EnemySpawn>,
    objects: Vec<ObjectSpawn>,
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
        self.with_object(ObjectSpawn::new(Transform::IDENTITY, Floor { size }))
    }

    pub fn with_wall(self, position: Vec3, size: Vec3) -> Self {
        self.with_object(ObjectSpawn::new(Transform::from_translation(position), Wall { size }))
    }

    pub fn with_enemy(mut self, position: Vec3, kind: EnemyKind) -> Self {
        self.enemies.push(EnemySpawn::new(position, kind));
        self
    }

    fn with_object(mut self, object: ObjectSpawn) -> Self {
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

#[derive(Resource)]
pub struct Map {
    pub(crate) name: &'static str,
    pub(crate) ambient: GlobalAmbientLight,
    pub(crate) clear_color: ClearColor,
    pub(crate) fog_volume: Option<FogVolumeSettings>,
    pub(crate) player_position: PlayerPosition,
    pub(crate) enemies: Vec<EnemySpawn>,
    pub(crate) objects: Vec<ObjectSpawn>,
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
    pub rotation: Quat,
    pub kind: EnemyKind,
}

impl EnemySpawn {
    pub fn new(position: Vec3, kind: EnemyKind) -> Self {
        Self {
            position,
            rotation: Quat::IDENTITY,
            kind,
        }
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
    let mut spawn_context = SpawnContext::new(&mut commands, &asset_server, &mut meshes, &mut materials);
    for object_spawn in &map.objects {
        object_spawn.object.spawn(&mut spawn_context, object_spawn.transform);
    }
    spawn_enemies(&mut *spawn_context.commands, spawn_context.asset_server, &map.enemies);
}

fn spawn_enemies(commands: &mut Commands, asset_server: &AssetServer, enemies: &[EnemySpawn]) {
    for (enemy_index, enemy_spawn) in enemies.iter().enumerate() {
        spawn_enemy(
            commands,
            asset_server,
            enemy_spawn.kind,
            enemy_index,
            Transform::from_translation(enemy_spawn.position).with_rotation(enemy_spawn.rotation),
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
