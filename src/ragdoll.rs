use crate::enemies::{Dying, EnemyHit};
use crate::player::Player;
use avian3d::prelude::*;
use bevy::ecs::system::SystemParam;
use bevy::mesh::skinning::{SkinnedMesh, SkinnedMeshInverseBindposes};
use bevy::mesh::{Mesh, Mesh3d, VertexAttributeValues};
use bevy::prelude::*;
use bevy::transform::TransformSystems;
use bevy::world_serialization::WorldInstanceReady;
use std::collections::{HashMap, HashSet};

mod rig;

pub(crate) use rig::{RagdollData, find_head_position};

pub(crate) const RAGDOLL_GROUP: u32 = 0b10;
pub(crate) const OBJECTS_GROUP: u32 = 0b100;
pub(crate) const WORLD_GROUP: u32 = 0b01;

pub struct RagdollPlugin;

#[derive(Default, Reflect, GizmoConfigGroup)]
pub(crate) struct RagdollBoneGizmos;

impl Plugin for RagdollPlugin {
    fn build(&self, app: &mut App) {
        app.add_observer(wake_ragdoll_bodies_on_hit)
            .add_systems(
                PostUpdate,
                (
                    spawn_ragdoll_bodies,
                    synchronize_ragdoll_bodies,
                    activate_ragdoll_on_death,
                    apply_pending_impacts,
                    apply_ragdoll_pose,
                )
                    .chain()
                    .after(TransformSystems::Propagate),
            )
            .add_systems(
                PostUpdate,
                draw_measured_ragdoll_bones.after(TransformSystems::Propagate),
            );
    }
}

const ANGULAR_DAMPING: f32 = 15.0;
const LINEAR_DAMPING: f32 = 1.0;
const JOINT_ANGULAR_DAMPING: f32 = 15.0;
const WRIST_ANGULAR_DAMPING: f32 = 64.0;
const NECK_ANGULAR_DAMPING: f32 = 64.0;
const HIP_ANGULAR_DAMPING: f32 = 15.0;
const ANKLE_ANGULAR_DAMPING: f32 = 64.0;
const RAGDOLL_ANGULAR_SLEEP_THRESHOLD: f32 = 0.6;

const COLLIDER_DENSITY: f32 = 1000.0;

#[derive(Component, Clone, Default)]
pub(crate) struct BoneMap(HashMap<Box<str>, Entity>);

impl BoneMap {
    pub(crate) fn get(&self, name: &str) -> Option<Entity> {
        self.0.get(name).copied()
    }

    pub(crate) fn list_ragdoll_entities(&self) -> impl Iterator<Item = Entity> + '_ {
        self.0.values().copied()
    }
}

#[derive(Component)]
struct PendingRagdoll;

pub(crate) struct RigPart {
    pub(crate) entity: Entity,
    pub(crate) body_part: RagdollBodyPart,
    pub(crate) initial: GlobalTransform,
    pub(crate) bones: Vec<(Entity, GlobalTransform)>,
}

#[derive(Component)]
#[relationship(relationship_target = CharacterPhysicsEntities)]
pub struct OwnedByCharacter(pub Entity);

pub(crate) fn exclude_character_bodies(
    character_entities: &[Entity],
    owned_bodies: &Query<(Entity, &OwnedByCharacter)>,
) -> SpatialQueryFilter {
    SpatialQueryFilter::default().with_excluded_entities(
        owned_bodies
            .iter()
            .filter(|(_, owner)| character_entities.contains(&owner.0))
            .map(|(body_entity, _)| body_entity),
    )
}

#[derive(Component)]
#[relationship_target(relationship = OwnedByCharacter, linked_spawn)]
pub struct CharacterPhysicsEntities(Vec<Entity>);

impl CharacterPhysicsEntities {
    pub fn list_bodies(&self) -> &[Entity] {
        &self.0
    }
}

#[derive(Component, Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum RagdollBodyPart {
    Pelvis,
    Spine01,
    Spine02,
    Spine03,
    Neck01,
    Head,
    ClavicleLeft,
    UpperArmLeft,
    ForearmLeft,
    HandLeft,
    ClavicleRight,
    UpperArmRight,
    ForearmRight,
    HandRight,
    ThighLeft,
    CalfLeft,
    FootLeft,
    ThighRight,
    CalfRight,
    FootRight,
}

#[derive(Clone, Copy)]
pub(crate) enum RagdollColliderShape {
    JointHullCollider,
    SubtreeHullCollider,
}

#[derive(Component)]
pub(crate) struct PendingRagdollImpact {
    pub impulse: Vec3,
    pub point: Vec3,
}

#[derive(Component)]
pub(crate) struct PendingRagdollImpactRequest {
    pub(crate) body_part: RagdollBodyPart,
    pub(crate) impulse: Vec3,
    pub(crate) point: Vec3,
}

fn apply_pending_impacts(mut commands: Commands, mut impacted_bodies: Query<(Entity, &PendingRagdollImpact, Forces)>) {
    for (entity, impact, mut forces) in &mut impacted_bodies {
        forces.apply_linear_impulse_at_point(impact.impulse, impact.point);
        commands.entity(entity).remove::<PendingRagdollImpact>();
    }
}

