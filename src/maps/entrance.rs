use avian3d::prelude::{
    ColliderConstructor, ColliderConstructorHierarchy, ColliderConstructorHierarchyReady, RigidBody,
};
use bevy::ecs::system::SystemParam;
use bevy::gltf::GltfAssetLabel;
use bevy::prelude::*;
use bevy::transform::TransformSystems;
use bevy::world_serialization::{WorldAssetRoot, WorldInstanceReady};

use super::{MapLoadingDependency, MapObject, SpawnContext};

#[derive(Component)]
struct UndergroundEntranceRoot {
    target_transform: Transform,
}

#[derive(Component)]
struct UndergroundEntranceSceneReady;

#[derive(Component)]
struct UndergroundEntranceCollidersReady;

#[derive(Component)]
struct UndergroundEntranceConnectorAligned;

pub(super) struct UndergroundEntrance {
    name: String,
    asset_path: &'static str,
}

impl UndergroundEntrance {
    pub(super) fn new(name: String, asset_path: &'static str) -> Self {
        Self { name, asset_path }
    }
}

impl MapObject for UndergroundEntrance {
    fn spawn(&self, context: &mut SpawnContext, transform: Transform) {
        let entrance_scene = context
            .asset_server
            .load(GltfAssetLabel::Scene(0).from_asset(self.asset_path));
        context.commands.spawn((
            Name::new(self.name.clone()),
            transform,
            WorldAssetRoot(entrance_scene),
            ColliderConstructorHierarchy::new(ColliderConstructor::TrimeshFromMesh),
            RigidBody::Static,
            MapLoadingDependency,
            UndergroundEntranceRoot {
                target_transform: transform,
            },
        ));
    }
}

pub(super) fn register_systems(app: &mut App) {
    app.add_observer(mark_underground_entrance_scene_ready)
        .add_observer(mark_underground_entrance_colliders_ready)
        .add_systems(
            PostUpdate,
            (
                align_underground_entrance_connectors,
                finish_loading_underground_entrances,
            )
                .chain()
                .after(TransformSystems::Propagate),
        );
}

fn mark_underground_entrance_scene_ready(
    world_instance_ready: On<WorldInstanceReady>,
    entrance_roots: Query<(), With<UndergroundEntranceRoot>>,
    mut commands: Commands,
) {
    if entrance_roots.get(world_instance_ready.entity).is_ok() {
        commands
            .entity(world_instance_ready.entity)
            .insert(UndergroundEntranceSceneReady);
    }
}

fn mark_underground_entrance_colliders_ready(
    collider_hierarchy_ready: On<ColliderConstructorHierarchyReady>,
    entrance_roots: Query<(), With<UndergroundEntranceRoot>>,
    mut commands: Commands,
) {
    if entrance_roots.get(collider_hierarchy_ready.entity).is_ok() {
        commands
            .entity(collider_hierarchy_ready.entity)
            .insert(UndergroundEntranceCollidersReady);
    }
}

#[derive(SystemParam)]
struct AlignUndergroundEntranceConnectorsContext<'w, 's> {
    entrance_roots: Query<
        'w,
        's,
        (Entity, &'static mut Transform, &'static UndergroundEntranceRoot),
        With<UndergroundEntranceSceneReady>,
    >,
    descendants: Query<'w, 's, &'static Children>,
    names: Query<'w, 's, &'static Name>,
    global_transforms: Query<'w, 's, &'static GlobalTransform>,
    commands: Commands<'w, 's>,
}

fn align_underground_entrance_connectors(mut context: AlignUndergroundEntranceConnectorsContext) {
    for (entrance_root, mut entrance_root_transform, entrance_root_data) in &mut context.entrance_roots {
        let entrance_root_global_transform = context
            .global_transforms
            .get(entrance_root)
            .expect("underground entrance root should have a global transform");
        let connector = find_named_descendant(
            entrance_root,
            &context.descendants,
            &context.names,
            "Entrance connector",
        );
        let connector_global_transform = context
            .global_transforms
            .get(connector)
            .expect("entrance connector should have a global transform");
        let connector_transform_relative_to_root =
            entrance_root_global_transform.affine().inverse() * connector_global_transform.affine();
        let aligned_root_affine =
            entrance_root_data.target_transform.compute_affine() * connector_transform_relative_to_root.inverse();
        *entrance_root_transform = Transform::from_matrix(Mat4::from(aligned_root_affine));
        context
            .commands
            .entity(entrance_root)
            .remove::<UndergroundEntranceSceneReady>()
            .insert(UndergroundEntranceConnectorAligned);
    }
}

fn finish_loading_underground_entrances(
    entrance_roots: Query<
        Entity,
        (
            With<UndergroundEntranceConnectorAligned>,
            With<UndergroundEntranceCollidersReady>,
        ),
    >,
    mut commands: Commands,
) {
    for entrance_root in &entrance_roots {
        commands
            .entity(entrance_root)
            .remove::<MapLoadingDependency>()
            .remove::<UndergroundEntranceRoot>()
            .remove::<UndergroundEntranceConnectorAligned>()
            .remove::<UndergroundEntranceCollidersReady>();
    }
}

fn find_named_descendant(
    root: Entity,
    descendants: &Query<&Children>,
    names: &Query<&Name>,
    expected_name: &str,
) -> Entity {
    let mut entities_to_visit = descendants
        .get(root)
        .map_or_else(|_| Vec::new(), |children| children.iter().collect::<Vec<_>>());
    while let Some(entity) = entities_to_visit.pop() {
        if names
            .get(entity)
            .is_ok_and(|entity_name| entity_name.as_str() == expected_name)
        {
            return entity;
        }
        if let Ok(children) = descendants.get(entity) {
            entities_to_visit.extend(children.iter());
        }
    }
    panic!("underground entrance should contain an {expected_name} marker");
}
