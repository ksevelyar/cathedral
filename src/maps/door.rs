use avian3d::prelude::{
    ColliderConstructor, ColliderConstructorHierarchy, ColliderConstructorHierarchyReady, RigidBody,
};
use bevy::ecs::system::SystemParam;
use bevy::gltf::GltfAssetLabel;
use bevy::prelude::*;
use bevy::transform::TransformSystems;
use bevy::world_serialization::{WorldAssetRoot, WorldInstanceReady};

use super::MapLoadingDependency;
use crate::player::{self, Player};
use crate::state::GameState;

const DOOR_TRIGGER_DISTANCE: f32 = 7.0;
const DOOR_ANIMATION_DURATION: f32 = 0.8;

#[derive(Component)]
struct DoorRoot {
    target_transform: Transform,
}

#[derive(Component)]
struct DoorMountPending;

#[derive(Component)]
struct DoorSceneReady;

#[derive(Component)]
struct DoorCollidersReady;

#[derive(Component)]
struct DoorConnectorAligned;

#[derive(Component)]
struct DoorController {
    connector: Entity,
    left_panel: Entity,
    right_panel: Entity,
    left_closed_transform: Transform,
    left_open_transform: Transform,
    right_closed_transform: Transform,
    right_open_transform: Transform,
    trigger_distance: f32,
    animation_progress: f32,
}

pub(super) fn register_systems(app: &mut App) {
    app.add_observer(collect_door_mounts)
        .add_observer(mark_door_scene_ready)
        .add_observer(mark_door_colliders_ready)
        .add_systems(
            PostUpdate,
            (spawn_doors_for_mounts, align_door_connectors, finish_door_setup)
                .chain()
                .after(TransformSystems::Propagate),
        )
        .add_systems(
            Update,
            update_door_panels
                .after(player::move_player)
                .run_if(in_state(GameState::Playing)),
        );
}

#[derive(SystemParam)]
struct CollectDoorMountsContext<'w, 's> {
    door_roots: Query<'w, 's, (), With<DoorRoot>>,
    descendants: Query<'w, 's, &'static Children>,
    names: Query<'w, 's, &'static Name>,
    commands: Commands<'w, 's>,
}

fn collect_door_mounts(world_instance_ready: On<WorldInstanceReady>, mut context: CollectDoorMountsContext) {
    let scene_root = world_instance_ready.entity;
    if context.door_roots.get(scene_root).is_ok() {
        return;
    }

    let mut entities_to_visit = context
        .descendants
        .get(scene_root)
        .map_or_else(|_| Vec::new(), |children| children.iter().collect::<Vec<_>>());
    while let Some(entity) = entities_to_visit.pop() {
        if context.door_roots.get(entity).is_ok() {
            continue;
        }
        if context
            .names
            .get(entity)
            .is_ok_and(|entity_name| entity_name.as_str() == "Door mount")
        {
            context.commands.entity(entity).insert(DoorMountPending);
        }
        if let Ok(children) = context.descendants.get(entity) {
            entities_to_visit.extend(children.iter());
        }
    }
}