fn wake_ragdoll_bodies_on_hit(
    hit: On<EnemyHit>,
    mut commands: Commands,
    ragdoll_bodies: Query<(&OwnedByCharacter, &RagdollBodyPart)>,
) {
    let Some(body_part) = ragdoll_bodies.get(hit.collider).ok().map(|(_, part)| *part) else {
        return;
    };
    let impact_point = hit.shot_origin + hit.shot_direction.as_vec3() * hit.impact_distance;
    commands.entity(hit.target).insert(PendingRagdollImpactRequest {
        body_part,
        impulse: hit.impulse,
        point: impact_point,
    });
}

#[derive(SystemParam)]
pub(crate) struct HullColliderSource<'w, 's> {
    skinned_meshes: Query<'w, 's, (&'static Mesh3d, &'static SkinnedMesh)>,
    mesh_assets: Res<'w, Assets<Mesh>>,
    inverse_bindposes: Res<'w, Assets<SkinnedMeshInverseBindposes>>,
    transforms: Query<'w, 's, &'static GlobalTransform>,
    children: Query<'w, 's, &'static Children>,
}

pub(crate) fn setup_ragdoll(
    scene_ready: On<WorldInstanceReady>,
    mut commands: Commands,
    children: Query<&Children>,
    names: Query<&Name>,
) {
    let root = scene_ready.entity;

    let mut bones: HashMap<Box<str>, Entity> = HashMap::new();
    for descendant in children.iter_descendants(root) {
        if let Ok(name) = names.get(descendant) {
            bones.insert(name.as_str().into(), descendant);
        }
    }

    let bone_map = BoneMap(bones);
    commands.entity(root).insert((bone_map, PendingRagdoll));
}

#[derive(Component)]
pub(crate) struct HullColliders(HashMap<RagdollBodyPart, Collider>);

impl HullColliders {
    pub(crate) fn collider_for(&self, body_part: RagdollBodyPart) -> &Collider {
        self.0
            .get(&body_part)
            .expect("every hull collider definition must have a matching extracted hull")
    }
}

fn create_hull_colliders(root: Entity, bones: &BoneMap, source: &HullColliderSource) -> HullColliders {
    let mut colliders = HashMap::new();
    for specification in LIMBS {
        let collider = create_gltf_hull_collider(root, specification, bones, source)
            .expect("skinned mesh should provide the required hull colliders");
        colliders.insert(specification.part, collider);
    }
    HullColliders(colliders)
}

fn get_hull_joint_name(body_part: RagdollBodyPart) -> Option<&'static str> {
    let specification = LIMBS.iter().find(|specification| specification.part == body_part)?;
    Some(specification.start)
}

fn create_gltf_hull_collider(
    root: Entity,
    specification: &LimbSpec,
    bones: &BoneMap,
    source: &HullColliderSource,
) -> Option<Collider> {
    let root_joint_entity = bones.get(get_hull_joint_name(specification.part)?)?;
    let (_, measured_limb) = measure_limb(specification, bones, &source.transforms)?;
    let included_joints = match specification.collider {
        RagdollColliderShape::SubtreeHullCollider => list_joint_subtree(root_joint_entity, source),
        RagdollColliderShape::JointHullCollider => HashSet::from([root_joint_entity]),
    };
    let world_vertices = collect_weighted_joint_vertices(root, &included_joints, source)?;
    let body_transform = measured_limb.segment.transform();
    let body_rotation_inverse = body_transform.rotation.inverse();
    let hull_vertices = world_vertices
        .iter()
        .map(|vertex_in_world| body_rotation_inverse * (*vertex_in_world - body_transform.translation))
        .collect::<Vec<_>>();
    Collider::convex_hull(hull_vertices)
}

fn list_joint_subtree(joint_entity: Entity, source: &HullColliderSource) -> HashSet<Entity> {
    let mut joint_entities = HashSet::from([joint_entity]);
    joint_entities.extend(source.children.iter_descendants(joint_entity));
    joint_entities
}

fn collect_weighted_joint_vertices(
    root: Entity,
    included_joints: &HashSet<Entity>,
    source: &HullColliderSource,
) -> Option<Vec<Vec3>> {
    let mut joint_vertices = Vec::new();
    for mesh_entity in source.children.iter_descendants(root) {
        let Ok((mesh_handle, skin)) = source.skinned_meshes.get(mesh_entity) else {
            continue;
        };
        let inverse_bindposes = source.inverse_bindposes.get(&skin.inverse_bindposes)?;
        let mut weighted_joints = Vec::new();
        for (joint_index, joint_entity) in skin.joints.iter().enumerate() {
            if !included_joints.contains(joint_entity) {
                continue;
            }
            weighted_joints.push(WeightedJoint {
                index: joint_index,
                inverse_bindpose: *inverse_bindposes.get(joint_index)?,
                world_transform: *source.transforms.get(*joint_entity).ok()?,
            });
        }
        if weighted_joints.is_empty() {
            continue;
        }
        let mesh = source.mesh_assets.get(&mesh_handle.0)?;
        joint_vertices.extend(select_weighted_vertices(mesh, &weighted_joints)?);
    }
    Some(joint_vertices)
}

