use super::{
    BoneMap, HullColliders, LIMBS, MeasuredLimb, PendingRagdollImpact, PendingRagdollImpactRequest, RagdollBodyPart,
    RigPart, SpawnedRagdollColliders, assign_drivers, measure_limb, measure_limbs, spawn_joints,
    spawn_ragdoll_colliders,
};
use avian3d::math::AsF32;
use avian3d::prelude::*;
use bevy::prelude::*;
use std::collections::HashMap;

#[derive(Component)]
pub(crate) struct RagdollData {
    bones: BoneMap,
    limbs: HashMap<RagdollBodyPart, MeasuredLimb>,
    parts: Vec<RigPart>,
}

impl RagdollData {
    pub(crate) fn has_all_limbs(&self) -> bool {
        self.limbs.len() == LIMBS.len()
    }
}

pub(crate) struct RagdollAssembleContext<'w, 's> {
    pub(crate) hull_colliders: &'w HullColliders,
    pub(crate) transforms: &'w Query<'w, 's, &'static GlobalTransform>,
    pub(crate) parents: &'w Query<'w, 's, &'static ChildOf>,
    pub(crate) is_disabled: bool,
    pub(crate) show_collider_gizmos: bool,
}

pub(crate) fn assemble_from_bones_and_colliders(
    commands: &mut Commands,
    character_entity: Entity,
    bones: &BoneMap,
    context: RagdollAssembleContext<'_, '_>,
) -> Option<RagdollData> {
    let RagdollAssembleContext {
        hull_colliders,
        transforms,
        parents,
        is_disabled,
        show_collider_gizmos,
    } = context;
    let measured_limbs = measure_limbs(character_entity, bones, transforms)?;
    let spawned_colliders = spawn_ragdoll_colliders(
        commands,
        character_entity,
        hull_colliders,
        &measured_limbs,
        !is_disabled,
        show_collider_gizmos,
    );

    let SpawnedRagdollColliders {
        part_entities,
        mut parts,
    } = spawned_colliders;
    let pelvis_entity = part_entities[&RagdollBodyPart::Pelvis];
    let mut bones_by_part = assign_drivers(bones, part_entities, parents, transforms, pelvis_entity);
    for part in &mut parts {
        part.bones = bones_by_part.remove(&part.entity).unwrap_or_default();
    }

    Some(RagdollData {
        bones: bones.clone(),
        limbs: measured_limbs,
        parts,
    })
}

pub(crate) fn synchronize_from_bones(
    ragdoll: &mut RagdollData,
    bone_transforms: &Query<&GlobalTransform>,
    body_transforms: &mut Query<(&mut Position, &mut Rotation, &mut Transform), With<RagdollBodyPart>>,
) {
    let mut measured_limbs: HashMap<RagdollBodyPart, MeasuredLimb> = HashMap::new();
    let mut limb_transforms: HashMap<RagdollBodyPart, Transform> = HashMap::new();
    for specification in LIMBS {
        let Some((transform, measured_limb)) = measure_limb(specification, &ragdoll.bones, bone_transforms) else {
            continue;
        };
        limb_transforms.insert(specification.part, transform);
        measured_limbs.insert(specification.part, measured_limb);
    }

    for part in &ragdoll.parts {
        let Some(body_transform) = limb_transforms.get(&part.body_part) else {
            continue;
        };
        let Ok((mut position, mut rotation, mut transform)) = body_transforms.get_mut(part.entity) else {
            continue;
        };
        position.0 = body_transform.translation;
        rotation.0 = body_transform.rotation;
        *transform = *body_transform;
    }

    if limb_transforms.len() == LIMBS.len() {
        ragdoll.limbs = measured_limbs;
    }
}

pub(crate) fn activate_with_impact(
    commands: &mut Commands,
    ragdoll: &RagdollData,
    impact: &PendingRagdollImpactRequest,
    character_velocity: LinearVelocity,
    character_entity: Entity,
) {
    let part_entities: HashMap<RagdollBodyPart, Entity> =
        ragdoll.parts.iter().map(|part| (part.body_part, part.entity)).collect();
    for part in &ragdoll.parts {
        commands.entity(part.entity).insert(character_velocity);
        commands.entity(part.entity).remove::<RigidBodyDisabled>();
    }
    spawn_joints(commands, character_entity, &part_entities, &ragdoll.limbs);
    let impact_body_entity = part_entities[&impact.body_part];
    commands.entity(impact_body_entity).insert(PendingRagdollImpact {
        impulse: impact.impulse,
        point: impact.point,
    });
}

pub(crate) fn apply_pose_to_bones(
    ragdoll: &RagdollData,
    body_transforms: &Query<(&Position, &Rotation)>,
    bone_transforms: &mut Query<&mut GlobalTransform>,
) {
    for part in &ragdoll.parts {
        let Ok((position, rotation)) = body_transforms.get(part.entity) else {
            continue;
        };
        let current_transform =
            GlobalTransform::from(Transform::from_translation(position.f32()).with_rotation(rotation.f32()));
        let transform_delta = current_transform.affine() * part.initial.affine().inverse();

        for &(bone_entity, bone_initial_transform) in &part.bones {
            if let Ok(mut bone_transform) = bone_transforms.get_mut(bone_entity) {
                *bone_transform = GlobalTransform::from(Mat4::from(transform_delta * bone_initial_transform.affine()));
            }
        }
    }
}

pub(crate) fn find_head_position(
    ragdoll: &RagdollData,
    body_positions: &Query<&Position, With<RagdollBodyPart>>,
) -> Option<Vec3> {
    let head_entity = ragdoll
        .parts
        .iter()
        .find(|part| part.body_part == RagdollBodyPart::Head)
        .map(|part| part.entity)?;
    body_positions.get(head_entity).ok().map(|position| position.0)
}
