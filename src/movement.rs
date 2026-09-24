use avian3d::prelude::*;
use bevy::prelude::*;
use core::time::Duration;

const GROUND_SNAP_DISTANCE: f32 = 4.0;
const MAX_FALL_SPEED: f32 = 30.0;

pub(crate) struct PhysicsWorld<'w, 's, 'a> {
    move_and_slide: &'a MoveAndSlide<'w, 's>,
    spatial_query: &'a SpatialQuery<'w, 's>,
    filter: &'a SpatialQueryFilter,
    radius: f32,
    center_height_offset: f32,
    rest_offset: f32,
    gravity_acceleration: f32,
    delta_secs: f32,
}

impl<'w, 's, 'a> PhysicsWorld<'w, 's, 'a> {
    pub(crate) fn new(
        move_and_slide: &'a MoveAndSlide<'w, 's>,
        gravity_acceleration: f32,
        delta_secs: f32,
        filter: &'a SpatialQueryFilter,
        radius: f32,
        center_height_offset: f32,
        rest_offset: f32,
    ) -> Self {
        Self {
            move_and_slide,
            spatial_query: &move_and_slide.spatial_query,
            filter,
            radius,
            center_height_offset,
            rest_offset,
            gravity_acceleration,
            delta_secs,
        }
    }

    pub(crate) fn get_delta_secs(&self) -> f32 {
        self.delta_secs
    }

    pub(crate) fn steer_direction(&self, from: Vec3, desired: Vec3) -> Vec3 {
        let probe_distance = 2.0;
        let avoidance_angles_degrees: [f32; 5] = [0.0, 45.0, -45.0, 90.0, -90.0];
        let distance = desired.length();
        if distance == 0.0 {
            return Vec3::ZERO;
        }
        let direction = desired / distance;
        for angle_degrees in avoidance_angles_degrees {
            let candidate = Quat::from_rotation_y(angle_degrees.to_radians()) * direction;
            let Ok(candidate_direction) = Dir3::new(candidate) else {
                continue;
            };
            if self
                .spatial_query
                .cast_shape(
                    &Collider::sphere(self.radius),
                    from + Vec3::Y * self.center_height_offset,
                    Quat::IDENTITY,
                    candidate_direction,
                    &ShapeCastConfig::from_max_distance(probe_distance),
                    self.filter,
                )
                .is_none()
            {
                return candidate_direction * distance;
            }
        }
        Vec3::ZERO
    }

    pub(crate) fn slide(&self, from: Vec3, desired: Vec3) -> Vec3 {
        let velocity = if self.delta_secs > 0.0 {
            desired / self.delta_secs
        } else {
            Vec3::ZERO
        };
        let shape_position = from + Vec3::Y * self.center_height_offset;
        let output = self.move_and_slide.move_and_slide(
            &Collider::sphere(self.radius),
            shape_position,
            Quat::IDENTITY,
            velocity,
            Duration::from_secs_f32(self.delta_secs),
            &MoveAndSlideConfig::default(),
            self.filter,
            |_| MoveAndSlideHitResponse::Accept,
        );
        output.position - shape_position
    }

    pub(crate) fn find_ground_height(&self, anchor: Vec3) -> Option<f32> {
        let center = anchor + Vec3::Y * self.center_height_offset;
        self.spatial_query
            .cast_ray(center, Dir3::NEG_Y, GROUND_SNAP_DISTANCE, true, self.filter)
            .map(|ray_hit| center.y - ray_hit.distance)
    }

    pub(crate) fn integrate_fall(&self, velocity: &mut LinearVelocity, translation: &mut Vec3) {
        let ground_height = self.find_ground_height(*translation);
        velocity.y = (velocity.y + self.gravity_acceleration * self.delta_secs).max(-MAX_FALL_SPEED);
        let fallen_height = translation.y + velocity.y * self.delta_secs;
        match ground_height {
            Some(ground_height) => {
                let rest_height = ground_height + self.rest_offset;
                if fallen_height <= rest_height {
                    translation.y = rest_height;
                    velocity.y = 0.0;
                } else {
                    translation.y = fallen_height;
                }
            }
            None => translation.y = fallen_height,
        }
    }
}