struct WeightedJoint {
    index: usize,
    inverse_bindpose: Mat4,
    world_transform: GlobalTransform,
}

fn select_weighted_vertices(mesh: &Mesh, weighted_joints: &[WeightedJoint]) -> Option<Vec<Vec3>> {
    let VertexAttributeValues::Float32x3(positions) = mesh.try_attribute(Mesh::ATTRIBUTE_POSITION).ok()? else {
        return None;
    };
    let VertexAttributeValues::Uint16x4(joint_indices) = mesh.try_attribute(Mesh::ATTRIBUTE_JOINT_INDEX).ok()? else {
        return None;
    };
    let VertexAttributeValues::Float32x4(joint_weights) = mesh.try_attribute(Mesh::ATTRIBUTE_JOINT_WEIGHT).ok()? else {
        return None;
    };
    let triangle_vertex_indices = mesh.indices()?.iter().collect::<Vec<_>>();
    let mut vertex_is_in_triangle = vec![false; positions.len()];
    for vertex_index in triangle_vertex_indices {
        vertex_is_in_triangle[vertex_index] = true;
    }
    Some(
        positions
            .iter()
            .zip(joint_indices)
            .zip(joint_weights)
            .enumerate()
            .filter(|(vertex_index, _)| vertex_is_in_triangle[*vertex_index])
            .filter_map(|(_, ((position, mesh_joint_indices), weights))| {
                let strongest_influence_index = weights
                    .iter()
                    .enumerate()
                    .max_by(|(_, left_weight), (_, right_weight)| left_weight.total_cmp(right_weight))
                    .map(|(influence_index, _)| influence_index)?;
                let strongest_joint_index = usize::from(mesh_joint_indices[strongest_influence_index]);
                let weighted_joint = weighted_joints
                    .iter()
                    .find(|weighted_joint| weighted_joint.index == strongest_joint_index)?;
                let vertex_in_joint_frame = weighted_joint.inverse_bindpose.transform_point3(Vec3::from(*position));
                Some(weighted_joint.world_transform.transform_point(vertex_in_joint_frame))
            })
            .collect::<Vec<_>>(),
    )
}

#[derive(Clone, Copy)]
pub(crate) struct Segment {
    pub(crate) start: Vec3,
    pub(crate) midpoint: Vec3,
    pub(crate) rotation: Quat,
}

impl Segment {
    pub(crate) fn create_between(start: Vec3, end: Vec3) -> Option<Self> {
        let direction = (end - start).try_normalize()?;
        Some(Self {
            start,
            midpoint: (start + end) * 0.5,
            rotation: Quat::from_rotation_arc(Vec3::Y, direction),
        })
    }

    pub(crate) fn transform(self) -> Transform {
        Transform::from_translation(self.midpoint).with_rotation(self.rotation)
    }

    fn create_at(position: Vec3, rotation: Quat) -> Self {
        Self {
            start: position,
            midpoint: position,
            rotation,
        }
    }
}

#[derive(Clone, Copy)]
enum JointSpec {
    Spherical { angular_damping: f32 },
    Revolute { angle_limits: (f32, f32) },
}

pub(crate) struct LimbSpec {
    pub(crate) part: RagdollBodyPart,
    start: &'static str,
    end: &'static str,
    parent: Option<RagdollBodyPart>,
    joint: Option<JointSpec>,
    collider: RagdollColliderShape,
    color: Color,
}

impl LimbSpec {
    fn list_bone_names(&self) -> Vec<&'static str> {
        vec![self.start, self.end]
    }

    fn list_driven_bones(&self) -> Vec<&'static str> {
        vec![self.start]
    }
}

