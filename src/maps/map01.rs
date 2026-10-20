use bevy::prelude::*;

use super::gltf::{CardboardBox, Desk, Jerrycan, Laptop, PlasticBottleGallon, PowerBox, Stool, StorageRack};
use super::hanging_lamp::HangingLamp;
use super::structure::{Ceiling, Floor, Wall};
use super::{EnemySpawn, FogVolumeSettings, Map, ObjectSpawn, PlayerPosition};
use crate::enemies::{EnemyBehavior, EnemyKind};

pub(super) fn build_map() -> Map {
    let name = "map01";
    let indirect_light_color = Color::srgb(0.45, 0.5, 0.58);
    let indirect_light_brightness = 50.0;
    Map {
        name,
        ambient: GlobalAmbientLight {
            color: indirect_light_color,
            brightness: indirect_light_brightness,
            ..default()
        },
        clear_color: ClearColor(Color::srgb(0.05, 0.05, 0.05)),
        fog_volume: Some(FogVolumeSettings {
            center: Vec3::new(0.0, 3.0, 0.0),
            size: Vec3::new(25.0, 6.0, 19.0),
            color: Color::srgb(0.32, 0.4, 0.5),
            density: 0.002,
            absorption: 0.15,
            scattering: 0.45,
            scattering_asymmetry: 0.7,
        }),
        player_position: PlayerPosition {
            position: Vec3::new(10.0, 1.9, 7.0),
            look_at: Vec3::new(-6.0, 2.0, -2.0),
        },
        enemies: build_enemies_before_player(),
        objects: build_control_room_objects(),
    }
}

fn build_enemies_before_player() -> Vec<EnemySpawn> {
    let player_position = Vec3::new(10.0, 0.0, 7.0);
    let player_look_at = Vec3::new(-6.0, 0.0, -2.0);
    let forward_direction = (player_look_at - player_position).normalize();
    let gunner_count = 3;
    let arc_radius = 7.0;
    let arc_span_radians = std::f32::consts::FRAC_PI_3;
    let first_gunner_angle = -arc_span_radians * 0.5;
    let gunner_angle_step = arc_span_radians / (gunner_count - 1) as f32;
    let mut enemies: Vec<EnemySpawn> = (0..gunner_count)
        .map(|gunner_index| {
            let gunner_angle = first_gunner_angle + gunner_angle_step * gunner_index as f32;
            let gunner_direction = Quat::from_rotation_y(gunner_angle).mul_vec3(forward_direction);
            let gunner_position = player_position + gunner_direction * arc_radius;
            let patrol_offset = Quat::from_rotation_y(std::f32::consts::FRAC_PI_2).mul_vec3(gunner_direction) * 2.0;
            EnemySpawn::new(
                vec![gunner_position, gunner_position + patrol_offset],
                EnemyKind::Gunner,
                EnemyBehavior::Patrol,
            )
        })
        .collect();
    let fighter_positions = [Vec3::new(-3.0, 0.0, 3.0), Vec3::new(3.0, 0.0, 3.0)];
    enemies.extend(fighter_positions.into_iter().map(|position| {
        let patrol_offset = Vec3::new(0.0, 0.0, 2.0);
        EnemySpawn::new(
            vec![position, position + patrol_offset],
            EnemyKind::Fighter,
            EnemyBehavior::Patrol,
        )
    }));
    enemies
}

fn build_control_room_objects() -> Vec<ObjectSpawn> {
    build_room_structure()
        .into_iter()
        .chain(build_hanging_lamps())
        .chain(build_room_furniture())
        .collect()
}

