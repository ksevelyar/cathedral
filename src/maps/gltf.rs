use avian3d::prelude::*;
use bevy::gltf::GltfAssetLabel;
use bevy::prelude::*;

use super::{MapObject, SpawnContext};
use crate::ragdoll::{OBJECTS_GROUP, WORLD_GROUP};

pub struct Desk;

pub struct Laptop;

pub struct Stool;

pub struct StorageRack;

pub struct CardboardBox;

pub struct Jerrycan;

pub struct PlasticBottleGallon;

pub struct PowerBox;

impl MapObject for Desk {
    fn spawn(&self, context: &mut SpawnContext, transform: Transform) {
        spawn_dynamic_gltf_object(
            context,
            transform,
            "desk",
            "objects/table/metal_office_desk_2k.gltf",
            Collider::cuboid(2.0, 0.788, 0.947),
            Transform::from_translation(Vec3::new(0.0, 0.394, 0.0)),
            ColliderDensity(300.0),
        );
    }
}

impl MapObject for Laptop {
    fn spawn(&self, context: &mut SpawnContext, transform: Transform) {
        spawn_dynamic_gltf_object(
            context,
            transform,
            "laptop",
            "objects/classic-laptop/classic_laptop_2k.gltf",
            Collider::cuboid(0.66, 0.46, 0.49),
            Transform::from_translation(Vec3::new(0.0, 0.23, 0.0)),
            ColliderDensity(1000.0),
        );
    }
}

impl MapObject for Stool {
    fn spawn(&self, context: &mut SpawnContext, transform: Transform) {
        spawn_dynamic_gltf_object(
            context,
            transform,
            "stool",
            "objects/metal-stool/metal_stool_03_2k.gltf",
            Collider::cuboid(0.46, 1.024, 0.47),
            Transform::from_translation(Vec3::new(0.0, 0.512, 0.0)),
            ColliderDensity(500.0),
        );
    }
}

impl MapObject for StorageRack {
    fn spawn(&self, context: &mut SpawnContext, transform: Transform) {
        spawn_dynamic_gltf_object(
            context,
            transform,
            "storage rack",
            "objects/worn-metal-rack/worn_metal_rack_2k.gltf",
            Collider::cuboid(0.92, 1.9, 0.6),
            Transform::from_translation(Vec3::new(0.0, 0.95, 0.0)),
            ColliderDensity(300.0),
        );
    }
}

impl MapObject for CardboardBox {
    fn spawn(&self, context: &mut SpawnContext, transform: Transform) {
        spawn_dynamic_gltf_object(
            context,
            transform,
            "cardboard box",
            "objects/cardboard-box/cardboard_box_01_2k.gltf",
            Collider::cuboid(0.39, 0.34, 0.52),
            Transform::from_translation(Vec3::new(0.0, 0.17, 0.03)),
            ColliderDensity(100.0),
        );
    }
}

impl MapObject for Jerrycan {
    fn spawn(&self, context: &mut SpawnContext, transform: Transform) {
        spawn_dynamic_gltf_object(
            context,
            transform,
            "jerrycan",
            "objects/metal-jerrycan/metal_jerrycan_2k.gltf",
            Collider::cuboid(0.35, 0.455, 0.172),
            Transform::from_translation(Vec3::new(0.005, 0.228, -0.003)),
            ColliderDensity(500.0),
        );
    }
}

impl MapObject for PlasticBottleGallon {
    fn spawn(&self, context: &mut SpawnContext, transform: Transform) {
        spawn_dynamic_gltf_object(
            context,
            transform,
            "plastic bottle gallon",
            "objects/plastic-bottle-gallon/plastic_bottle_gallon_2k.gltf",
            Collider::cuboid(0.166, 0.275, 0.126),
            Transform::from_translation(Vec3::new(-0.005, 0.137, -0.002)),
            ColliderDensity(100.0),
        );
    }
}

impl MapObject for PowerBox {
    fn spawn(&self, context: &mut SpawnContext, transform: Transform) {
        let asset_path = "objects/power-box/power_box_01_2k.gltf";
        let scene = context
            .asset_server
            .load(GltfAssetLabel::Scene(0).from_asset(asset_path));
        let collider = Collider::cuboid(0.35, 0.51, 0.12);
        let density = ColliderDensity(500.0);
        context
            .commands
            .spawn((
                Name::new("power box"),
                transform,
                WorldAssetRoot(scene),
                RigidBody::Static,
            ))
            .with_children(|object_parent| {
                object_parent.spawn((collider, density, CollisionLayers::new(WORLD_GROUP, OBJECTS_GROUP)));
            });
    }
}

fn spawn_dynamic_gltf_object(
    context: &mut SpawnContext,
    transform: Transform,
    name: &'static str,
    asset_path: &'static str,
    collider: Collider,
    collider_transform: Transform,
    density: ColliderDensity,
) {
    let scene = context
        .asset_server
        .load(GltfAssetLabel::Scene(0).from_asset(asset_path));
    let object_linear_damping = 0.5;
    let object_angular_damping = 8.0;
    context
        .commands
        .spawn((
            Name::new(name),
            transform,
            WorldAssetRoot(scene),
            RigidBody::Dynamic,
            LinearDamping(object_linear_damping),
            AngularDamping(object_angular_damping),
        ))
        .with_children(|object_parent| {
            object_parent.spawn((
                collider,
                collider_transform,
                density,
                CollisionLayers::new(OBJECTS_GROUP, WORLD_GROUP | OBJECTS_GROUP),
            ));
        });
}