pub(crate) const LIMBS: &[LimbSpec] = &[
    LimbSpec {
        part: RagdollBodyPart::Pelvis,
        start: "pelvis",
        end: "spine_01",
        parent: None,
        joint: None,
        collider: RagdollColliderShape::JointHullCollider,
        color: Color::srgb(1.0, 0.55, 0.55),
    },
    LimbSpec {
        part: RagdollBodyPart::Spine01,
        start: "spine_01",
        end: "spine_02",
        parent: Some(RagdollBodyPart::Pelvis),
        joint: Some(JointSpec::Spherical {
            angular_damping: JOINT_ANGULAR_DAMPING,
        }),
        collider: RagdollColliderShape::JointHullCollider,
        color: Color::srgb(1.0, 0.65, 0.55),
    },
    LimbSpec {
        part: RagdollBodyPart::Spine02,
        start: "spine_02",
        end: "spine_03",
        parent: Some(RagdollBodyPart::Spine01),
        joint: Some(JointSpec::Spherical {
            angular_damping: JOINT_ANGULAR_DAMPING,
        }),
        collider: RagdollColliderShape::JointHullCollider,
        color: Color::srgb(1.0, 0.75, 0.55),
    },
    LimbSpec {
        part: RagdollBodyPart::Spine03,
        start: "spine_03",
        end: "neck_01",
        parent: Some(RagdollBodyPart::Spine02),
        joint: Some(JointSpec::Spherical {
            angular_damping: JOINT_ANGULAR_DAMPING,
        }),
        collider: RagdollColliderShape::JointHullCollider,
        color: Color::srgb(1.0, 0.85, 0.55),
    },
    LimbSpec {
        part: RagdollBodyPart::Neck01,
        start: "neck_01",
        end: "Head",
        parent: Some(RagdollBodyPart::Spine03),
        joint: Some(JointSpec::Spherical {
            angular_damping: NECK_ANGULAR_DAMPING,
        }),
        collider: RagdollColliderShape::JointHullCollider,
        color: Color::srgb(0.95, 0.95, 0.6),
    },
    LimbSpec {
        part: RagdollBodyPart::Head,
        start: "Head",
        end: "Head",
        parent: Some(RagdollBodyPart::Neck01),
        joint: Some(JointSpec::Spherical {
            angular_damping: NECK_ANGULAR_DAMPING,
        }),
        collider: RagdollColliderShape::SubtreeHullCollider,
        color: Color::srgb(1.0, 1.0, 0.6),
    },
    LimbSpec {
        part: RagdollBodyPart::ClavicleLeft,
        start: "clavicle_l",
        end: "upperarm_l",
        parent: Some(RagdollBodyPart::Spine03),
        joint: Some(JointSpec::Spherical {
            angular_damping: JOINT_ANGULAR_DAMPING,
        }),
        collider: RagdollColliderShape::JointHullCollider,
        color: Color::srgb(1.0, 0.7, 0.7),
    },
    LimbSpec {
        part: RagdollBodyPart::UpperArmLeft,
        start: "upperarm_l",
        end: "lowerarm_l",
        parent: Some(RagdollBodyPart::ClavicleLeft),
        joint: Some(JointSpec::Spherical {
            angular_damping: JOINT_ANGULAR_DAMPING,
        }),
        collider: RagdollColliderShape::JointHullCollider,
        color: Color::srgb(0.6, 1.0, 0.6),
    },
    LimbSpec {
        part: RagdollBodyPart::ForearmLeft,
        start: "lowerarm_l",
        end: "hand_l",
        parent: Some(RagdollBodyPart::UpperArmLeft),
        joint: Some(JointSpec::Revolute {
            angle_limits: (-2.6, 0.1),
        }),
        collider: RagdollColliderShape::JointHullCollider,
        color: Color::srgb(0.55, 0.95, 0.8),
    },
    LimbSpec {
        part: RagdollBodyPart::HandLeft,
        start: "hand_l",
        end: "middle_04_leaf_l",
        parent: Some(RagdollBodyPart::ForearmLeft),
        joint: Some(JointSpec::Spherical {
            angular_damping: WRIST_ANGULAR_DAMPING,
        }),
        collider: RagdollColliderShape::SubtreeHullCollider,
        color: Color::srgb(0.6, 0.9, 1.0),
    },
    LimbSpec {
        part: RagdollBodyPart::ClavicleRight,
        start: "clavicle_r",
        end: "upperarm_r",
        parent: Some(RagdollBodyPart::Spine03),
        joint: Some(JointSpec::Spherical {
            angular_damping: JOINT_ANGULAR_DAMPING,
        }),
        collider: RagdollColliderShape::JointHullCollider,
        color: Color::srgb(1.0, 0.7, 0.7),
    },
    LimbSpec {
        part: RagdollBodyPart::UpperArmRight,
        start: "upperarm_r",
        end: "lowerarm_r",
        parent: Some(RagdollBodyPart::ClavicleRight),
        joint: Some(JointSpec::Spherical {
            angular_damping: JOINT_ANGULAR_DAMPING,
        }),
        collider: RagdollColliderShape::JointHullCollider,
        color: Color::srgb(0.6, 1.0, 0.6),
    },
    LimbSpec {
        part: RagdollBodyPart::ForearmRight,
        start: "lowerarm_r",
        end: "hand_r",
        parent: Some(RagdollBodyPart::UpperArmRight),
        joint: Some(JointSpec::Revolute {
            angle_limits: (-0.1, 2.6),
        }),
        collider: RagdollColliderShape::JointHullCollider,
        color: Color::srgb(0.55, 0.95, 0.8),
    },
    LimbSpec {
        part: RagdollBodyPart::HandRight,
        start: "hand_r",
        end: "middle_04_leaf_r",
        parent: Some(RagdollBodyPart::ForearmRight),
        joint: Some(JointSpec::Spherical {
            angular_damping: WRIST_ANGULAR_DAMPING,
        }),
        collider: RagdollColliderShape::SubtreeHullCollider,
        color: Color::srgb(0.6, 0.9, 1.0),
    },
    LimbSpec {
        part: RagdollBodyPart::ThighLeft,
        start: "thigh_l",
        end: "calf_l",
        parent: Some(RagdollBodyPart::Pelvis),
        joint: Some(JointSpec::Spherical {
            angular_damping: HIP_ANGULAR_DAMPING,
        }),
        collider: RagdollColliderShape::JointHullCollider,
        color: Color::srgb(0.6, 0.75, 1.0),
    },
    LimbSpec {
        part: RagdollBodyPart::CalfLeft,
        start: "calf_l",
        end: "foot_l",
        parent: Some(RagdollBodyPart::ThighLeft),
        joint: Some(JointSpec::Revolute {
            angle_limits: (-0.1, 2.6),
        }),
        collider: RagdollColliderShape::JointHullCollider,
        color: Color::srgb(0.7, 0.7, 1.0),
    },
    LimbSpec {
        part: RagdollBodyPart::FootLeft,
        start: "foot_l",
        end: "ball_leaf_l",
        parent: Some(RagdollBodyPart::CalfLeft),
        joint: Some(JointSpec::Spherical {
            angular_damping: ANKLE_ANGULAR_DAMPING,
        }),
        collider: RagdollColliderShape::SubtreeHullCollider,
        color: Color::srgb(0.85, 0.65, 1.0),
    },
    LimbSpec {
        part: RagdollBodyPart::ThighRight,
        start: "thigh_r",
        end: "calf_r",
        parent: Some(RagdollBodyPart::Pelvis),
        joint: Some(JointSpec::Spherical {
            angular_damping: HIP_ANGULAR_DAMPING,
        }),
        collider: RagdollColliderShape::JointHullCollider,
        color: Color::srgb(0.6, 0.75, 1.0),
    },
    LimbSpec {
        part: RagdollBodyPart::CalfRight,
        start: "calf_r",
        end: "foot_r",
        parent: Some(RagdollBodyPart::ThighRight),
        joint: Some(JointSpec::Revolute {
            angle_limits: (-0.1, 2.6),
        }),
        collider: RagdollColliderShape::JointHullCollider,
        color: Color::srgb(0.7, 0.7, 1.0),
    },
    LimbSpec {
        part: RagdollBodyPart::FootRight,
        start: "foot_r",
        end: "ball_leaf_r",
        parent: Some(RagdollBodyPart::CalfRight),
        joint: Some(JointSpec::Spherical {
            angular_damping: ANKLE_ANGULAR_DAMPING,
        }),
        collider: RagdollColliderShape::SubtreeHullCollider,
        color: Color::srgb(0.85, 0.65, 1.0),
    },
];

