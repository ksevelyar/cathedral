use avian3d::prelude::*;
use bevy::ecs::system::RunSystemOnce;
use bevy::prelude::*;
use cathedral::app::build_headless_app;
use cathedral::maps::CurrentMap;
use cathedral::player::Player;
use cathedral::shooting::shoot;
use std::time::Duration;

const MAX_HANGING_DROP: f32 = 0.5;
const MAX_LAMP_KICK_SPEED: f32 = 12.0;
const SETTLING_DEADLINE_SECONDS: f32 = 10.0;

fn create_lamp_test_app() -> App {
    let fixed_timestep = Duration::from_secs_f64(1.0 / 64.0);
    let mut app = build_headless_app(CurrentMap::Map01, None);
    app.insert_resource(Time::<Fixed>::from_duration(fixed_timestep));
    app
}

fn read_lamp_positions(world: &mut World) -> Vec<(Entity, Vec3)> {
    let bodies: Vec<Entity> = world
        .query::<&SphericalJoint>()
        .iter(world)
        .map(|joint| joint.body2)
        .filter(|body_entity| world.get::<RigidBody>(*body_entity) == Some(&RigidBody::Dynamic))
        .collect();
    bodies
        .into_iter()
        .filter_map(|body_entity| {
            world
                .get::<Position>(body_entity)
                .map(|position| (body_entity, position.0))
        })
        .collect()
}

fn wait_for_lamps(app: &mut App) -> Vec<(Entity, Vec3)> {
    for _ in 0..600 {
        app.update();
        let lamps = read_lamp_positions(app.world_mut());
        if !lamps.is_empty() {
            return lamps;
        }
    }
    panic!("map01 should spawn hanging lamps");
}

fn belongs_to_lamp(hit_entity: Entity, lamp_body: Entity, collider_parents: &Query<&ChildOf>) -> bool {
    let mut ancestor = hit_entity;
    loop {
        if ancestor == lamp_body {
            return true;
        }
        let Ok(collider_parent) = collider_parents.get(ancestor) else {
            return false;
        };
        ancestor = collider_parent.parent();
    }
}

fn aim_player_at_lamp(
    target: Res<ShotTarget>,
    mut player: Query<&mut Transform, With<Player>>,
    positions: Query<&Position>,
    collider_parents: Query<&ChildOf>,
    spatial_query: SpatialQuery,
) -> Vec3 {
    let target_position = positions.get(target.0).expect("lamp body position").0;
    let offsets = [
        Vec3::Z * 2.0,
        Vec3::X * 2.0,
        Vec3::NEG_X * 2.0,
        Vec3::Y * 2.0,
        Vec3::NEG_Y * 2.0,
        Vec3::NEG_Z * 2.0,
    ];
    for offset in offsets {
        let camera_transform =
            Transform::from_translation(target_position + offset).looking_at(target_position, Vec3::Y);
        let direction = camera_transform.forward();
        if let Some(hit) = spatial_query.cast_ray(
            camera_transform.translation,
            direction,
            100.0,
            true,
            &SpatialQueryFilter::default(),
        ) && belongs_to_lamp(hit.entity, target.0, &collider_parents)
        {
            *player.single_mut().expect("one player should spawn") = camera_transform;
            return direction.as_vec3();
        }
    }
    panic!("lamp body is not visible to a ray");
}

#[derive(Resource, Clone, Copy)]
struct ShotTarget(Entity);

fn fire_gun(app: &mut App) {
    app.world_mut()
        .resource_mut::<ButtonInput<MouseButton>>()
        .press(MouseButton::Left);
    app.world_mut().run_system_once(shoot).expect("shot system should run");
    app.update();
    app.update();
}

fn read_lamp_body_kick(world: &World, lamp_body: Entity) -> Vec3 {
    world
        .get::<LinearVelocity>(lamp_body)
        .expect("hit lamp should have velocity after the shot")
        .0
}

fn list_awake_dynamic_bodies(app: &mut App) -> Vec<(Entity, String)> {
    app.world_mut()
        .query::<(
            Entity,
            &Position,
            &RigidBody,
            &LinearVelocity,
            &AngularVelocity,
            Has<Sleeping>,
        )>()
        .iter(app.world())
        .filter(|(_, _, body, .., sleeping)| **body == RigidBody::Dynamic && !sleeping)
        .map(|(entity, position, _, velocity, _, _)| {
            (
                entity,
                format!(
                    "position={:?}, linear_speed={:.6}, angular_speed={:.6}",
                    position.0,
                    velocity.length(),
                    velocity.length()
                ),
            )
        })
        .collect()
}

#[test]
fn shot_lamp_swing_settles_to_sleep() {
    let mut app = create_lamp_test_app();
    let initial_lamps = wait_for_lamps(&mut app);
    let (lamp_body, rest_position) = *initial_lamps.first().expect("map01 should spawn hanging lamps");
    for _ in 0..256 {
        app.update();
    }
    app.insert_resource(ShotTarget(lamp_body));
    let shot_direction = app
        .world_mut()
        .run_system_once(aim_player_at_lamp)
        .expect("target lamp should be aimable");

    fire_gun(&mut app);

    let kick = read_lamp_body_kick(app.world(), lamp_body);
    assert!(
        kick.dot(shot_direction) > 0.0,
        "kick {kick:?} should push the lamp along the shot direction {shot_direction:?}"
    );
    assert!(
        kick.length() < MAX_LAMP_KICK_SPEED,
        "kick launched the lamp at {} m/s, it must be mass-capped below {MAX_LAMP_KICK_SPEED} m/s",
        kick.length()
    );

    let fixed_timestep_seconds = 1.0 / 64.0;
    let settling_steps = (SETTLING_DEADLINE_SECONDS / fixed_timestep_seconds) as usize;
    for _ in 0..settling_steps {
        app.update();
        if list_awake_dynamic_bodies(&mut app).is_empty() {
            break;
        }
    }
    let still_awake_bodies = list_awake_dynamic_bodies(&mut app);
    assert!(
        still_awake_bodies.is_empty(),
        "lamp bodies did not sleep within {SETTLING_DEADLINE_SECONDS} seconds:\n{}",
        still_awake_bodies
            .iter()
            .map(|(_, report)| report.clone())
            .collect::<Vec<_>>()
            .join("\n")
    );
    let final_position = app
        .world()
        .get::<Position>(lamp_body)
        .expect("lamp should still exist after settling")
        .0;
    assert!(
        final_position.distance(rest_position) < MAX_HANGING_DROP * 2.0,
        "lamp scattered from rest position {rest_position:?} to {final_position:?}"
    );
}

#[test]
fn lamps_stay_hanging_from_their_wires() {
    let mut app = create_lamp_test_app();
    let initial_lamps = wait_for_lamps(&mut app);
    assert!(!initial_lamps.is_empty(), "map01 should spawn hanging lamps");

    let simulated_frames = 256;
    for _ in 0..simulated_frames {
        app.update();
    }

    let final_lamps: std::collections::HashMap<Entity, Vec3> =
        read_lamp_positions(app.world_mut()).into_iter().collect();
    for (entity, initial_position) in initial_lamps {
        let final_position = final_lamps
            .get(&entity)
            .unwrap_or_else(|| panic!("lamp {entity:?} should still exist after simulation"));
        let drop = initial_position.y - final_position.y;
        assert!(
            drop < MAX_HANGING_DROP,
            "lamp {entity:?} should hang from its wire but dropped {:.2} meters from y {:.2} to y {:.2}",
            drop,
            initial_position.y,
            final_position.y
        );
    }
}
