mod gltf;
mod hanging_lamp;
mod intro;
mod map01;
mod meshes;
mod moon;
mod sandfield;
mod structure;

use bevy::light::FogVolume;
use bevy::prelude::*;
use bevy::transform::TransformSystems;

use crate::enemies::{EnemyBehavior, EnemyKind, EnemySpawnConfiguration, spawn_enemy};
use crate::player;

pub use gltf::{CardboardBox, Desk, Jerrycan, Laptop, PlasticBottleGallon, PowerBox, Stool, StorageRack};
pub use hanging_lamp::HangingLamp;
pub use sandfield::{SandField, SandFieldGeometry, SandFieldOpening};
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

pub fn build_intro() -> Map {
    intro::build_map()
}

pub fn build_map01() -> Map {
    map01::build_map()
}

pub fn build_named_map(name: &str) -> Option<Map> {
    match name {
        "intro" => Some(build_intro()),
        "map01" => Some(build_map01()),
        _ => None,
    }
}

pub trait MapObject: Send + Sync {
    fn spawn(&self, context: &mut SpawnContext, transform: Transform);
}

pub struct SpawnContext<'commands, 'world, 'state> {
    pub(crate) commands: &'commands mut Commands<'world, 'state>,
    pub(crate) asset_server: &'commands AssetServer,
    pub(crate) meshes: &'commands mut Assets<Mesh>,
    pub(crate) images: &'commands mut Assets<Image>,
    pub(crate) materials: &'commands mut Assets<StandardMaterial>,
}

impl<'commands, 'world, 'state> SpawnContext<'commands, 'world, 'state> {
    pub fn new(
        commands: &'commands mut Commands<'world, 'state>,
        asset_server: &'commands AssetServer,
        meshes: &'commands mut Assets<Mesh>,
        images: &'commands mut Assets<Image>,
        materials: &'commands mut Assets<StandardMaterial>,
    ) -> Self {
        Self {
            commands,
            asset_server,
            meshes,
            images,
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
    viewpoints: Vec<PlayerPosition>,
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
            viewpoints: vec![PlayerPosition::new("Default", Vec3::new(0.0, 0.5, 0.0))],
            enemies: vec![],
            objects: vec![],
        }
    }

    pub fn with_player_position(self, position: Vec3, look_at: Vec3) -> Self {
        Self {
            viewpoints: vec![PlayerPosition {
                label: "default",
                position,
                look_at,
            }],
            ..self
        }
    }

    pub fn with_viewpoints(self, viewpoints: Vec<PlayerPosition>) -> Self {
        Self { viewpoints, ..self }
    }

    pub fn with_floor(self, size: Vec3) -> Self {
        self.with_object(ObjectSpawn::new(Transform::IDENTITY, Floor { size }))
    }

    pub fn with_wall(self, position: Vec3, size: Vec3) -> Self {
        self.with_object(ObjectSpawn::new(Transform::from_translation(position), Wall { size }))
    }

    pub fn with_enemy(
        mut self,
        positions: Vec<Vec3>,
        look_at: Vec3,
        kind: EnemyKind,
        initial_behavior: EnemyBehavior,
    ) -> Self {
        self.enemies
            .push(EnemySpawn::facing(positions, look_at, kind, initial_behavior));
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
            viewpoints: self.viewpoints,
            enemies: self.enemies,
            objects: self.objects,
        }
    }
}

#[derive(Resource, Clone, Copy)]
pub struct PlayerPosition {
    pub label: &'static str,
    pub position: Vec3,
    pub look_at: Vec3,
}

impl PlayerPosition {
    pub fn new(label: &'static str, position: Vec3) -> Self {
        Self {
            label,
            position,
            look_at: position + Vec3::NEG_Z,
        }
    }
}

#[derive(Resource, Default)]
pub struct ActiveViewpointIndex(pub usize);

#[derive(Resource)]
pub struct Map {
    pub(crate) name: &'static str,
    pub(crate) ambient: GlobalAmbientLight,
    pub(crate) clear_color: ClearColor,
    pub(crate) fog_volume: Option<FogVolumeSettings>,
    pub(crate) viewpoints: Vec<PlayerPosition>,
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

#[derive(Clone)]
pub(crate) struct EnemySpawn {
    pub positions: Vec<Vec3>,
    pub rotation: Quat,
    pub kind: EnemyKind,
    pub initial_behavior: EnemyBehavior,
}

impl EnemySpawn {
    pub fn new(positions: Vec<Vec3>, kind: EnemyKind, initial_behavior: EnemyBehavior) -> Self {
        assert!(!positions.is_empty(), "enemy positions should contain a spawn position");
        Self {
            positions,
            rotation: Quat::IDENTITY,
            kind,
            initial_behavior,
        }
    }

    pub fn facing(positions: Vec<Vec3>, look_at: Vec3, kind: EnemyKind, initial_behavior: EnemyBehavior) -> Self {
        let spawn_position = positions[0];
        let planar_offset = (look_at - spawn_position).xz().normalize_or_zero();
        let direction = Vec3::new(planar_offset.x, 0.0, planar_offset.y);
        let mut spawn_transform = Transform::from_translation(spawn_position);
        spawn_transform.look_to(-direction, Vec3::Y);
        Self {
            rotation: spawn_transform.rotation,
            ..Self::new(positions, kind, initial_behavior)
        }
    }
}

fn spawn_map(
    map: Res<Map>,
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut images: ResMut<Assets<Image>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    info!("spawning map {}", map.name);
    assert!(!map.viewpoints.is_empty(), "map should contain at least one viewpoint");
    commands.insert_resource(map.viewpoints[0]);
    commands.insert_resource(ActiveViewpointIndex(0));
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
    let mut spawn_context = SpawnContext::new(&mut commands, &asset_server, &mut meshes, &mut images, &mut materials);
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
            Transform::from_translation(enemy_spawn.positions[0]).with_rotation(enemy_spawn.rotation),
            EnemySpawnConfiguration {
                patrol_positions: enemy_spawn.positions.clone(),
                initial_behavior: enemy_spawn.initial_behavior,
            },
            true,
        );
    }
}

pub(crate) fn restart(
    map: &mut Map,
    player_position: &mut PlayerPosition,
    active_viewpoint_index: &mut ActiveViewpointIndex,
    commands: &mut Commands,
    asset_server: &AssetServer,
) {
    let current_map_name = map.name;
    *map =
        build_named_map(current_map_name).unwrap_or_else(|| panic!("cannot restart unknown map '{current_map_name}'"));
    *player_position = map.viewpoints[0];
    active_viewpoint_index.0 = 0;
    spawn_enemies(commands, asset_server, &map.enemies);
}