fn get_bone_position(name: &str, bones: &BoneMap, transforms: &Query<&GlobalTransform>) -> Option<Vec3> {
    let entity = bones.get(name)?;
    transforms.get(entity).ok().map(GlobalTransform::translation)
}

pub(crate) fn measure_limb(
    spec: &LimbSpec,
    bones: &BoneMap,
    transforms: &Query<&GlobalTransform>,
) -> Option<(Transform, MeasuredLimb)> {
    let start = get_bone_position(spec.start, bones, transforms)?;
    let end = get_bone_position(spec.end, bones, transforms)?;
    let segment = if let Some(segment) = Segment::create_between(start, end) {
        segment
    } else {
        let start_joint_entity = bones.get(spec.start)?;
        let start_joint_transform = transforms.get(start_joint_entity).ok()?;
        Segment::create_at(start, start_joint_transform.rotation())
    };
    Some((segment.transform(), MeasuredLimb { anchor: start, segment }))
}

fn draw_measured_ragdoll_bones(
    gizmos: Option<Gizmos<RagdollBoneGizmos>>,
    bone_maps: Query<&BoneMap>,
    transforms: Query<&GlobalTransform>,
) {
    let Some(mut gizmos) = gizmos else {
        return;
    };
    let measured_bone_color = Color::srgb(0.2, 0.2, 1.0);
    for bones in &bone_maps {
        for specification in LIMBS {
            let Some((_, measured_limb)) = measure_limb(specification, bones, &transforms) else {
                continue;
            };
            let segment_end = measured_limb.segment.midpoint * 2.0 - measured_limb.segment.start;
            gizmos.line(measured_limb.segment.start, segment_end, measured_bone_color);
        }
    }
}

#[derive(Clone, Copy)]
pub(crate) struct MeasuredLimb {
    pub(crate) anchor: Vec3,
    pub(crate) segment: Segment,
}

#[derive(SystemParam)]
struct RagdollSceneContext<'w, 's> {
    transforms: Query<'w, 's, &'static GlobalTransform>,
    parents: Query<'w, 's, &'static ChildOf>,
}

struct SphericalJointBodies {
    enemy: Entity,
    parent: Entity,
    child: Entity,
    parent_segment: Segment,
    child_segment: Segment,
    anchor: Vec3,
}

