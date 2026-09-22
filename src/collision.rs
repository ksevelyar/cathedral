use avian3d::prelude::*;
use bevy::prelude::*;
use core::time::Duration;

const GROUND_SNAP_DISTANCE: f32 = 4.0;
pub(crate) const MAX_FALL_SPEED: f32 = 30.0;

pub(crate) fn slide_translation(
    move_and_slide: &MoveAndSlide,
    from: Vec3,
    radius: f32,
    center_height_offset: f32,
    filter: &SpatialQueryFilter,
    desired: Vec3,
    delta_secs: f32,
) -> Vec3 {
    let velocity = if delta_secs > 0.0 {
        desired / delta_secs
    } else {
        Vec3::ZERO
    };
    let shape_position = from + Vec3::Y * center_height_offset;
    let output = move_and_slide.move_and_slide(
        &Collider::sphere(radius),
        shape_position,
        Quat::IDENTITY,
        velocity,
        Duration::from_secs_f32(delta_secs),
        &MoveAndSlideConfig::default(),
        filter,
        |_| MoveAndSlideHitResponse::Accept,
    );
    output.position - shape_position
}

pub(crate) fn snapped_ground_height(
    spatial_query: &SpatialQuery,
    center: Vec3,
    filter: &SpatialQueryFilter,
) -> Option<f32> {
    spatial_query
        .cast_ray(center, Dir3::NEG_Y, GROUND_SNAP_DISTANCE, true, filter)
        .map(|ray_hit| center.y - ray_hit.distance)
}

pub(crate) fn integrate_fall(
    velocity: &mut LinearVelocity,
    height: &mut f32,
    ground_height: Option<f32>,
    rest_offset: f32,
    delta_secs: f32,
    gravity: f32,
) {
    velocity.y = (velocity.y + gravity * delta_secs).max(-MAX_FALL_SPEED);
    let fallen_height = *height + velocity.y * delta_secs;
    match ground_height {
        Some(ground_height) => {
            let rest_height = ground_height + rest_offset;
            if fallen_height <= rest_height {
                *height = rest_height;
                velocity.y = 0.0;
            } else {
                *height = fallen_height;
            }
        }
        None => *height = fallen_height,
    }
}