fn build_room_structure() -> Vec<ObjectSpawn> {
    [
        ObjectSpawn::new(
            Transform::from_translation(Vec3::new(0.0, 0.05, 0.0)),
            Floor {
                size: Vec3::new(25.8, 0.1, 19.8),
            },
        ),
        ObjectSpawn::new(
            Transform::from_translation(Vec3::new(0.0, 5.95, 0.0)),
            Ceiling {
                size: Vec3::new(25.8, 0.1, 19.8),
            },
        ),
        ObjectSpawn::new(
            Transform::from_translation(Vec3::new(0.0, 3.0, 9.6)),
            Wall {
                size: Vec3::new(25.4, 6.0, 0.2),
            },
        ),
        ObjectSpawn::new(
            Transform::from_translation(Vec3::new(12.6, 3.0, 0.0)),
            Wall {
                size: Vec3::new(0.2, 6.0, 19.4),
            },
        ),
        ObjectSpawn::new(
            Transform::from_translation(Vec3::new(-12.6, 3.0, 0.0)),
            Wall {
                size: Vec3::new(0.2, 6.0, 19.4),
            },
        ),
        ObjectSpawn::new(
            Transform::from_translation(Vec3::new(-6.75, 3.0, -9.6)),
            Wall {
                size: Vec3::new(11.9, 6.0, 0.2),
            },
        ),
        ObjectSpawn::new(
            Transform::from_translation(Vec3::new(6.75, 3.0, -9.6)),
            Wall {
                size: Vec3::new(11.9, 6.0, 0.2),
            },
        ),
        ObjectSpawn::new(
            Transform::from_translation(Vec3::new(0.0, 4.1, -9.6)),
            Wall {
                size: Vec3::new(1.6, 3.8, 0.2),
            },
        ),
    ]
    .into_iter()
    .collect()
}

fn build_hanging_lamps() -> [ObjectSpawn; 4] {
    let lamp_pivot_height = 5.85;
    let lamp_wire_length = 2.2;
    [(-6.0, -4.5), (6.0, -4.5), (-6.0, 4.5), (6.0, 4.5)].map(|(x_position, z_position)| {
        ObjectSpawn::new(
            Transform::from_translation(Vec3::new(x_position, lamp_pivot_height, z_position)),
            HangingLamp {
                wire_length: lamp_wire_length,
            },
        )
    })
}

fn build_room_furniture() -> [ObjectSpawn; 11] {
    let floor_surface_height = 0.1;
    let desk_height = 0.788;
    let object_separation = 0.01;
    let desk_z_position = -1.0;
    let stool_z_position = 0.1;
    let laptop_height_above_desk = desk_height + object_separation;
    let half_turn_rotation = Quat::from_rotation_y(std::f32::consts::PI);
    let storage_rack_rotation = Quat::from_rotation_y(std::f32::consts::FRAC_PI_2);
    [
        ObjectSpawn::new(
            Transform::from_translation(Vec3::new(-4.0, floor_surface_height, desk_z_position)),
            Desk,
        ),
        ObjectSpawn::new(
            Transform::from_translation(Vec3::new(4.0, floor_surface_height, desk_z_position)),
            Desk,
        ),
        ObjectSpawn::new(
            Transform::from_translation(Vec3::new(
                -4.0,
                floor_surface_height + laptop_height_above_desk,
                desk_z_position,
            )),
            Laptop,
        ),
        ObjectSpawn::new(
            Transform::from_translation(Vec3::new(-4.0, floor_surface_height, stool_z_position))
                .with_rotation(half_turn_rotation),
            Stool,
        ),
        ObjectSpawn::new(
            Transform::from_translation(Vec3::new(4.0, floor_surface_height, stool_z_position))
                .with_rotation(half_turn_rotation),
            Stool,
        ),
        ObjectSpawn::new(
            Transform::from_translation(Vec3::new(-12.1, floor_surface_height, 6.5))
                .with_rotation(storage_rack_rotation),
            StorageRack,
        ),
        ObjectSpawn::new(
            Transform::from_translation(Vec3::new(-10.8, floor_surface_height, 7.8)),
            CardboardBox,
        ),
        ObjectSpawn::new(
            Transform::from_translation(Vec3::new(-10.35, floor_surface_height, 7.85)),
            Jerrycan,
        ),
        ObjectSpawn::new(
            Transform::from_translation(Vec3::new(-11.15, floor_surface_height, 7.85)),
            PlasticBottleGallon,
        ),
        ObjectSpawn::new(Transform::from_translation(Vec3::new(-10.0, 2.0, 9.43)), PowerBox),
        ObjectSpawn::new(Transform::from_translation(Vec3::new(10.0, 2.0, 9.43)), PowerBox),
    ]
}