fn spawn_spherical_joint(
    commands: &mut Commands,
    bodies: SphericalJointBodies,
    angular_damping: f32,
    swing_limits: (f32, f32),
    twist_limits: (f32, f32),
) -> Entity {
    let parent_local_anchor =
        bodies.parent_segment.rotation.inverse() * (bodies.anchor - bodies.parent_segment.midpoint);
    let child_local_anchor = bodies.child_segment.rotation.inverse() * (bodies.anchor - bodies.child_segment.midpoint);
    commands
        .spawn((
            OwnedByCharacter(bodies.enemy),
            SphericalJoint::new(bodies.parent, bodies.child)
                .with_local_anchor1(parent_local_anchor)
                .with_local_anchor2(child_local_anchor)
                .with_local_basis1(Quat::IDENTITY)
                .with_local_basis2(bodies.child_segment.rotation.inverse() * bodies.parent_segment.rotation)
                .with_swing_limits(swing_limits.0, swing_limits.1)
                .with_twist_limits(twist_limits.0, twist_limits.1),
            JointDamping {
                linear: 0.0,
                angular: angular_damping,
            },
            JointCollisionDisabled,
        ))
        .id()
}

struct JointBodies {
    enemy: Entity,
    parent: Entity,
    child: Entity,
}

fn spawn_revolute_joint(
    commands: &mut Commands,
    bodies: JointBodies,
    parent_segment: Segment,
    child_segment: Segment,
    compute_hinge_axis: Vec3,
    angle_limits: (f32, f32),
) -> Entity {
    commands
        .spawn((
            OwnedByCharacter(bodies.enemy),
            make_revolute_joint(
                bodies.parent,
                bodies.child,
                parent_segment,
                child_segment,
                compute_hinge_axis,
                angle_limits,
            ),
            JointDamping {
                linear: 0.0,
                angular: JOINT_ANGULAR_DAMPING,
            },
            JointCollisionDisabled,
        ))
        .id()
}

fn make_revolute_joint(
    parent: Entity,
    child: Entity,
    parent_segment: Segment,
    child_segment: Segment,
    compute_hinge_axis: Vec3,
    angle_limits: (f32, f32),
) -> RevoluteJoint {
    let parent_axis = parent_segment.rotation * Vec3::Y;
    let hinge_perpendicular_axis = (parent_axis - compute_hinge_axis * parent_axis.dot(compute_hinge_axis))
        .try_normalize()
        .unwrap_or(Vec3::X);
    let hinge_tangent_axis = compute_hinge_axis.cross(hinge_perpendicular_axis);
    let world_joint_basis = Quat::from_mat3(&Mat3::from_cols(
        hinge_perpendicular_axis,
        hinge_tangent_axis,
        compute_hinge_axis,
    ));
    let anchor = child_segment.start;

    RevoluteJoint::new(parent, child)
        .with_local_anchor1(parent_segment.rotation.inverse() * (anchor - parent_segment.midpoint))
        .with_local_anchor2(child_segment.rotation.inverse() * (anchor - child_segment.midpoint))
        .with_local_basis1(parent_segment.rotation.inverse() * world_joint_basis)
        .with_local_basis2(child_segment.rotation.inverse() * world_joint_basis)
        .with_angle_limits(angle_limits.0, angle_limits.1)
}

fn spawn_body(
    commands: &mut Commands,
    character: Entity,
    spec: &LimbSpec,
    transform: Transform,
    collider: Collider,
    show_collider_gizmos: bool,
) -> Entity {
    commands
        .spawn((
            Name::new(spec.start),
            OwnedByCharacter(character),
            spec.part,
            RigidBody::Dynamic,
            AngularDamping(ANGULAR_DAMPING),
            LinearDamping(LINEAR_DAMPING),
            SleepThreshold {
                linear: 0.15,
                angular: RAGDOLL_ANGULAR_SLEEP_THRESHOLD,
            },
            ColliderDensity(COLLIDER_DENSITY),
            CollisionLayers::new(RAGDOLL_GROUP, WORLD_GROUP | RAGDOLL_GROUP),
            Friction::new(0.1).with_combine_rule(CoefficientCombine::Min),
            DebugRender {
                axis_lengths: None,
                collider_color: show_collider_gizmos.then_some(spec.color),
                ..default()
            },
            transform,
            collider,
        ))
        .id()
}

fn compute_hinge_axis(limbs: &HashMap<RagdollBodyPart, MeasuredLimb>) -> Vec3 {
    let left_shoulder = limbs[&RagdollBodyPart::UpperArmLeft].anchor;
    let right_shoulder = limbs[&RagdollBodyPart::UpperArmRight].anchor;
    let shoulder_axis = (left_shoulder - right_shoulder).try_normalize().unwrap_or(Vec3::X);
    let chest_axis = limbs[&RagdollBodyPart::Spine03].segment.rotation * Vec3::Y;
    shoulder_axis.cross(chest_axis).try_normalize().unwrap_or(Vec3::Z)
}

fn find_nearest_driver(
    entity: Entity,
    drivers: &HashMap<Entity, Entity>,
    parents: &Query<&ChildOf>,
    default: Entity,
) -> Entity {
    let mut current = entity;
    loop {
        if let Some(&part) = drivers.get(&current) {
            return part;
        }
        let Some(parent) = parents.get(current).ok().map(ChildOf::parent) else {
            return default;
        };
        current = parent;
    }
}

type PendingRagdollQuery<'w, 's> = Query<
    'w,
    's,
    (
        Entity,
        &'static BoneMap,
        &'static PendingRagdoll,
        Has<Dying>,
        Has<RagdollData>,
        Has<Player>,
    ),
