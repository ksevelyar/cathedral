use crate::enemies::{Dying, EnemyHit};
use avian3d::math::AsF32;
use avian3d::prelude::*;
use bevy::ecs::system::SystemParam;
use bevy::mesh::skinning::SkinnedMesh;
use bevy::mesh::{Mesh3d, VertexAttributeValues};
use bevy::prelude::*;
use bevy::transform::TransformSystems;
use bevy::world_serialization::WorldInstanceReady;
use std::collections::HashMap;

pub(crate) const RAGDOLL_GROUP: u32 = 0b10;
pub(crate) const OBJECTS_GROUP: u32 = 0b100;
pub(crate) const WORLD_GROUP: u32 = 0b01;

pub struct RagdollPlugin;

impl Plugin for RagdollPlugin {
    fn build(&self, app: &mut App) {
        app.add_observer(wake_ragdoll_bodies_on_hit)
            .add_systems(
                PostUpdate,
                (
                    spawn_ragdoll_bodies,
                    synchronize_ragdoll_bodies,
                    activate_ragdoll_on_death,
                    apply_ragdoll_pose,
                )
                    .chain()
                    .after(TransformSystems::Propagate),
            )
            .add_systems(
                FixedPostUpdate,
                apply_pending_impacts
                    .after(PhysicsSystems::Prepare)
                    .before(PhysicsSystems::StepSimulation),
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

const HEAD_CROWN_OFFSET: f32 = 0.30;
const COLLIDER_FIT_PERCENTILE: f32 = 0.9;
const CUBOID_CROSS_SECTION_PERCENTILE: f32 = 0.65;
const MINIMUM_COLLIDER_RADIUS: f32 = 0.03;
const DOMINANT_WEIGHT_THRESHOLD: f32 = 0.3;
const COLLIDER_DENSITY: f32 = 1000.0;
const HAND_LENGTH: f32 = 0.24;
const FOOT_LENGTH: f32 = 0.24;

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

#[derive(Component)]
struct RagdollData {
    bones: BoneMap,
    limbs: HashMap<RagdollBodyPart, MeasuredLimb>,
    parts: Vec<RigPart>,
}

struct RigPart {
    entity: Entity,
    body_part: RagdollBodyPart,
    initial: GlobalTransform,
    bones: Vec<(Entity, GlobalTransform)>,
}

#[derive(Component)]
#[relationship(relationship_target = EnemyPhysicsEntities)]
pub struct OwnedByEnemy(pub Entity);

#[derive(Component)]
#[relationship_target(relationship = OwnedByEnemy, linked_spawn)]
pub struct EnemyPhysicsEntities(Vec<Entity>);

#[derive(Component)]
#[relationship(relationship_target = JointsAnchoredToBody)]
struct JointAnchoredToBody(Entity);

#[derive(Component)]
#[relationship_target(relationship = JointAnchoredToBody)]
struct JointsAnchoredToBody(Vec<Entity>);

#[derive(Component, Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum RagdollBodyPart {
    Torso,
    Head,
    LeftUpperArm,
    LeftForearm,
    LeftHand,
    RightUpperArm,
    RightForearm,
    RightHand,
    LeftThigh,
    LeftCalf,
    LeftFoot,
    RightThigh,
    RightCalf,
    RightFoot,
}

#[derive(Component)]
pub(crate) struct PendingRagdollImpact {
    pub impulse: Vec3,
    pub point: Vec3,
}

#[derive(Component)]
struct PendingRagdollImpactRequest {
    body_part: RagdollBodyPart,
    impulse: Vec3,
    point: Vec3,
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
    ragdoll_bodies: Query<(&OwnedByEnemy, &RagdollBodyPart)>,
) {
    let Some(body_part) = ragdoll_bodies.get(hit.body).ok().map(|(_, part)| *part) else {
        return;
    };
    commands.entity(hit.entity).insert(PendingRagdollImpactRequest {
        body_part,
        impulse: hit.impulse,
        point: hit.point,
    });
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

    commands.entity(root).insert((BoneMap(bones), PendingRagdoll));
}

#[derive(Clone, Copy)]
struct Segment {
    start: Vec3,
    midpoint: Vec3,
    length: f32,
    rotation: Quat,
}

impl Segment {
    fn create_between(start: Vec3, end: Vec3) -> Option<Self> {
        let direction = (end - start).try_normalize()?;
        Some(Self {
            start,
            midpoint: (start + end) * 0.5,
            length: (end - start).length(),
            rotation: Quat::from_rotation_arc(Vec3::Y, direction),
        })
    }

    fn transform(self) -> Transform {
        Transform::from_translation(self.midpoint).with_rotation(self.rotation)
    }
}

#[derive(Clone, Copy)]
enum BonePoint {
    Joint(&'static str),
    Raised { bone: &'static str, local_offset: Vec3 },
    Extended { toward: &'static str, length: f32 },
}

#[derive(Clone, Copy)]
enum Shape {
    Capsule,
    Cuboid,
}

#[derive(Clone, Copy)]
enum JointSpec {
    Spherical { angular_damping: f32 },
    Revolute { angle_limits: (f32, f32) },
}

struct LimbSpec {
    part: RagdollBodyPart,
    name: &'static str,
    shape: Shape,
    start: BonePoint,
    end: BonePoint,
    bucket_bones: &'static [&'static str],
    parent: Option<RagdollBodyPart>,
    joint: Option<JointSpec>,
}

impl LimbSpec {
    fn list_bone_names(&self) -> Vec<&'static str> {
        let mut names = Vec::new();
        match self.start {
            BonePoint::Joint(name) => names.push(name),
            BonePoint::Raised { bone, .. } => names.push(bone),
            BonePoint::Extended { toward, .. } => names.push(toward),
        }
        match self.end {
            BonePoint::Joint(name) => names.push(name),
            BonePoint::Raised { bone, .. } => names.push(bone),
            BonePoint::Extended { toward, .. } => names.push(toward),
        }
        names
    }

    fn list_driven_bones(&self) -> Vec<&'static str> {
        let mut names = Vec::new();
        if let BonePoint::Joint(name) = self.start {
            names.push(name);
        }
        match self.end {
            BonePoint::Raised { bone, .. } => names.push(bone),
            BonePoint::Extended { toward, .. } => names.push(toward),
            _ => {}
        }
        names
    }
}

const LIMBS: &[LimbSpec] = &[
    LimbSpec {
        part: RagdollBodyPart::Torso,
        name: "torso",
        shape: Shape::Capsule,
        start: BonePoint::Joint("pelvis"),
        end: BonePoint::Joint("neck_01"),
        bucket_bones: &["pelvis", "spine_01", "spine_02", "spine_03", "neck_01"],
        parent: None,
        joint: None,
    },
    LimbSpec {
        part: RagdollBodyPart::Head,
        name: "head",
        shape: Shape::Capsule,
        start: BonePoint::Joint("neck_01"),
        end: BonePoint::Raised {
            bone: "Head",
            local_offset: Vec3::new(0.0, HEAD_CROWN_OFFSET, 0.0),
        },
        bucket_bones: &["Head"],
        parent: Some(RagdollBodyPart::Torso),
        joint: Some(JointSpec::Spherical {
            angular_damping: NECK_ANGULAR_DAMPING,
        }),
    },
    LimbSpec {
        part: RagdollBodyPart::LeftUpperArm,
        name: "left_upper_arm",
        shape: Shape::Capsule,
        start: BonePoint::Joint("upperarm_l"),
        end: BonePoint::Joint("lowerarm_l"),
        bucket_bones: &["upperarm_l"],
        parent: Some(RagdollBodyPart::Torso),
        joint: Some(JointSpec::Spherical {
            angular_damping: JOINT_ANGULAR_DAMPING,
        }),
    },
    LimbSpec {
        part: RagdollBodyPart::RightUpperArm,
        name: "right_upper_arm",
        shape: Shape::Capsule,
        start: BonePoint::Joint("upperarm_r"),
        end: BonePoint::Joint("lowerarm_r"),
        bucket_bones: &["upperarm_r"],
        parent: Some(RagdollBodyPart::Torso),
        joint: Some(JointSpec::Spherical {
            angular_damping: JOINT_ANGULAR_DAMPING,
        }),
    },
    LimbSpec {
        part: RagdollBodyPart::LeftForearm,
        name: "left_forearm",
        shape: Shape::Capsule,
        start: BonePoint::Joint("lowerarm_l"),
        end: BonePoint::Joint("hand_l"),
        bucket_bones: &["lowerarm_l"],
        parent: Some(RagdollBodyPart::LeftUpperArm),
        joint: Some(JointSpec::Revolute {
            angle_limits: (-2.6, 0.1),
        }),
    },
    LimbSpec {
        part: RagdollBodyPart::LeftHand,
        name: "left_hand",
        shape: Shape::Cuboid,
        start: BonePoint::Joint("hand_l"),
        end: BonePoint::Extended {
            toward: "middle_01_l",
            length: HAND_LENGTH,
        },
        bucket_bones: &[
            "hand_l",
            "index_01_l",
            "middle_01_l",
            "pinky_01_l",
            "ring_01_l",
            "thumb_01_l",
        ],
        parent: Some(RagdollBodyPart::LeftForearm),
        joint: Some(JointSpec::Spherical {
            angular_damping: WRIST_ANGULAR_DAMPING,
        }),
    },
    LimbSpec {
        part: RagdollBodyPart::RightForearm,
        name: "right_forearm",
        shape: Shape::Capsule,
        start: BonePoint::Joint("lowerarm_r"),
        end: BonePoint::Joint("hand_r"),
        bucket_bones: &["lowerarm_r"],
        parent: Some(RagdollBodyPart::RightUpperArm),
        joint: Some(JointSpec::Revolute {
            angle_limits: (-0.1, 2.6),
        }),
    },
    LimbSpec {
        part: RagdollBodyPart::RightHand,
        name: "right_hand",
        shape: Shape::Cuboid,
        start: BonePoint::Joint("hand_r"),
        end: BonePoint::Extended {
            toward: "middle_01_r",
            length: HAND_LENGTH,
        },
        bucket_bones: &[
            "hand_r",
            "index_01_r",
            "middle_01_r",
            "pinky_01_r",
            "ring_01_r",
            "thumb_01_r",
        ],
        parent: Some(RagdollBodyPart::RightForearm),
        joint: Some(JointSpec::Spherical {
            angular_damping: WRIST_ANGULAR_DAMPING,
        }),
    },
    LimbSpec {
        part: RagdollBodyPart::LeftThigh,
        name: "left_thigh",
        shape: Shape::Capsule,
        start: BonePoint::Joint("thigh_l"),
        end: BonePoint::Joint("calf_l"),
        bucket_bones: &["thigh_l"],
        parent: Some(RagdollBodyPart::Torso),
        joint: Some(JointSpec::Spherical {
            angular_damping: HIP_ANGULAR_DAMPING,
        }),
    },
    LimbSpec {
        part: RagdollBodyPart::LeftCalf,
        name: "left_calf",
        shape: Shape::Capsule,
        start: BonePoint::Joint("calf_l"),
        end: BonePoint::Joint("foot_l"),
        bucket_bones: &["calf_l"],
        parent: Some(RagdollBodyPart::LeftThigh),
        joint: Some(JointSpec::Revolute {
            angle_limits: (-0.1, 2.6),
        }),
    },
    LimbSpec {
        part: RagdollBodyPart::LeftFoot,
        name: "left_foot",
        shape: Shape::Cuboid,
        start: BonePoint::Joint("foot_l"),
        end: BonePoint::Extended {
            toward: "ball_l",
            length: FOOT_LENGTH,
        },
        bucket_bones: &["foot_l", "ball_l"],
        parent: Some(RagdollBodyPart::LeftCalf),
        joint: Some(JointSpec::Spherical {
            angular_damping: ANKLE_ANGULAR_DAMPING,
        }),
    },
    LimbSpec {
        part: RagdollBodyPart::RightThigh,
        name: "right_thigh",
        shape: Shape::Capsule,
        start: BonePoint::Joint("thigh_r"),
        end: BonePoint::Joint("calf_r"),
        bucket_bones: &["thigh_r"],
        parent: Some(RagdollBodyPart::Torso),
        joint: Some(JointSpec::Spherical {
            angular_damping: HIP_ANGULAR_DAMPING,
        }),
    },
    LimbSpec {
        part: RagdollBodyPart::RightCalf,
        name: "right_calf",
        shape: Shape::Capsule,
        start: BonePoint::Joint("calf_r"),
        end: BonePoint::Joint("foot_r"),
        bucket_bones: &["calf_r"],
        parent: Some(RagdollBodyPart::RightThigh),
        joint: Some(JointSpec::Revolute {
            angle_limits: (-0.1, 2.6),
        }),
    },
    LimbSpec {
        part: RagdollBodyPart::RightFoot,
        name: "right_foot",
        shape: Shape::Cuboid,
        start: BonePoint::Joint("foot_r"),
        end: BonePoint::Extended {
            toward: "ball_r",
            length: FOOT_LENGTH,
        },
        bucket_bones: &["foot_r", "ball_r"],
        parent: Some(RagdollBodyPart::RightCalf),
        joint: Some(JointSpec::Spherical {
            angular_damping: ANKLE_ANGULAR_DAMPING,
        }),
    },
];

fn get_bone_position(name: &str, bones: &BoneMap, transforms: &Query<&GlobalTransform>) -> Option<Vec3> {
    let entity = bones.get(name)?;
    transforms.get(entity).ok().map(|transform| transform.translation())
}

fn measure_point(
    point: BonePoint,
    start_position: Vec3,
    bones: &BoneMap,
    transforms: &Query<&GlobalTransform>,
) -> Option<Vec3> {
    match point {
        BonePoint::Joint(name) => get_bone_position(name, bones, transforms),
        BonePoint::Raised { bone, local_offset } => {
            let head_entity = bones.get(bone)?;
            let head_transform = transforms.get(head_entity).ok()?;
            Some(head_transform.translation() + head_transform.rotation() * local_offset)
        }
        BonePoint::Extended { toward, length } => {
            let toward_position = get_bone_position(toward, bones, transforms)?;
            let direction = (toward_position - start_position).try_normalize()?;
            Some(start_position + direction * length)
        }
    }
}

fn measure_limb(
    spec: &LimbSpec,
    bones: &BoneMap,
    transforms: &Query<&GlobalTransform>,
) -> Option<(Transform, MeasuredLimb)> {
    let start = measure_point(spec.start, Vec3::ZERO, bones, transforms)?;
    let end = measure_point(spec.end, start, bones, transforms)?;
    let segment = Segment::create_between(start, end)?;
    Some((segment.transform(), MeasuredLimb { anchor: start, segment }))
}

#[derive(Clone, Copy)]
struct MeasuredLimb {
    anchor: Vec3,
    segment: Segment,
}

struct SkinnedVertices {
    world_positions: Vec<Vec3>,
    dominant_bones: Vec<Box<str>>,
}

#[derive(SystemParam)]
struct RagdollSceneContext<'w, 's> {
    transforms: Query<'w, 's, &'static GlobalTransform>,
    parents: Query<'w, 's, &'static ChildOf>,
    children: Query<'w, 's, &'static Children>,
    skinned_meshes: Query<'w, 's, (&'static SkinnedMesh, &'static Mesh3d)>,
    mesh_assets: Res<'w, Assets<Mesh>>,
    names: Query<'w, 's, &'static Name>,
}

fn collect_skinned_vertices(bones: &BoneMap, scene: &RagdollSceneContext) -> Option<SkinnedVertices> {
    let skeleton_root = bones.get("root")?;
    let armature = scene.parents.get(skeleton_root).ok()?.parent();
    let armature_global = scene.transforms.get(armature).ok()?.compute_transform();
    let mut world_positions = Vec::new();
    let mut dominant_bones = Vec::new();
    for entity in scene.children.iter_descendants(armature) {
        let Ok((skinned_mesh, mesh_handle)) = scene.skinned_meshes.get(entity) else {
            continue;
        };
        let Some(mesh) = scene.mesh_assets.get(&mesh_handle.0) else {
            continue;
        };
        let Some(vertex_positions) = mesh_attribute_positions(mesh) else {
            continue;
        };
        let Some(joint_indices) = mesh_attribute_joint_indices(mesh) else {
            continue;
        };
        let Some(joint_weights) = mesh_attribute_joint_weights(mesh) else {
            continue;
        };
        for (vertex_index, &vertex_position) in vertex_positions.iter().enumerate() {
            let mut strongest_influence: Option<(u16, f32)> = None;
            for (&joint_index, weight) in joint_indices[vertex_index].iter().zip(joint_weights[vertex_index]) {
                let beats_strongest = strongest_influence.is_none_or(|(_, strongest_weight)| weight > strongest_weight);
                if beats_strongest {
                    strongest_influence = Some((joint_index, weight));
                }
            }
            let Some((joint_index, weight)) = strongest_influence else {
                continue;
            };
            if weight < DOMINANT_WEIGHT_THRESHOLD {
                continue;
            }
            let Some(&joint_entity) = skinned_mesh.joints.get(joint_index as usize) else {
                continue;
            };
            let Ok(joint_name) = scene.names.get(joint_entity) else {
                continue;
            };
            world_positions.push(armature_global * Vec3::from_array(vertex_position));
            dominant_bones.push(joint_name.as_str().into());
        }
    }
    (!world_positions.is_empty()).then_some(SkinnedVertices {
        world_positions,
        dominant_bones,
    })
}

fn mesh_attribute_positions(mesh: &Mesh) -> Option<&Vec<[f32; 3]>> {
    match mesh.try_attribute(Mesh::ATTRIBUTE_POSITION) {
        Ok(VertexAttributeValues::Float32x3(values)) => Some(values),
        _ => None,
    }
}

fn mesh_attribute_joint_indices(mesh: &Mesh) -> Option<&Vec<[u16; 4]>> {
    match mesh.try_attribute(Mesh::ATTRIBUTE_JOINT_INDEX) {
        Ok(VertexAttributeValues::Uint16x4(values)) => Some(values),
        _ => None,
    }
}

fn mesh_attribute_joint_weights(mesh: &Mesh) -> Option<&Vec<[f32; 4]>> {
    match mesh.try_attribute(Mesh::ATTRIBUTE_JOINT_WEIGHT) {
        Ok(VertexAttributeValues::Float32x4(values)) => Some(values),
        _ => None,
    }
}

fn fit_limb_collider(spec: &LimbSpec, vertices: &SkinnedVertices, segment: Segment) -> Collider {
    let axis_direction = segment.rotation * Vec3::Y;
    let mut along_axis = Vec::new();
    let mut perpendicular_distances = Vec::new();
    for (&vertex_position, dominant_bone) in vertices.world_positions.iter().zip(&vertices.dominant_bones) {
        if !spec.bucket_bones.contains(&&**dominant_bone) {
            continue;
        }
        let offset = vertex_position - segment.start;
        along_axis.push(offset.dot(axis_direction));
        perpendicular_distances.push(offset.reject_from(axis_direction).length());
    }
    if along_axis.is_empty() {
        return fallback_limb_collider(spec, segment);
    }
    let perpendicular_radius =
        percentile(&mut perpendicular_distances, COLLIDER_FIT_PERCENTILE).max(MINIMUM_COLLIDER_RADIUS);
    match spec.shape {
        Shape::Capsule => {
            let cylinder_half_height = (segment.length * 0.5 - perpendicular_radius).max(0.01);
            Collider::capsule(perpendicular_radius, cylinder_half_height * 2.0)
        }
        Shape::Cuboid => {
            let lowest_along_axis = percentile(&mut along_axis, 0.05);
            let highest_along_axis = percentile(&mut along_axis, 0.95);
            let cross_section_radius =
                percentile(&mut perpendicular_distances, CUBOID_CROSS_SECTION_PERCENTILE).max(MINIMUM_COLLIDER_RADIUS);
            let half_length = highest_along_axis
                .abs()
                .max(lowest_along_axis.abs())
                .min(segment.length * 0.5)
                .max(MINIMUM_COLLIDER_RADIUS);
            Collider::cuboid(
                cross_section_radius * 2.0,
                half_length * 2.0,
                cross_section_radius * 2.0,
            )
        }
    }
}

fn fallback_limb_collider(spec: &LimbSpec, segment: Segment) -> Collider {
    let fallback_radius = match spec.part {
        RagdollBodyPart::Torso => 0.24,
        RagdollBodyPart::Head => 0.16,
        RagdollBodyPart::LeftUpperArm | RagdollBodyPart::RightUpperArm => 0.06,
        RagdollBodyPart::LeftForearm | RagdollBodyPart::RightForearm => 0.05,
        RagdollBodyPart::LeftHand | RagdollBodyPart::RightHand => 0.06,
        RagdollBodyPart::LeftThigh | RagdollBodyPart::RightThigh => 0.08,
        RagdollBodyPart::LeftCalf | RagdollBodyPart::RightCalf => 0.06,
        RagdollBodyPart::LeftFoot | RagdollBodyPart::RightFoot => 0.035,
    };
    match spec.shape {
        Shape::Capsule => Collider::capsule(fallback_radius, segment.length),
        Shape::Cuboid => Collider::cuboid(fallback_radius * 2.0, segment.length, fallback_radius * 2.0),
    }
}

fn percentile(values: &mut [f32], fraction: f32) -> f32 {
    values.sort_by(f32::total_cmp);
    let index = ((values.len() as f32 * fraction).floor() as usize).min(values.len() - 1);
    values[index]
}

fn spawn_spherical_joint(
    commands: &mut Commands,
    enemy: Entity,
    parent: Entity,
    child: Entity,
    anchor: Vec3,
    angular_damping: f32,
) -> Entity {
    commands
        .spawn((
            OwnedByEnemy(enemy),
            SphericalJoint::new(parent, child).with_anchor(anchor),
            JointDamping {
                linear: 0.0,
                angular: angular_damping,
            },
            JointCollisionDisabled,
            JointAnchoredToBody(child),
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
            OwnedByEnemy(bodies.enemy),
            JointAnchoredToBody(bodies.child),
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
    let joint_x_axis = (parent_axis - compute_hinge_axis * parent_axis.dot(compute_hinge_axis))
        .try_normalize()
        .unwrap_or(Vec3::X);
    let joint_y_axis = compute_hinge_axis.cross(joint_x_axis);
    let world_joint_basis = Quat::from_mat3(&Mat3::from_cols(joint_x_axis, joint_y_axis, compute_hinge_axis));
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
    enemy: Entity,
    spec: &LimbSpec,
    transform: Transform,
    fitted: Collider,
) -> Entity {
    commands
        .spawn((
            Name::new(spec.name),
            OwnedByEnemy(enemy),
            spec.part,
            RigidBody::Dynamic,
            AngularDamping(ANGULAR_DAMPING),
            LinearDamping(LINEAR_DAMPING),
            SleepThreshold {
                linear: 0.15,
                angular: RAGDOLL_ANGULAR_SLEEP_THRESHOLD,
            },
            ColliderDensity(COLLIDER_DENSITY),
            CollisionLayers::new(RAGDOLL_GROUP, WORLD_GROUP),
            Friction::new(0.1).with_combine_rule(CoefficientCombine::Min),
            transform,
            fitted,
        ))
        .id()
}

fn compute_hinge_axis(limbs: &HashMap<RagdollBodyPart, MeasuredLimb>) -> Vec3 {
    let left_shoulder = limbs[&RagdollBodyPart::LeftUpperArm].anchor;
    let right_shoulder = limbs[&RagdollBodyPart::RightUpperArm].anchor;
    let shoulder_axis = (left_shoulder - right_shoulder).try_normalize().unwrap_or(Vec3::X);
    let torso_axis = limbs[&RagdollBodyPart::Torso].segment.rotation * Vec3::Y;
    shoulder_axis.cross(torso_axis).try_normalize().unwrap_or(Vec3::Z)
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
    ),
>;

fn spawn_ragdoll_bodies(mut commands: Commands, pending: PendingRagdollQuery<'_, '_>, scene: RagdollSceneContext) {
    for (root, bones, _, dying, already_spawned) in &pending {
        if already_spawned {
            commands.entity(root).remove::<PendingRagdoll>();
            continue;
        }

        let missing_bone = LIMBS
            .iter()
            .flat_map(LimbSpec::list_bone_names)
            .find(|name| bones.get(name).is_none());
        if let Some(name) = missing_bone {
            warn!("Cannot create ragdoll for {root:?}: missing bone {name}");
            commands.entity(root).remove::<PendingRagdoll>();
            continue;
        }

        let Some(skinned_vertices) = collect_skinned_vertices(bones, &scene) else {
            warn!("Cannot create ragdoll for {root:?}: no skinned mesh vertices");
            commands.entity(root).remove::<PendingRagdoll>();
            continue;
        };

        let mut limbs: HashMap<RagdollBodyPart, MeasuredLimb> = HashMap::new();
        let mut part_entities: HashMap<RagdollBodyPart, Entity> = HashMap::new();
        let mut parts: Vec<RigPart> = Vec::new();

        for spec in LIMBS {
            let Some((transform, measured)) = measure_limb(spec, bones, &scene.transforms) else {
                warn!("Cannot create ragdoll for {root:?}: cannot measure limb {}", spec.name);
                break;
            };

            let collider = fit_limb_collider(spec, &skinned_vertices, measured.segment);
            let entity = spawn_body(&mut commands, root, spec, transform, collider);
            if !dying {
                commands.entity(entity).insert(RigidBodyDisabled);
            }
            part_entities.insert(spec.part, entity);
            limbs.insert(spec.part, measured);
            parts.push(RigPart {
                entity,
                body_part: spec.part,
                initial: GlobalTransform::from(transform),
                bones: Vec::new(),
            });
        }

        if parts.len() < LIMBS.len() {
            commands.entity(root).remove::<PendingRagdoll>();
            continue;
        }

        if dying {
            spawn_limb_joints(&mut commands, root, &part_entities, &limbs);
        }

        let torso_entity = part_entities[&RagdollBodyPart::Torso];
        let mut bones_by_part = assign_drivers(bones, part_entities, &scene.parents, &scene.transforms, torso_entity);

        for part in parts.iter_mut() {
            part.bones = bones_by_part.remove(&part.entity).unwrap_or_default();
        }

        commands.entity(root).insert(RagdollData {
            bones: bones.clone(),
            limbs,
            parts,
        });
        commands.entity(root).remove::<PendingRagdoll>();
    }
}

fn spawn_limb_joints(
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

fn spawn_joint(
    commands: &mut Commands,
    endpoints: JointEndpoints,
    specification: JointSpec,
    limbs: &HashMap<RagdollBodyPart, MeasuredLimb>,
) {
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
            spawn_spherical_joint(
                commands,
                root,
                parent_entity,
                child_entity,
                child_measured.anchor,
                angular_damping,
            );
        }
        JointSpec::Revolute { angle_limits } => {
            spawn_revolute_joint(
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
            );
        }
    }
}

fn assign_drivers(
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

        let mut measured_limbs: HashMap<RagdollBodyPart, MeasuredLimb> = HashMap::new();
        let mut limb_transforms: HashMap<RagdollBodyPart, Transform> = HashMap::new();
        for spec in LIMBS {
            let Some((transform, measured)) = measure_limb(spec, &ragdoll.bones, &bone_transforms) else {
                continue;
            };
            limb_transforms.insert(spec.part, transform);
            measured_limbs.insert(spec.part, measured);
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
}

fn activate_ragdoll_on_death(
    mut commands: Commands,
    pending: Query<(Entity, &PendingRagdollImpactRequest, &RagdollData)>,
) {
    for (root, impact, ragdoll) in &pending {
        if ragdoll.limbs.len() != LIMBS.len() {
            continue;
        }

        let part_entities = ragdoll
            .parts
            .iter()
            .map(|part| (part.body_part, part.entity))
            .collect::<HashMap<_, _>>();
        for part in &ragdoll.parts {
            commands.entity(part.entity).remove::<RigidBodyDisabled>();
        }
        spawn_limb_joints(&mut commands, root, &part_entities, &ragdoll.limbs);

        let Some(&body) = part_entities.get(&impact.body_part) else {
            continue;
        };
        commands.entity(body).insert(PendingRagdollImpact {
            impulse: impact.impulse,
            point: impact.point,
        });
        commands.entity(root).remove::<PendingRagdollImpactRequest>();
    }
}

fn apply_ragdoll_pose(
    ragdolls: Query<&RagdollData, With<Dying>>,
    positions: Query<(&Position, &Rotation)>,
    mut bone_globals: Query<&mut GlobalTransform>,
) {
    for ragdoll in &ragdolls {
        for part in &ragdoll.parts {
            let Ok((pos, rot)) = positions.get(part.entity) else {
                continue;
            };

            let current = GlobalTransform::from(Transform::from_translation(pos.f32()).with_rotation(rot.f32()));
            let delta = current.affine() * part.initial.affine().inverse();

            for &(bone_entity, bone_initial) in &part.bones {
                if let Ok(mut global) = bone_globals.get_mut(bone_entity) {
                    *global = GlobalTransform::from(Mat4::from(delta * bone_initial.affine()));
                }
            }
        }
    }
}
