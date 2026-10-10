use bevy::prelude::*;

use super::pieces::{Piece, build_cuboid, build_hanging_lamp, make_concrete_material};
use super::{EnemySpawn, FogVolumeSettings, Map, PlayerPosition};
use crate::enemies::{EnemyKind, Fighter};

pub(super) fn build_map() -> Map {
    let indirect_light_color = Color::srgb(0.45, 0.5, 0.58);
    let indirect_light_brightness = 50.0;
    Map {
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
        enemies: spawn_fighters_in_arc_before_player(),
        pieces: spawn_control_room_pieces(),
    }
}

fn spawn_fighters_in_arc_before_player() -> Vec<EnemySpawn> {
    let player_position = Vec3::new(10.0, 0.0, 7.0);
    let player_look_at = Vec3::new(-6.0, 0.0, -2.0);
    let forward_direction = (player_look_at - player_position).normalize();
    let fighter_count = 10;
    let arc_radius = 7.0;
    let arc_span_radians = std::f32::consts::FRAC_PI_3;
    let first_fighter_angle = -arc_span_radians * 0.5;
    let fighter_angle_step = arc_span_radians / (fighter_count - 1) as f32;

    (0..fighter_count)
        .map(|fighter_index| {
            let fighter_angle = first_fighter_angle + fighter_angle_step * fighter_index as f32;
            let fighter_direction = Quat::from_rotation_y(fighter_angle).mul_vec3(forward_direction);
            EnemySpawn {
                position: player_position + fighter_direction * arc_radius,
                kind: EnemyKind::Fighter(Fighter::default()),
            }
        })
        .collect()
}

fn spawn_control_room_pieces() -> Vec<Piece> {
    let mut pieces = vec![build_floor(), build_ceiling()];
    pieces.extend(build_walls());
    pieces.extend(spawn_control_room_lights());
    pieces
}

fn build_floor() -> Piece {
    build_cuboid(
        Vec3::new(0.0, 0.05, 0.0),
        Vec3::new(25.8, 0.1, 19.8),
        make_concrete_material(2.0),
    )
}

fn build_ceiling() -> Piece {
    build_cuboid(
        Vec3::new(0.0, 5.95, 0.0),
        Vec3::new(25.8, 0.1, 19.8),
        make_concrete_material(3.0),
    )
}

fn build_walls() -> Vec<Piece> {
    vec![
        build_cuboid(
            Vec3::new(0.0, 3.0, 9.6),
            Vec3::new(25.4, 6.0, 0.2),
            make_concrete_material(3.0),
        ),
        build_cuboid(
            Vec3::new(12.6, 3.0, 0.0),
            Vec3::new(0.2, 6.0, 19.4),
            make_concrete_material(3.0),
        ),
        build_cuboid(
            Vec3::new(-12.6, 3.0, 0.0),
            Vec3::new(0.2, 6.0, 19.4),
            make_concrete_material(3.0),
        ),
        build_cuboid(
            Vec3::new(-6.75, 3.0, -9.6),
            Vec3::new(11.9, 6.0, 0.2),
            make_concrete_material(3.0),
        ),
        build_cuboid(
            Vec3::new(6.75, 3.0, -9.6),
            Vec3::new(11.9, 6.0, 0.2),
            make_concrete_material(3.0),
        ),
        build_cuboid(
            Vec3::new(0.0, 4.1, -9.6),
            Vec3::new(1.6, 3.8, 0.2),
            make_concrete_material(3.0),
        ),
    ]
}

fn spawn_control_room_lights() -> Vec<Piece> {
    const LAMP_PIVOT_HEIGHT: f32 = 5.85;
    const LAMP_WIRE_LENGTH: f32 = 2.2;
    [(-6.0, -4.5), (6.0, -4.5), (-6.0, 4.5), (6.0, 4.5)]
        .into_iter()
        .map(|(x_position, z_position)| {
            build_hanging_lamp(Vec3::new(x_position, LAMP_PIVOT_HEIGHT, z_position), LAMP_WIRE_LENGTH)
        })
        .collect()
}