>;

fn spawn_ragdoll_bodies(
    mut commands: Commands,
    pending: PendingRagdollQuery<'_, '_>,
    scene: RagdollSceneContext,
    hull_collider_source: HullColliderSource,
) {
    for (character_entity, bone_map, _, is_dying, has_ragdoll, is_player) in &pending {
        if has_ragdoll {
            remove_pending_ragdoll(&mut commands, character_entity);
            continue;
        }

        let hull_colliders = create_hull_colliders(character_entity, bone_map, &hull_collider_source);

        let Some(ragdoll) = rig::assemble_from_bones_and_colliders(
            &mut commands,
            character_entity,
            bone_map,
            rig::RagdollAssembleContext {
                hull_colliders: &hull_colliders,
                transforms: &scene.transforms,
                parents: &scene.parents,
                is_disabled: !is_dying,
                show_collider_gizmos: !is_player,
            },
        ) else {
            remove_pending_ragdoll(&mut commands, character_entity);
            continue;
        };
        commands.entity(character_entity).insert(ragdoll);
        remove_pending_ragdoll(&mut commands, character_entity);
    }
}

fn remove_pending_ragdoll(commands: &mut Commands, character_entity: Entity) {
    commands.entity(character_entity).remove::<PendingRagdoll>();
}

pub(crate) fn measure_limbs(
    character_entity: Entity,
    bone_map: &BoneMap,
    transforms: &Query<&GlobalTransform>,
) -> Option<HashMap<RagdollBodyPart, MeasuredLimb>> {
    let missing_bone = LIMBS
        .iter()
        .flat_map(LimbSpec::list_bone_names)
        .find(|bone_name| bone_map.get(bone_name).is_none());
    if let Some(bone_name) = missing_bone {
        warn!("Cannot create ragdoll for {character_entity:?}: missing bone {bone_name}");
        return None;
    }

    let mut measured_limbs = HashMap::new();
    for specification in LIMBS {
        let Some((_, measured_limb)) = measure_limb(specification, bone_map, transforms) else {
            warn!(
                "Cannot create ragdoll for {character_entity:?}: cannot measure limb {}",
                specification.start
            );
            return None;
        };
        measured_limbs.insert(specification.part, measured_limb);
    }

    Some(measured_limbs)
}

pub(crate) struct SpawnedRagdollColliders {
    pub(crate) part_entities: HashMap<RagdollBodyPart, Entity>,
    pub(crate) parts: Vec<RigPart>,
}

pub(crate) fn spawn_ragdoll_colliders(
    commands: &mut Commands,
    character_entity: Entity,
    hull_colliders: &HullColliders,
    measured_limbs: &HashMap<RagdollBodyPart, MeasuredLimb>,
    is_dying: bool,
    show_colliders: bool,
) -> SpawnedRagdollColliders {
    let mut part_entities = HashMap::new();
    let mut parts = Vec::with_capacity(LIMBS.len());
    for specification in LIMBS {
        let measured_limb = measured_limbs[&specification.part];
        let transform = measured_limb.segment.transform();
        let collider = hull_colliders.collider_for(specification.part).clone();
        let entity = spawn_body(commands, character_entity, specification, transform, collider, show_colliders);
        if !is_dying {
            commands.entity(entity).insert(RigidBodyDisabled);
        }
        part_entities.insert(specification.part, entity);
        parts.push(RigPart {
            entity,
            body_part: specification.part,
            initial: GlobalTransform::from(transform),
            bones: Vec::new(),
        });
    }
    SpawnedRagdollColliders { part_entities, parts }
}

pub(crate) fn spawn_joints(
    commands: &mut Commands,
    enemy: Entity,
    part_entities: &HashMap<RagdollBodyPart, Entity>,
    limbs: &HashMap<RagdollBodyPart, MeasuredLimb>,
) {
    for spec in LIMBS {
        let (Some(parent_part), Some(joint)) = (spec.parent, spec.joint) else {
            continue;
        };
        spawn_joint(
            commands,
            JointEndpoints {
                root: enemy,
                parent_part,
                child_part: spec.part,
                parent_entity: part_entities[&parent_part],
                child_entity: part_entities[&spec.part],
            },
            joint,
            limbs,
        );
    }
}

struct JointEndpoints {
    root: Entity,
    parent_part: RagdollBodyPart,
    child_part: RagdollBodyPart,
    parent_entity: Entity,
    child_entity: Entity,
}

