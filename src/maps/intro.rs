use avian3d::prelude::{ColliderConstructor, ColliderConstructorHierarchy, RigidBody};
use bevy::ecs::system::SystemParam;
use bevy::prelude::*;
use bevy::world_serialization::{WorldAssetRoot, WorldInstanceReady};

use super::entrance::UndergroundEntrance;
use super::moon::Moon;
use super::{
    Map, MapLoadingDependency, MapObject, ObjectSpawn, PlayerPosition, SandField, SpawnContext, TerrainCutout,
};

struct Tower;

pub(super) fn register_systems(app: &mut App) {
    app.add_observer(mark_terrain_cutout_scene_ready).add_systems(
        PostUpdate,
        spawn_sand_field_from_terrain_cutout.after(TransformSystems::Propagate),
    );
}

#[derive(Component)]
struct SpawnSandFieldFromTerrainCutout {
    radius: f32,
}

#[derive(Component)]
struct TerrainCutoutSceneReady;

pub(super) fn build_map() -> Map {
    let tower_horizontal_position = Vec2::new(0.0, -72.0);
    let tower_base_height = SandField::calculate_height_at(tower_horizontal_position);
    let tower_base_position = Vec3::new(
        tower_horizontal_position.x,
        tower_base_height,
        tower_horizontal_position.y,
    );
    let viewpoints = vec![
        PlayerPosition {
            label: "tower",
            position: Vec3::new(70.374, 3.095, 3.826),
            look_at: Vec3::new(69.603, 3.336, 3.237),
        },
        PlayerPosition {
            label: "tower-entrance",
            position: Vec3::new(-9.987, 3.315, -50.426),
            look_at: Vec3::new(-9.708, 3.166, -51.374),
        },
        PlayerPosition {
            label: "dunes",
            position: Vec3::new(-14.755, 3.537, 37.628),
            look_at: Vec3::new(-15.352, 3.543, 38.430),
        },
        PlayerPosition {
            label: "stairwell",
            position: Vec3::new(18.937, 13.396, -73.338),
            look_at: Vec3::new(18.052, 13.757, -73.631),
        },
        PlayerPosition {
            label: "underground-entrance-1",
            position: Vec3::new(-15.223, -9.604, -69.282),
            look_at: Vec3::new(-15.970, -9.624, -68.617),
        },
    ];

    let moon_direction = Vec3::new(-0.859, 0.438, 0.265).normalize();
    let mut tower_transform = Transform::from_translation(tower_base_position);
    tower_transform.rotation = Quat::from_rotation_y(-28.0_f32.to_radians());
    let tower = Tower;
    let moon = Moon::create(moon_direction);
    Map {
        name: "intro",
        ambient: GlobalAmbientLight {
            color: Color::srgb(0.25, 0.34, 0.55),
            brightness: 15.0,
            ..default()
        },
        clear_color: ClearColor(Color::srgb(0.004, 0.009, 0.025)),
        fog_volume: None,
        viewpoints,
        enemies: Vec::new(),
        objects: vec![
            ObjectSpawn::new(tower_transform, tower),
            ObjectSpawn::new(Transform::from_translation(tower_base_position), moon),
        ],
    }
}

impl MapObject for Tower {
    fn spawn(&self, context: &mut SpawnContext, transform: Transform) {
        let tower_scene = context.asset_server.load("models/intro-tower.glb#Scene0");
        let sand_field_radius = 300.0;

        context.commands.spawn((
            WorldAssetRoot(tower_scene),
            ColliderConstructorHierarchy::new(ColliderConstructor::TrimeshFromMesh),
            RigidBody::Static,
            MapLoadingDependency,
            SpawnSandFieldFromTerrainCutout {
                radius: sand_field_radius,
            },
            Name::new("Intro tower"),
            transform,
        ));
    }
}

fn mark_terrain_cutout_scene_ready(
    world_instance_ready: On<WorldInstanceReady>,
    tower_roots: Query<(), With<SpawnSandFieldFromTerrainCutout>>,
    mut commands: Commands,
) {
    if tower_roots.get(world_instance_ready.entity).is_ok() {
        commands
            .entity(world_instance_ready.entity)
            .insert(TerrainCutoutSceneReady);
    }
}