#[derive(SystemParam)]
struct SpawnDoorsForMountsContext<'w, 's> {
    door_mounts: Query<'w, 's, (Entity, &'static GlobalTransform), With<DoorMountPending>>,
    asset_server: Res<'w, AssetServer>,
    commands: Commands<'w, 's>,
}

fn spawn_doors_for_mounts(mut context: SpawnDoorsForMountsContext) {
    for (door_mount, door_mount_global_transform) in &context.door_mounts {
        let target_transform = door_mount_global_transform.compute_transform();
        let door_scene = context
            .asset_server
            .load(GltfAssetLabel::Scene(0).from_asset("models/door.glb"));
        let door_colliders = ColliderConstructorHierarchy::new(ColliderConstructor::TrimeshFromMesh)
            .with_constructor_for_name("Door panel left", ColliderConstructor::ConvexHullFromMesh)
            .with_constructor_for_name("Door panel right", ColliderConstructor::ConvexHullFromMesh);

        context.commands.entity(door_mount).remove::<DoorMountPending>();
        context.commands.spawn((
            Name::new("Door"),
            target_transform,
            WorldAssetRoot(door_scene),
            door_colliders,
            RigidBody::Static,
            MapLoadingDependency,
            DoorRoot { target_transform },
        ));
    }
}

fn mark_door_scene_ready(
    world_instance_ready: On<WorldInstanceReady>,
    door_roots: Query<(), With<DoorRoot>>,
    mut commands: Commands,
) {
    if door_roots.get(world_instance_ready.entity).is_ok() {
        commands.entity(world_instance_ready.entity).insert(DoorSceneReady);
    }
}

fn mark_door_colliders_ready(
    collider_hierarchy_ready: On<ColliderConstructorHierarchyReady>,
    door_roots: Query<(), With<DoorRoot>>,
    mut commands: Commands,
) {
    if door_roots.get(collider_hierarchy_ready.entity).is_ok() {
        commands
            .entity(collider_hierarchy_ready.entity)
            .insert(DoorCollidersReady);
    }
}

#[derive(SystemParam)]
struct AlignDoorConnectorsContext<'w, 's> {
    door_roots: Query<'w, 's, (Entity, &'static mut Transform, &'static DoorRoot), With<DoorSceneReady>>,
    descendants: Query<'w, 's, &'static Children>,
    names: Query<'w, 's, &'static Name>,
    global_transforms: Query<'w, 's, &'static GlobalTransform>,
    commands: Commands<'w, 's>,
}

fn align_door_connectors(mut context: AlignDoorConnectorsContext) {
    for (door_root, mut door_root_transform, door_root_data) in &mut context.door_roots {
        let door_root_global_transform = context
            .global_transforms
            .get(door_root)
            .expect("door root should have a global transform");
        let door_connector = find_named_descendant(door_root, &context.descendants, &context.names, "Door connector");
        let door_connector_global_transform = context
            .global_transforms
            .get(door_connector)
            .expect("door connector should have a global transform");
        let door_connector_transform_relative_to_root =
            door_root_global_transform.affine().inverse() * door_connector_global_transform.affine();
        let aligned_root_affine =
            door_root_data.target_transform.compute_affine() * door_connector_transform_relative_to_root.inverse();
        *door_root_transform = Transform::from_matrix(Mat4::from(aligned_root_affine));
        context
            .commands
            .entity(door_root)
            .remove::<DoorSceneReady>()
            .insert(DoorConnectorAligned);
    }
}

#[derive(SystemParam)]
struct FinishDoorSetupContext<'w, 's> {
    door_roots: Query<'w, 's, Entity, (With<DoorConnectorAligned>, With<DoorCollidersReady>)>,
    descendants: Query<'w, 's, &'static Children>,
    names: Query<'w, 's, &'static Name>,
    transforms: Query<'w, 's, &'static Transform>,
    commands: Commands<'w, 's>,
}

fn finish_door_setup(mut context: FinishDoorSetupContext) {
    for door_root in &context.door_roots {
        let door_connector = find_named_descendant(door_root, &context.descendants, &context.names, "Door connector");
        let left_panel = find_named_descendant(door_root, &context.descendants, &context.names, "Door panel left");
        let right_panel = find_named_descendant(door_root, &context.descendants, &context.names, "Door panel right");
        let left_open_marker =
            find_named_descendant(door_root, &context.descendants, &context.names, "Door panel left open");
        let right_open_marker =
            find_named_descendant(door_root, &context.descendants, &context.names, "Door panel right open");
        let left_closed_transform = *context
            .transforms
            .get(left_panel)
            .expect("left door panel should have a local transform");
        let right_closed_transform = *context
            .transforms
            .get(right_panel)
            .expect("right door panel should have a local transform");
        let left_open_transform = *context
            .transforms
            .get(left_open_marker)
            .expect("left door open marker should have a local transform");
        let right_open_transform = *context
            .transforms
            .get(right_open_marker)
            .expect("right door open marker should have a local transform");

        context.commands.entity(left_panel).insert(RigidBody::Kinematic);
        context.commands.entity(right_panel).insert(RigidBody::Kinematic);
        context.commands.entity(door_root).insert(DoorController {
            connector: door_connector,
            left_panel,
            right_panel,
            left_closed_transform,
            left_open_transform,
            right_closed_transform,
            right_open_transform,
            trigger_distance: DOOR_TRIGGER_DISTANCE,
            animation_progress: 0.0,
        });
        context
            .commands
            .entity(door_root)
            .remove::<MapLoadingDependency>()
            .remove::<DoorRoot>()
            .remove::<DoorConnectorAligned>()
            .remove::<DoorCollidersReady>();
    }
}

fn update_door_panels(
    time: Res<Time>,
    player_global_transforms: Query<&GlobalTransform, With<Player>>,
    connector_global_transforms: Query<&GlobalTransform>,
    mut door_controllers: Query<&mut DoorController>,
    mut door_panel_transforms: Query<&mut Transform>,
) {
    let player_global_position = player_global_transforms
        .single()
        .expect("one player should exist while the game is playing")
        .translation();
    let animation_progress_delta = time.delta_secs() / DOOR_ANIMATION_DURATION;

    for mut door_controller in &mut door_controllers {
        let connector_global_position = connector_global_transforms
            .get(door_controller.connector)
            .expect("door connector should have a global transform")
            .translation();
        let player_distance_from_door = player_global_position.distance(connector_global_position);
        let target_animation_progress = if player_distance_from_door < door_controller.trigger_distance {
            1.0
        } else {
            0.0
        };
        door_controller.animation_progress = if door_controller.animation_progress < target_animation_progress {
            (door_controller.animation_progress + animation_progress_delta).min(target_animation_progress)
        } else {
            (door_controller.animation_progress - animation_progress_delta).max(target_animation_progress)
        };
        let eased_animation_progress =
            door_controller.animation_progress.powi(2) * (3.0 - 2.0 * door_controller.animation_progress);

        let [mut left_panel_transform, mut right_panel_transform] = door_panel_transforms
            .get_many_mut([door_controller.left_panel, door_controller.right_panel])
            .expect("door controller should reference two distinct panels");
        *left_panel_transform = interpolate_transform(
            door_controller.left_closed_transform,
            door_controller.left_open_transform,
            eased_animation_progress,
        );
        *right_panel_transform = interpolate_transform(
            door_controller.right_closed_transform,
            door_controller.right_open_transform,
            eased_animation_progress,
        );
    }
}

fn interpolate_transform(
    start_transform: Transform,
    end_transform: Transform,
    interpolation_progress: f32,
) -> Transform {
    Transform {
        translation: start_transform
            .translation
            .lerp(end_transform.translation, interpolation_progress),
        rotation: start_transform
            .rotation
            .slerp(end_transform.rotation, interpolation_progress),
        scale: start_transform.scale.lerp(end_transform.scale, interpolation_progress),
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
    panic!("door scene should contain a {expected_name} node");
}