fn get_spherical_joint_limits(body_part: RagdollBodyPart) -> ((f32, f32), (f32, f32)) {
    match body_part {
        RagdollBodyPart::Spine01 | RagdollBodyPart::Spine02 => ((-0.45, 0.45), (-0.30, 0.30)),
        RagdollBodyPart::Spine03 => ((-0.45, 0.45), (-0.35, 0.35)),
        RagdollBodyPart::Neck01 => ((-0.50, 0.50), (-0.40, 0.40)),
        RagdollBodyPart::Head => ((-0.60, 0.60), (-0.50, 0.50)),
        RagdollBodyPart::ClavicleLeft | RagdollBodyPart::ClavicleRight => ((-0.60, 0.60), (-0.25, 0.25)),
        RagdollBodyPart::UpperArmLeft | RagdollBodyPart::UpperArmRight => ((-1.60, 1.60), (-0.90, 0.90)),
        RagdollBodyPart::HandLeft | RagdollBodyPart::HandRight => ((-0.55, 0.55), (-0.45, 0.45)),
        RagdollBodyPart::ThighLeft | RagdollBodyPart::ThighRight => ((-1.25, 1.25), (-0.70, 0.70)),
        RagdollBodyPart::FootLeft | RagdollBodyPart::FootRight => ((-0.50, 0.50), (-0.25, 0.25)),
        RagdollBodyPart::Pelvis
        | RagdollBodyPart::ForearmLeft
        | RagdollBodyPart::ForearmRight
        | RagdollBodyPart::CalfLeft
        | RagdollBodyPart::CalfRight => unreachable!("only spherical joints request spherical limits"),
    }
}

fn spawn_joint(
    commands: &mut Commands,
    endpoints: JointEndpoints,
    specification: JointSpec,
    limbs: &HashMap<RagdollBodyPart, MeasuredLimb>,
) -> Entity {
    let JointEndpoints {
        root,
        parent_part,
        child_part,
        parent_entity,
        child_entity,
    } = endpoints;
    let child_measured = limbs[&child_part];
    match specification {
        JointSpec::Spherical { angular_damping } => {
            let (swing_limits, twist_limits) = get_spherical_joint_limits(child_part);
            spawn_spherical_joint(
                commands,
                SphericalJointBodies {
                    enemy: root,
                    parent: parent_entity,
                    child: child_entity,
                    parent_segment: limbs[&parent_part].segment,
                    child_segment: child_measured.segment,
                    anchor: child_measured.anchor,
                },
                angular_damping,
                swing_limits,
                twist_limits,
            )
        }
        JointSpec::Revolute { angle_limits } => spawn_revolute_joint(
            commands,
            JointBodies {
                enemy: root,
                parent: parent_entity,
                child: child_entity,
            },
            limbs[&parent_part].segment,
            child_measured.segment,
            compute_hinge_axis(limbs),
            angle_limits,
        ),
    }
}

pub(crate) fn assign_drivers(
    bones: &BoneMap,
    part_entities: HashMap<RagdollBodyPart, Entity>,
    parents: &Query<&ChildOf>,
    transforms: &Query<&GlobalTransform>,
    torso_entity: Entity,
) -> HashMap<Entity, Vec<(Entity, GlobalTransform)>> {
    let mut drivers: HashMap<Entity, Entity> = HashMap::new();
    for spec in LIMBS {
        let part_entity = part_entities[&spec.part];
        for name in spec.list_driven_bones() {
            if let Some(bone_entity) = bones.get(name) {
                drivers.insert(bone_entity, part_entity);
            }
        }
    }

    let mut bones_by_part: HashMap<Entity, Vec<(Entity, GlobalTransform)>> = HashMap::new();
    for bone_entity in bones.list_ragdoll_entities() {
        let Ok(transform) = transforms.get(bone_entity).copied() else {
            continue;
        };
        let part = find_nearest_driver(bone_entity, &drivers, parents, torso_entity);
        bones_by_part.entry(part).or_default().push((bone_entity, transform));
    }
    bones_by_part
}

fn synchronize_ragdoll_bodies(
    mut ragdolls: Query<(&mut RagdollData, Option<Ref<Dying>>)>,
    bone_transforms: Query<&GlobalTransform>,
    mut body_transforms: Query<(&mut Position, &mut Rotation, &mut Transform), With<RagdollBodyPart>>,
) {
    for (mut ragdoll, dying) in &mut ragdolls {
        if dying.as_ref().is_some_and(|dying| !dying.is_added()) {
            continue;
        }

        rig::synchronize_from_bones(&mut ragdoll, &bone_transforms, &mut body_transforms);
    }
}

type DyingRagdollRequestQuery<'w, 's> = Query<
    'w,
    's,
    (
        Entity,
        &'static PendingRagdollImpactRequest,
        &'static RagdollData,
        Option<&'static LinearVelocity>,
        Has<Dying>,
    ),
>;

fn activate_ragdoll_on_death(mut commands: Commands, pending: DyingRagdollRequestQuery) {
    for (root, impact, ragdoll, character_velocity, dying) in &pending {
        if !dying || !ragdoll.has_all_limbs() {
            continue;
        }

        let character_velocity = character_velocity.copied().unwrap_or_default();
        rig::activate_with_impact(&mut commands, ragdoll, impact, character_velocity, root);
        commands.entity(root).remove::<PendingRagdollImpactRequest>();
    }
}

fn apply_ragdoll_pose(
    ragdolls: Query<&RagdollData, With<Dying>>,
    positions: Query<(&Position, &Rotation)>,
    mut bone_globals: Query<&mut GlobalTransform>,
) {
    for ragdoll in &ragdolls {
        rig::apply_pose_to_bones(ragdoll, &positions, &mut bone_globals);
    }
}
