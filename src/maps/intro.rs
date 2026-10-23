use avian3d::prelude::{ColliderConstructor, ColliderConstructorHierarchy, RigidBody};
use bevy::prelude::*;

use super::moon::Moon;
use super::{Map, MapObject, ObjectSpawn, PlayerPosition, SandField, SandFieldOpening, SpawnContext};

struct Tower;

const TOWER_HORIZONTAL_POSITION: Vec2 = Vec2::new(0.0, -72.0);
const TOWER_TERRAIN_OPENING_HALF_WIDTH: f32 = 17.5;

pub(super) fn build_map() -> Map {
    let tower_horizontal_position = TOWER_HORIZONTAL_POSITION;
    let tower_base_height = SandField::calculate_height_at(tower_horizontal_position);
    let tower_base_position = Vec3::new(
        tower_horizontal_position.x,
        tower_base_height,
        tower_horizontal_position.y,
    );
    let viewpoints = vec![
        PlayerPosition {
            label: "tower",
            position: Vec3::new(8.857, 3.108, 17.065),
            look_at: Vec3::new(8.680, 3.366, 16.115),
        },
        PlayerPosition {
            label: "dunes",
            position: Vec3::new(-14.755, 3.537, 37.628),
            look_at: Vec3::new(-15.352, 3.543, 38.430),
        },
        PlayerPosition {
            label: "stairwell",
            position: Vec3::new(14.462, 1.896, -55.609),
            look_at: Vec3::new(14.076, 2.488, -56.317),
        },
    ];

    let moon_direction = Vec3::new(-0.7, 0.42, -0.7).normalize();
    let tower_transform = Transform::from_translation(tower_base_position);
    let sand_field_radius = 300.0;
    let tower_terrain_opening =
        SandFieldOpening::create(TOWER_HORIZONTAL_POSITION, Vec2::splat(TOWER_TERRAIN_OPENING_HALF_WIDTH));
    let sand_field = SandField::create(sand_field_radius).with_opening(tower_terrain_opening);
    let tower = Tower;
    let moon = Moon::create(moon_direction);
    Map {
        name: "intro",
        ambient: GlobalAmbientLight {
            color: Color::srgb(0.25, 0.34, 0.55),
            brightness: 35.0,
            ..default()
        },
        clear_color: ClearColor(Color::srgb(0.004, 0.009, 0.025)),
        fog_volume: None,
        viewpoints,
        enemies: Vec::new(),
        objects: vec![
            ObjectSpawn::new(Transform::IDENTITY, sand_field),
            ObjectSpawn::new(tower_transform, tower),
            ObjectSpawn::new(Transform::IDENTITY, moon),
        ],
    }
}

impl MapObject for Tower {
    fn spawn(&self, context: &mut SpawnContext, transform: Transform) {
        let tower_scene = context.asset_server.load("models/intro-tower.glb#Scene0");

        context.commands.spawn((
            WorldAssetRoot(tower_scene),
            ColliderConstructorHierarchy::new(ColliderConstructor::TrimeshFromMesh),
            RigidBody::Static,
            Name::new("Intro tower"),
            transform,
        ));
    }
}
