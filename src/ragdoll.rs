use crate::enemies::{Dying, EnemyHit};
use avian3d::math::AsF32;
use avian3d::prelude::*;
use bevy::prelude::*;
use bevy::transform::TransformSystems;
use bevy::world_serialization::WorldInstanceReady;
use std::collections::HashMap;

pub(crate) const RAGDOLL_GROUP: u32 = 0b10;
pub(crate) const WORLD_GROUP: u32 = 0b01;

pub struct RagdollPlugin;

impl Plugin for RagdollPlugin {
    fn build(&self, app: &mut App) {
        app.add_observer(wake_ragdoll_bodies_on_hit)
            .add_systems(
                PostUpdate,
                (spawn_ragdoll_bodies, synchronize_ragdoll_bodies, apply_ragdoll_pose)
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

const TORSO_RADIUS: f32 = 0.18;
const HEAD_RADIUS: f32 = 0.12;
const HEAD_OFFSET: f32 = 0.06;

const UPPER_ARM_RADIUS: f32 = 0.06;
const FOREARM_RADIUS: f32 = 0.05;
const HAND_RADIUS: f32 = 0.06;
const HAND_LENGTH: f32 = 0.24;

const THIGH_RADIUS: f32 = 0.08;
const CALF_RADIUS: f32 = 0.06;
const FOOT_RADIUS: f32 = 0.035;
const FOOT_LENGTH: f32 = 0.24;

#[derive(Component, Clone, Default)]
pub(crate) struct BoneMap(HashMap<Box<str>, Entity>);

impl BoneMap {
    pub(crate) fn get(&self, name: &str) -> Option<Entity> {
        self.0.get(name).copied()
    }

    pub(crate) fn entities(&self) -> impl Iterator<Item = Entity> + '_ {
        self.0.values().copied()
    }
}

#[derive(Component)]
struct PendingRagdoll;

#[derive(Component)]
struct RagdollData {
    bones: BoneMap,
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

fn apply_pending_impacts(mut commands: Commands, mut impacted_bodies: Query<(Entity, &PendingRagdollImpact, Forces)>) {
    for (entity, impact, mut forces) in &mut impacted_bodies {
        forces.apply_linear_impulse_at_point(impact.impulse, impact.point);
        commands.entity(entity).remove::<PendingRagdollImpact>();
    }
}

type OwnedJoint = (Entity, &'static OwnedByEnemy);
type OwnedJoints<'w, 's> = Query<'w, 's, OwnedJoint, Or<(With<SphericalJoint>, With<RevoluteJoint>)>>;

fn wake_ragdoll_bodies_on_hit(
    hit: On<EnemyHit>,
    mut commands: Commands,
    ragdoll_bodies: Query<(Entity, &OwnedByEnemy), With<RagdollBodyPart>>,
    ragdoll_joints: OwnedJoints,
) {
    for (body, owner) in &ragdoll_bodies {
        if owner.0 == hit.entity {
            commands.entity(body).remove::<RigidBodyDisabled>();
        }
    }

    for (joint, owner) in &ragdoll_joints {
        if owner.0 == hit.entity {
            commands.entity(joint).remove::<JointDisabled>();
        }
    }

    commands.entity(hit.body).insert(PendingRagdollImpact {
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
    fn between(start: Vec3, end: Vec3) -> Option<Self> {
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
    First(&'static [&'static str]),
    Raised { bone: &'static str, offset: Vec3 },
    Extended { toward: &'static str, length: f32 },
}

#[derive(Clone, Copy)]
enum Shape {
    Capsule { radius: f32 },
    Sphere { radius: f32 },
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
    parent: Option<RagdollBodyPart>,
    joint: Option<JointSpec>,
}

impl LimbSpec {
    fn bone_names(&self) -> Vec<&'static str> {
        let mut names = Vec::new();
        match self.start {
            BonePoint::Joint(name) => names.push(name),
            BonePoint::First(bone_names) => names.extend_from_slice(bone_names),
            BonePoint::Raised { bone, .. } => names.push(bone),
            BonePoint::Extended { toward, .. } => names.push(toward),
        }
        match self.end {
            BonePoint::Joint(name) => names.push(name),
            BonePoint::First(bone_names) => names.extend_from_slice(bone_names),
            BonePoint::Raised { bone, .. } => names.push(bone),
            BonePoint::Extended { toward, .. } => names.push(toward),
        }
        names
    }

    fn driven_bones(&self) -> Vec<&'static str> {
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
        shape: Shape::Capsule { radius: TORSO_RADIUS },
        start: BonePoint::Joint("pelvis"),
        end: BonePoint::First(&["spine_03", "spine_02"]),
        parent: None,
        joint: None,
    },
    LimbSpec {
        part: RagdollBodyPart::Head,
        name: "head",
        shape: Shape::Sphere { radius: HEAD_RADIUS },
        start: BonePoint::Joint("neck_01"),
        end: BonePoint::Raised {
            bone: "Head",
            offset: Vec3::new(0.0, HEAD_OFFSET, 0.0),
        },
        parent: Some(RagdollBodyPart::Torso),
        joint: Some(JointSpec::Spherical {
            angular_damping: NECK_ANGULAR_DAMPING,
        }),
    },
    LimbSpec {
        part: RagdollBodyPart::LeftUpperArm,
        name: "left_upper_arm",
        shape: Shape::Capsule {
            radius: UPPER_ARM_RADIUS,
        },
        start: BonePoint::Joint("upperarm_l"),
        end: BonePoint::Joint("lowerarm_l"),
        parent: Some(RagdollBodyPart::Torso),
        joint: Some(JointSpec::Spherical {
            angular_damping: JOINT_ANGULAR_DAMPING,
        }),
    },
    LimbSpec {
        part: RagdollBodyPart::RightUpperArm,
        name: "right_upper_arm",
        shape: Shape::Capsule {
            radius: UPPER_ARM_RADIUS,
        },
        start: BonePoint::Joint("upperarm_r"),
        end: BonePoint::Joint("lowerarm_r"),
        parent: Some(RagdollBodyPart::Torso),
        joint: Some(JointSpec::Spherical {
            angular_damping: JOINT_ANGULAR_DAMPING,
        }),
    },
    LimbSpec {
        part: RagdollBodyPart::LeftForearm,
        name: "left_forearm",
        shape: Shape::Capsule { radius: FOREARM_RADIUS },
        start: BonePoint::Joint("lowerarm_l"),
        end: BonePoint::Joint("hand_l"),
        parent: Some(RagdollBodyPart::LeftUpperArm),
        joint: Some(JointSpec::Revolute {
            angle_limits: (-2.6, 0.1),
        }),
    },
    LimbSpec {
        part: RagdollBodyPart::LeftHand,
        name: "left_hand",
        shape: Shape::Capsule { radius: HAND_RADIUS },
        start: BonePoint::Joint("hand_l"),
        end: BonePoint::Extended {
            toward: "middle_01_l",
            length: HAND_LENGTH,
        },
        parent: Some(RagdollBodyPart::LeftForearm),
        joint: Some(JointSpec::Spherical {
            angular_damping: WRIST_ANGULAR_DAMPING,
        }),
    },
    LimbSpec {
        part: RagdollBodyPart::RightForearm,
        name: "right_forearm",
        shape: Shape::Capsule { radius: FOREARM_RADIUS },
        start: BonePoint::Joint("lowerarm_r"),
        end: BonePoint::Joint("hand_r"),
        parent: Some(RagdollBodyPart::RightUpperArm),
        joint: Some(JointSpec::Revolute {
            angle_limits: (-0.1, 2.6),
        }),
    },
    LimbSpec {
        part: RagdollBodyPart::RightHand,
        name: "right_hand",
        shape: Shape::Capsule { radius: HAND_RADIUS },
        start: BonePoint::Joint("hand_r"),
        end: BonePoint::Extended {
            toward: "middle_01_r",
            length: HAND_LENGTH,
        },
        parent: Some(RagdollBodyPart::RightForearm),
        joint: Some(JointSpec::Spherical {
            angular_damping: WRIST_ANGULAR_DAMPING,
        }),
    },
    LimbSpec {
        part: RagdollBodyPart::LeftThigh,
        name: "left_thigh",
        shape: Shape::Capsule { radius: THIGH_RADIUS },
        start: BonePoint::Joint("thigh_l"),
        end: BonePoint::Joint("calf_l"),
        parent: Some(RagdollBodyPart::Torso),
        joint: Some(JointSpec::Spherical {
            angular_damping: HIP_ANGULAR_DAMPING,
        }),
    },
    LimbSpec {
        part: RagdollBodyPart::LeftCalf,
        name: "left_calf",
        shape: Shape::Capsule { radius: CALF_RADIUS },
        start: BonePoint::Joint("calf_l"),
        end: BonePoint::Joint("foot_l"),
        parent: Some(RagdollBodyPart::LeftThigh),
        joint: Some(JointSpec::Revolute {
            angle_limits: (-0.1, 2.6),
        }),
    },
    LimbSpec {
        part: RagdollBodyPart::LeftFoot,
        name: "left_foot",
        shape: Shape::Capsule { radius: FOOT_RADIUS },
        start: BonePoint::Joint("foot_l"),
        end: BonePoint::Extended {
            toward: "ball_l",
            length: FOOT_LENGTH,
        },
        parent: Some(RagdollBodyPart::LeftCalf),
        joint: Some(JointSpec::Spherical {
            angular_damping: ANKLE_ANGULAR_DAMPING,
        }),
    },
    LimbSpec {
        part: RagdollBodyPart::RightThigh,
        name: "right_thigh",
        shape: Shape::Capsule { radius: THIGH_RADIUS },
        start: BonePoint::Joint("thigh_r"),
        end: BonePoint::Joint("calf_r"),
        parent: Some(RagdollBodyPart::Torso),
        joint: Some(JointSpec::Spherical {
            angular_damping: HIP_ANGULAR_DAMPING,
        }),
    },
    LimbSpec {
        part: RagdollBodyPart::RightCalf,
        name: "right_calf",
        shape: Shape::Capsule { radius: CALF_RADIUS },
        start: BonePoint::Joint("calf_r"),
        end: BonePoint::Joint("foot_r"),
        parent: Some(RagdollBodyPart::RightThigh),
        joint: Some(JointSpec::Revolute {
            angle_limits: (-0.1, 2.6),
        }),
    },
    LimbSpec {
        part: RagdollBodyPart::RightFoot,
        name: "right_foot",
        shape: Shape::Capsule { radius: FOOT_RADIUS },
        start: BonePoint::Joint("foot_r"),
        end: BonePoint::Extended {
            toward: "ball_r",
            length: FOOT_LENGTH,
        },
        parent: Some(RagdollBodyPart::RightCalf),
        joint: Some(JointSpec::Spherical {
            angular_damping: ANKLE_ANGULAR_DAMPING,
        }),
    },
];

fn bone_position(name: &str, bones: &BoneMap, transforms: &Query<&GlobalTransform>) -> Option<Vec3> {
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
        BonePoint::Joint(name) => bone_position(name, bones, transforms),
        BonePoint::First(bone_names) => bone_names
            .iter()
            .find_map(|name| bone_position(name, bones, transforms)),
        BonePoint::Raised { bone, offset } => Some(bone_position(bone, bones, transforms)? + offset),
        BonePoint::Extended { toward, length } => {
            let toward_position = bone_position(toward, bones, transforms)?;
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
    let segment = match spec.shape {
        Shape::Capsule { .. } => Some(Segment::between(start, end)?),
        Shape::Sphere { .. } => None,
    };
    let transform = segment
        .map(Segment::transform)
        .unwrap_or_else(|| Transform::from_translation(end));
    Some((transform, MeasuredLimb { anchor: start, segment }))
}

#[derive(Clone, Copy)]
struct MeasuredLimb {
    anchor: Vec3,
    segment: Option<Segment>,
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
    hinge_axis: Vec3,
    angle_limits: (f32, f32),
) -> Entity {
    let parent_axis = parent_segment.rotation * Vec3::Y;
    let joint_x_axis = (parent_axis - hinge_axis * parent_axis.dot(hinge_axis))
        .try_normalize()
        .unwrap_or(Vec3::X);
    let joint_y_axis = hinge_axis.cross(joint_x_axis);
    let world_joint_basis = Quat::from_mat3(&Mat3::from_cols(joint_x_axis, joint_y_axis, hinge_axis));
    let anchor = child_segment.start;

    commands
        .spawn((
            OwnedByEnemy(bodies.enemy),
            RevoluteJoint::new(bodies.parent, bodies.child)
                .with_local_anchor1(parent_segment.rotation.inverse() * (anchor - parent_segment.midpoint))
                .with_local_anchor2(child_segment.rotation.inverse() * (anchor - child_segment.midpoint))
                .with_local_basis1(parent_segment.rotation.inverse() * world_joint_basis)
                .with_local_basis2(child_segment.rotation.inverse() * world_joint_basis)
                .with_angle_limits(angle_limits.0, angle_limits.1),
            JointDamping {
                linear: 0.0,
                angular: JOINT_ANGULAR_DAMPING,
            },
            JointCollisionDisabled,
        ))
        .id()
}

fn spawn_body(
    commands: &mut Commands,
    enemy: Entity,
    spec: &LimbSpec,
    transform: Transform,
    segment: Option<Segment>,
) -> Entity {
    let collider = match (spec.shape, segment) {
        (Shape::Capsule { radius }, Some(segment)) => Collider::capsule(radius, segment.length - 2.0 * radius),
        (Shape::Sphere { radius }, _) => Collider::sphere(radius),
        _ => unreachable!("capsule limbs always have a measured segment"),
    };

    let mut body = commands.spawn((
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
        collider,
        ColliderDensity(1000.0),
        CollisionLayers::new(RAGDOLL_GROUP, WORLD_GROUP),
        Friction::new(0.1).with_combine_rule(CoefficientCombine::Min),
        transform,
    ));

    if matches!(spec.shape, Shape::Sphere { .. }) {
        body.insert(Restitution::ZERO);
    }

    body.id()
}

fn hinge_axis(limbs: &HashMap<RagdollBodyPart, MeasuredLimb>) -> Vec3 {
    let left_shoulder = limbs[&RagdollBodyPart::LeftUpperArm].anchor;
    let right_shoulder = limbs[&RagdollBodyPart::RightUpperArm].anchor;
    let shoulder_axis = (left_shoulder - right_shoulder).try_normalize().unwrap_or(Vec3::X);
    let torso_axis = limbs[&RagdollBodyPart::Torso].segment.unwrap().rotation * Vec3::Y;
    shoulder_axis.cross(torso_axis).try_normalize().unwrap_or(Vec3::Z)
}

fn nearest_driver(
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

fn spawn_ragdoll_bodies(
    mut commands: Commands,
    pending: Query<(Entity, &BoneMap, &PendingRagdoll, Has<Dying>)>,
    transforms: Query<&GlobalTransform>,
    parents: Query<&ChildOf>,
) {
    for (root, bones, _, dying) in &pending {
        let missing_bone = LIMBS
            .iter()
            .flat_map(LimbSpec::bone_names)
            .find(|name| bones.get(name).is_none());
        if let Some(name) = missing_bone {
            warn!("Cannot create ragdoll for {root:?}: missing bone {name}");
            commands.entity(root).remove::<PendingRagdoll>();
            continue;
        }

        let mut limbs: HashMap<RagdollBodyPart, MeasuredLimb> = HashMap::new();
        let mut part_entities: HashMap<RagdollBodyPart, Entity> = HashMap::new();
        let mut parts: Vec<RigPart> = Vec::new();
        let mut joints: Vec<Entity> = Vec::new();

        for spec in LIMBS {
            let Ok((transform, measured)) = measure_limb(spec, bones, &transforms)
                .ok_or(spec.name)
                .inspect_err(|&name| warn!("Cannot create ragdoll for {root:?}: cannot measure limb {name}"))
            else {
                break;
            };

            let entity = spawn_body(&mut commands, root, spec, transform, measured.segment);

            if let (Some(parent_part), Some(joint)) = (spec.parent, spec.joint) {
                let parent_entity = part_entities[&parent_part];
                match joint {
                    JointSpec::Spherical { angular_damping } => {
                        let joint = spawn_spherical_joint(
                            &mut commands,
                            root,
                            parent_entity,
                            entity,
                            measured.anchor,
                            angular_damping,
                        );
                        joints.push(joint);
                    }
                    JointSpec::Revolute { angle_limits } => {
                        let joint = spawn_revolute_joint(
                            &mut commands,
                            JointBodies {
                                enemy: root,
                                parent: parent_entity,
                                child: entity,
                            },
                            limbs[&parent_part].segment.unwrap(),
                            measured.segment.unwrap(),
                            hinge_axis(&limbs),
                            angle_limits,
                        );
                        joints.push(joint);
                    }
                }
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

        let incomplete = parts.len() < LIMBS.len();
        if incomplete {
            commands.entity(root).remove::<PendingRagdoll>();
            continue;
        }

        let torso_entity = part_entities[&RagdollBodyPart::Torso];
        let mut drivers: HashMap<Entity, Entity> = HashMap::new();
        for spec in LIMBS {
            let part_entity = part_entities[&spec.part];
            for name in spec.driven_bones() {
                if let Some(bone_entity) = bones.get(name) {
                    drivers.insert(bone_entity, part_entity);
                }
            }
        }

        let mut bones_by_part: HashMap<Entity, Vec<(Entity, GlobalTransform)>> = HashMap::new();
        for bone_entity in bones.entities() {
            let Ok(transform) = transforms.get(bone_entity).copied() else {
                continue;
            };
            let part = nearest_driver(bone_entity, &drivers, &parents, torso_entity);
            bones_by_part.entry(part).or_default().push((bone_entity, transform));
        }

        if !dying {
            for joint in &joints {
                commands.entity(*joint).insert(JointDisabled);
            }
            for part in &parts {
                commands.entity(part.entity).insert(RigidBodyDisabled);
            }
        }
        for part in parts.iter_mut() {
            part.bones = bones_by_part.remove(&part.entity).unwrap_or_default();
        }

        commands.entity(root).insert(RagdollData {
            bones: bones.clone(),
            parts,
        });
        commands.entity(root).remove::<PendingRagdoll>();
    }
}

fn synchronize_ragdoll_bodies(
    ragdolls: Query<(&RagdollData, Option<Ref<Dying>>)>,
    bone_transforms: Query<&GlobalTransform>,
    mut body_transforms: Query<(&mut Position, &mut Rotation, &mut Transform), With<RagdollBodyPart>>,
) {
    for (ragdoll, dying) in &ragdolls {
        if dying.as_ref().is_some_and(|dying| !dying.is_added()) {
            continue;
        }

        let limb_transforms: HashMap<RagdollBodyPart, Transform> = LIMBS
            .iter()
            .filter_map(|spec| Some((spec.part, measure_limb(spec, &ragdoll.bones, &bone_transforms)?.0)))
            .collect();

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