#[derive(SystemParam)]
struct SpawnSandFieldFromTerrainCutoutContext<'w, 's> {
    commands: Commands<'w, 's>,
    tower_roots: Query<'w, 's, (Entity, &'static SpawnSandFieldFromTerrainCutout), With<TerrainCutoutSceneReady>>,
    descendants: Query<'w, 's, &'static Children>,
    names: Query<'w, 's, &'static Name>,
    global_transforms: Query<'w, 's, &'static GlobalTransform>,
    asset_server: Res<'w, AssetServer>,
    meshes: ResMut<'w, Assets<Mesh>>,
    images: ResMut<'w, Assets<Image>>,
    materials: ResMut<'w, Assets<StandardMaterial>>,
}

fn spawn_sand_field_from_terrain_cutout(mut context: SpawnSandFieldFromTerrainCutoutContext) {
    let mut spawn_context = SpawnContext::new(
        &mut context.commands,
        &context.asset_server,
        &mut context.meshes,
        &mut context.images,
        &mut context.materials,
    );
    for (tower_root, sand_field_spawn) in &context.tower_roots {
        let terrain_cutout_vertices = collect_terrain_cutout_vertices(
            tower_root,
            &context.descendants,
            &context.names,
            &context.global_transforms,
        );
        let entrance_asset_mapping = [
            ("Level -1", "models/entrance-to-underground-level.glb"),
            ("Level -2", "models/entrance-to-underground-level.glb"),
            ("Level -3", "models/entrance-to-underground-level.glb"),
            ("Level -4", "models/entrance-to-underground-level.glb"),
        ];
        for (level_marker_name, entrance_asset_path) in entrance_asset_mapping {
            let level_marker =
                find_named_tower_descendant(tower_root, &context.descendants, &context.names, level_marker_name);
            let level_marker_transform = *context
                .global_transforms
                .get(level_marker)
                .expect("tower level marker should have a global transform");
            let entrance_root_name = format!("Entrance to {level_marker_name}");
            let entrance = UndergroundEntrance::new(entrance_root_name, entrance_asset_path);
            entrance.spawn(&mut spawn_context, level_marker_transform.compute_transform());
        }

        let sand_field =
            SandField::create(sand_field_spawn.radius).with_cutout(TerrainCutout::new(terrain_cutout_vertices));
        sand_field.spawn(&mut spawn_context, Transform::IDENTITY);
        spawn_context
            .commands
            .entity(tower_root)
            .remove::<SpawnSandFieldFromTerrainCutout>()
            .remove::<MapLoadingDependency>()
            .remove::<TerrainCutoutSceneReady>();
    }
}

fn find_named_tower_descendant(
    tower_root: Entity,
    descendants: &Query<&Children>,
    names: &Query<&Name>,
    expected_name: &str,
) -> Entity {
    let mut entities_to_visit = descendants
        .get(tower_root)
        .map_or_default(|child_entities| child_entities.iter().collect::<Vec<_>>());
    while let Some(entity) = entities_to_visit.pop() {
        if let Ok(child_entities) = descendants.get(entity) {
            entities_to_visit.extend(child_entities.iter());
        }
        if names
            .get(entity)
            .is_ok_and(|entity_name| entity_name.as_str() == expected_name)
        {
            return entity;
        }
    }
    panic!("intro tower scene should contain a {expected_name} marker");
}

fn collect_terrain_cutout_vertices(
    tower_root: Entity,
    descendants: &Query<&Children>,
    names: &Query<&Name>,
    global_transforms: &Query<&GlobalTransform>,
) -> Vec<Vec2> {
    let mut entities_to_visit = descendants
        .get(tower_root)
        .map_or_default(|child_entities| child_entities.iter().collect::<Vec<_>>());
    let terrain_cutout_entity = loop {
        let Some(entity) = entities_to_visit.pop() else {
            panic!("intro tower scene should contain a Terrain cutout marker");
        };
        if let Ok(child_entities) = descendants.get(entity) {
            entities_to_visit.extend(child_entities.iter());
        }
        if names
            .get(entity)
            .is_ok_and(|entity_name| entity_name.as_str() == "Terrain cutout")
        {
            break entity;
        }
    };
    let terrain_cutout_children = descendants
        .get(terrain_cutout_entity)
        .map_or_default(|child_entities| child_entities.iter().collect::<Vec<_>>());
    let required_vertex_count = 8;
    let terrain_cutout_vertices = (0..required_vertex_count)
        .map(|vertex_index| {
            let required_vertex_name = format!("Terrain cutout vertex {vertex_index:02}");
            let vertex_entity = terrain_cutout_children
                .iter()
                .copied()
                .find(|child_entity| {
                    names
                        .get(*child_entity)
                        .is_ok_and(|entity_name| entity_name.as_str() == required_vertex_name)
                })
                .unwrap_or_else(|| {
                    panic!("intro tower scene should contain terrain cutout vertex {vertex_index:02}");
                });
            global_transforms
                .get(vertex_entity)
                .expect("terrain cutout vertex should have a global transform")
                .translation()
                .xz()
        })
        .collect::<Vec<_>>();
    assert_eq!(
        terrain_cutout_children.len(),
        required_vertex_count,
        "intro tower scene should contain exactly eight direct children under the Terrain cutout marker"
    );
    terrain_cutout_vertices
}
