use avian3d::math::AsF32;
use avian3d::prelude::*;
use bevy::prelude::*;
use bevy::transform::TransformSystems;
use bevy::world_serialization::WorldInstanceReady;
use std::collections::HashMap;

use crate::enemies::Dying;

pub struct RagdollPlugin;

impl Plugin for RagdollPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
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

#[derive(Component)]
struct PendingRagdoll {
    bones: HashMap<String, Entity>,
}

struct RigPart {
    entity: Entity,
    body_part: RagdollBodyPart,
    initial: GlobalTransform,
    bones: Vec<(Entity, GlobalTransform)>,
}

#[derive(Component)]
struct RagdollData {
    bones: RagdollBones,
    parts: Vec<RigPart>,
}

#[derive(Component)]
#[relationship(relationship_target = EnemyPhysicsEntities)]
pub struct OwnedByEnemy(pub Entity);

#[derive(Component)]
#[relationship_target(relationship = OwnedByEnemy, linked_spawn)]
pub struct EnemyPhysicsEntities(Vec<Entity>);

#[derive(Component, Clone, Copy, Debug, PartialEq, Eq)]
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

pub fn setup_ragdoll(
    scene_ready: On<WorldInstanceReady>,
    mut commands: Commands,
    children: Query<&Children>,
    names: Query<&Name>,
) {
    let root = scene_ready.entity;

    let mut bones: HashMap<String, Entity> = HashMap::new();
    for descendant in children.iter_descendants(root) {
        if let Ok(name) = names.get(descendant) {
            bones.insert(name.as_str().to_string(), descendant);
        }
    }

    commands.entity(root).insert(PendingRagdoll { bones });
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

struct PhysicsPart {
    entity: Entity,
    body_part: RagdollBodyPart,
    initial: GlobalTransform,
}

fn spawn_spherical_joint(
    commands: &mut Commands,
    enemy: Entity,
    parent: Entity,
    child: Entity,
    anchor: Vec3,
    angular_damping: f32,
) {
    commands.spawn((
        OwnedByEnemy(enemy),
        SphericalJoint::new(parent, child).with_anchor(anchor),
        JointDamping {
            linear: 0.0,
            angular: angular_damping,
        },
        JointCollisionDisabled,
    ));
}

fn spawn_revolute_joint(
    commands: &mut Commands,
    bodies: JointBodies,
    parent_segment: Segment,
    child_segment: Segment,
    hinge_axis: Vec3,
    angle_limits: (f32, f32),
) {
    let parent_axis = parent_segment.rotation * Vec3::Y;
    let joint_x_axis = (parent_axis - hinge_axis * parent_axis.dot(hinge_axis))
        .try_normalize()
        .unwrap_or(Vec3::X);
    let joint_y_axis = hinge_axis.cross(joint_x_axis);
    let world_joint_basis = Quat::from_mat3(&Mat3::from_cols(joint_x_axis, joint_y_axis, hinge_axis));
    let anchor = child_segment.start;

    commands.spawn((
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
    ));
}

struct JointBodies {
    enemy: Entity,
    parent: Entity,
    child: Entity,
}

fn spawn_capsule(
    commands: &mut Commands,
    enemy: Entity,
    body_part: RagdollBodyPart,
    name: &'static str,
    segment: Segment,
    radius: f32,
) -> PhysicsPart {
    let entity = commands
        .spawn((
            Name::new(name),
            OwnedByEnemy(enemy),
            body_part,
            RigidBody::Dynamic,
            AngularDamping(ANGULAR_DAMPING),
            LinearDamping(LINEAR_DAMPING),
            SleepThreshold {
                linear: 0.15,
                angular: RAGDOLL_ANGULAR_SLEEP_THRESHOLD,
            },
            Collider::capsule(radius, segment.length - 2.0 * radius),
            ColliderDensity(1000.0),
            CollisionLayers::new(0b10, 0b01),
            Friction::new(0.1).with_combine_rule(CoefficientCombine::Min),
            segment.transform(),
        ))
        .id();
    PhysicsPart {
        entity,
        body_part,
        initial: GlobalTransform::from(segment.transform()),
    }
}

fn spawn_head(commands: &mut Commands, enemy: Entity, center: Vec3) -> PhysicsPart {
    let entity = commands
        .spawn((
            Name::new("head"),
            OwnedByEnemy(enemy),
            RagdollBodyPart::Head,
            RigidBody::Dynamic,
            AngularDamping(ANGULAR_DAMPING),
            LinearDamping(LINEAR_DAMPING),
            SleepThreshold {
                linear: 0.15,
                angular: RAGDOLL_ANGULAR_SLEEP_THRESHOLD,
            },
            Collider::sphere(HEAD_RADIUS),
            ColliderDensity(1000.0),
            CollisionLayers::new(0b10, 0b01),
            Friction::new(0.1).with_combine_rule(CoefficientCombine::Min),
            Restitution::ZERO,
            Transform::from_translation(center),
        ))
        .id();
    PhysicsPart {
        entity,
        body_part: RagdollBodyPart::Head,
        initial: GlobalTransform::from_translation(center),
    }
}

#[derive(Clone, Copy)]
struct ArmBones {
    shoulder: Entity,
    elbow: Entity,
    wrist: Entity,
    finger: Entity,
}

#[derive(Clone, Copy)]
struct LegBones {
    hip: Entity,
    knee: Entity,
    ankle: Entity,
    ball: Entity,
}

#[derive(Clone, Copy)]
struct RagdollBones {
    pelvis: Entity,
    spine_tip: Entity,
    neck: Entity,
    head: Entity,
    left_arm: ArmBones,
    right_arm: ArmBones,
    left_leg: LegBones,
    right_leg: LegBones,
}

impl RagdollBones {
    fn resolve(bones: &HashMap<String, Entity>) -> Result<Self, &'static str> {
        let resolve = |name: &'static str| bones.get(name).copied().ok_or(name);

        Ok(Self {
            pelvis: resolve("pelvis")?,
            spine_tip: bones
                .get("spine_03")
                .or_else(|| bones.get("spine_02"))
                .copied()
                .ok_or("spine_03")?,
            neck: resolve("neck_01")?,
            head: resolve("Head")?,
            left_arm: ArmBones {
                shoulder: resolve("upperarm_l")?,
                elbow: resolve("lowerarm_l")?,
                wrist: resolve("hand_l")?,
                finger: resolve("middle_01_l")?,
            },
            right_arm: ArmBones {
                shoulder: resolve("upperarm_r")?,
                elbow: resolve("lowerarm_r")?,
                wrist: resolve("hand_r")?,
                finger: resolve("middle_01_r")?,
            },
            left_leg: LegBones {
                hip: resolve("thigh_l")?,
                knee: resolve("calf_l")?,
                ankle: resolve("foot_l")?,
                ball: resolve("ball_l")?,
            },
            right_leg: LegBones {
                hip: resolve("thigh_r")?,
                knee: resolve("calf_r")?,
                ankle: resolve("foot_r")?,
                ball: resolve("ball_r")?,
            },
        })
    }
}

struct ArmPose {
    upper_arm: Segment,
    forearm: Segment,
    hand: Segment,
}

impl ArmPose {
    fn read(bones: &ArmBones, transforms: &Query<&GlobalTransform>) -> Option<Self> {
        let shoulder = transforms.get(bones.shoulder).ok()?.translation();
        let elbow = transforms.get(bones.elbow).ok()?.translation();
        let wrist = transforms.get(bones.wrist).ok()?.translation();
        let finger = transforms.get(bones.finger).ok()?.translation();

        let upper_arm = Segment::between(shoulder, elbow)?;
        let forearm = Segment::between(elbow, wrist)?;
        let hand_forward = (finger - wrist).try_normalize()?;
        let hand = Segment::between(wrist, wrist + hand_forward * HAND_LENGTH)?;

        Some(Self {
            upper_arm,
            forearm,
            hand,
        })
    }
}

struct LegPose {
    thigh: Segment,
    calf: Segment,
    foot: Segment,
}

impl LegPose {
    fn read(bones: &LegBones, transforms: &Query<&GlobalTransform>) -> Option<Self> {
        let hip = transforms.get(bones.hip).ok()?.translation();
        let knee = transforms.get(bones.knee).ok()?.translation();
        let ankle = transforms.get(bones.ankle).ok()?.translation();
        let ball = transforms.get(bones.ball).ok()?.translation();

        let thigh = Segment::between(hip, knee)?;
        let calf = Segment::between(knee, ankle)?;
        let foot_forward = (ball - ankle).try_normalize()?;
        let foot = Segment::between(ankle, ankle + foot_forward * FOOT_LENGTH)?;

        Some(Self { thigh, calf, foot })
    }
}

struct RagdollPose {
    torso: Segment,
    neck: Vec3,
    head_center: Vec3,
    left_arm: ArmPose,
    right_arm: ArmPose,
    left_leg: LegPose,
    right_leg: LegPose,
}

impl RagdollPose {
    fn read(bones: &RagdollBones, transforms: &Query<&GlobalTransform>) -> Option<Self> {
        let pelvis = transforms.get(bones.pelvis).ok()?.translation();
        let spine_tip = transforms.get(bones.spine_tip).ok()?.translation();
        let neck = transforms.get(bones.neck).ok()?.translation();
        let head = transforms.get(bones.head).ok()?.translation();

        let torso = Segment::between(pelvis, spine_tip)?;
        let head_center = head + Vec3::Y * HEAD_OFFSET;

        Some(Self {
            torso,
            neck,
            head_center,
            left_arm: ArmPose::read(&bones.left_arm, transforms)?,
            right_arm: ArmPose::read(&bones.right_arm, transforms)?,
            left_leg: LegPose::read(&bones.left_leg, transforms)?,
            right_leg: LegPose::read(&bones.right_leg, transforms)?,
        })
    }

    fn body_transform(&self, body_part: RagdollBodyPart) -> Transform {
        match body_part {
            RagdollBodyPart::Torso => self.torso.transform(),
            RagdollBodyPart::Head => Transform::from_translation(self.head_center),
            RagdollBodyPart::LeftUpperArm => self.left_arm.upper_arm.transform(),
            RagdollBodyPart::LeftForearm => self.left_arm.forearm.transform(),
            RagdollBodyPart::LeftHand => self.left_arm.hand.transform(),
            RagdollBodyPart::RightUpperArm => self.right_arm.upper_arm.transform(),
            RagdollBodyPart::RightForearm => self.right_arm.forearm.transform(),
            RagdollBodyPart::RightHand => self.right_arm.hand.transform(),
            RagdollBodyPart::LeftThigh => self.left_leg.thigh.transform(),
            RagdollBodyPart::LeftCalf => self.left_leg.calf.transform(),
            RagdollBodyPart::LeftFoot => self.left_leg.foot.transform(),
            RagdollBodyPart::RightThigh => self.right_leg.thigh.transform(),
            RagdollBodyPart::RightCalf => self.right_leg.calf.transform(),
            RagdollBodyPart::RightFoot => self.right_leg.foot.transform(),
        }
    }
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
    pending: Query<(Entity, &PendingRagdoll, Has<Dying>)>,
    transforms: Query<&GlobalTransform>,
    parents: Query<&ChildOf>,
) {
    for (root, ragdoll, dying) in &pending {
        let bones = match RagdollBones::resolve(&ragdoll.bones) {
            Ok(bones) => bones,
            Err(missing) => {
                warn!("Cannot create ragdoll for {root:?}: missing bone {missing}");
                commands.entity(root).remove::<PendingRagdoll>();
                continue;
            }
        };

        let Some(pose) = RagdollPose::read(&bones, &transforms) else {
            warn!("Cannot create ragdoll for {root:?}: failed to read bone pose");
            commands.entity(root).remove::<PendingRagdoll>();
            continue;
        };

        let torso = spawn_capsule(
            &mut commands,
            root,
            RagdollBodyPart::Torso,
            "torso",
            pose.torso,
            TORSO_RADIUS,
        );
        let upper_arm = spawn_capsule(
            &mut commands,
            root,
            RagdollBodyPart::LeftUpperArm,
            "left_upper_arm",
            pose.left_arm.upper_arm,
            UPPER_ARM_RADIUS,
        );
        spawn_spherical_joint(
            &mut commands,
            root,
            torso.entity,
            upper_arm.entity,
            pose.left_arm.upper_arm.start,
            JOINT_ANGULAR_DAMPING,
        );

        let forearm = spawn_capsule(
            &mut commands,
            root,
            RagdollBodyPart::LeftForearm,
            "left_forearm",
            pose.left_arm.forearm,
            FOREARM_RADIUS,
        );
        let shoulder_axis = (pose.left_arm.upper_arm.start - pose.right_arm.upper_arm.start)
            .try_normalize()
            .unwrap_or(Vec3::X);
        let torso_axis = pose.torso.rotation * Vec3::Y;
        let hinge_axis = shoulder_axis.cross(torso_axis).try_normalize().unwrap_or(Vec3::Z);
        spawn_revolute_joint(
            &mut commands,
            JointBodies {
                enemy: root,
                parent: upper_arm.entity,
                child: forearm.entity,
            },
            pose.left_arm.upper_arm,
            pose.left_arm.forearm,
            hinge_axis,
            (-2.6, 0.1),
        );

        let hand = spawn_capsule(
            &mut commands,
            root,
            RagdollBodyPart::LeftHand,
            "left_hand",
            pose.left_arm.hand,
            HAND_RADIUS,
        );
        spawn_spherical_joint(
            &mut commands,
            root,
            forearm.entity,
            hand.entity,
            pose.left_arm.hand.start,
            WRIST_ANGULAR_DAMPING,
        );

        let right_upper_arm = spawn_capsule(
            &mut commands,
            root,
            RagdollBodyPart::RightUpperArm,
            "right_upper_arm",
            pose.right_arm.upper_arm,
            UPPER_ARM_RADIUS,
        );
        spawn_spherical_joint(
            &mut commands,
            root,
            torso.entity,
            right_upper_arm.entity,
            pose.right_arm.upper_arm.start,
            JOINT_ANGULAR_DAMPING,
        );

        let right_forearm = spawn_capsule(
            &mut commands,
            root,
            RagdollBodyPart::RightForearm,
            "right_forearm",
            pose.right_arm.forearm,
            FOREARM_RADIUS,
        );
        spawn_revolute_joint(
            &mut commands,
            JointBodies {
                enemy: root,
                parent: right_upper_arm.entity,
                child: right_forearm.entity,
            },
            pose.right_arm.upper_arm,
            pose.right_arm.forearm,
            hinge_axis,
            (-0.1, 2.6),
        );

        let right_hand = spawn_capsule(
            &mut commands,
            root,
            RagdollBodyPart::RightHand,
            "right_hand",
            pose.right_arm.hand,
            HAND_RADIUS,
        );
        spawn_spherical_joint(
            &mut commands,
            root,
            right_forearm.entity,
            right_hand.entity,
            pose.right_arm.hand.start,
            WRIST_ANGULAR_DAMPING,
        );

        let head = spawn_head(&mut commands, root, pose.head_center);
        spawn_spherical_joint(
            &mut commands,
            root,
            torso.entity,
            head.entity,
            pose.neck,
            NECK_ANGULAR_DAMPING,
        );

        let left_thigh = spawn_capsule(
            &mut commands,
            root,
            RagdollBodyPart::LeftThigh,
            "left_thigh",
            pose.left_leg.thigh,
            THIGH_RADIUS,
        );
        spawn_spherical_joint(
            &mut commands,
            root,
            torso.entity,
            left_thigh.entity,
            pose.left_leg.thigh.start,
            HIP_ANGULAR_DAMPING,
        );

        let left_calf = spawn_capsule(
            &mut commands,
            root,
            RagdollBodyPart::LeftCalf,
            "left_calf",
            pose.left_leg.calf,
            CALF_RADIUS,
        );
        spawn_revolute_joint(
            &mut commands,
            JointBodies {
                enemy: root,
                parent: left_thigh.entity,
                child: left_calf.entity,
            },
            pose.left_leg.thigh,
            pose.left_leg.calf,
            shoulder_axis,
            (-0.1, 2.6),
        );

        let left_foot = spawn_capsule(
            &mut commands,
            root,
            RagdollBodyPart::LeftFoot,
            "left_foot",
            pose.left_leg.foot,
            FOOT_RADIUS,
        );
        spawn_spherical_joint(
            &mut commands,
            root,
            left_calf.entity,
            left_foot.entity,
            pose.left_leg.foot.start,
            ANKLE_ANGULAR_DAMPING,
        );

        let right_thigh = spawn_capsule(
            &mut commands,
            root,
            RagdollBodyPart::RightThigh,
            "right_thigh",
            pose.right_leg.thigh,
            THIGH_RADIUS,
        );
        spawn_spherical_joint(
            &mut commands,
            root,
            torso.entity,
            right_thigh.entity,
            pose.right_leg.thigh.start,
            HIP_ANGULAR_DAMPING,
        );

        let right_calf = spawn_capsule(
            &mut commands,
            root,
            RagdollBodyPart::RightCalf,
            "right_calf",
            pose.right_leg.calf,
            CALF_RADIUS,
        );
        spawn_revolute_joint(
            &mut commands,
            JointBodies {
                enemy: root,
                parent: right_thigh.entity,
                child: right_calf.entity,
            },
            pose.right_leg.thigh,
            pose.right_leg.calf,
            shoulder_axis,
            (-0.1, 2.6),
        );

        let right_foot = spawn_capsule(
            &mut commands,
            root,
            RagdollBodyPart::RightFoot,
            "right_foot",
            pose.right_leg.foot,
            FOOT_RADIUS,
        );
        spawn_spherical_joint(
            &mut commands,
            root,
            right_calf.entity,
            right_foot.entity,
            pose.right_leg.foot.start,
            ANKLE_ANGULAR_DAMPING,
        );

        let drivers = HashMap::from([
            (bones.neck, head.entity),
            (bones.head, head.entity),
            (bones.left_leg.hip, left_thigh.entity),
            (bones.left_leg.knee, left_calf.entity),
            (bones.left_leg.ankle, left_foot.entity),
            (bones.left_leg.ball, left_foot.entity),
            (bones.right_leg.hip, right_thigh.entity),
            (bones.right_leg.knee, right_calf.entity),
            (bones.right_leg.ankle, right_foot.entity),
            (bones.right_leg.ball, right_foot.entity),
            (bones.left_arm.shoulder, upper_arm.entity),
            (bones.left_arm.elbow, forearm.entity),
            (bones.left_arm.wrist, hand.entity),
            (bones.right_arm.shoulder, right_upper_arm.entity),
            (bones.right_arm.elbow, right_forearm.entity),
            (bones.right_arm.wrist, right_hand.entity),
        ]);
        let mut bones_by_part: HashMap<Entity, Vec<(Entity, GlobalTransform)>> = HashMap::new();
        for &entity in ragdoll.bones.values() {
            let Ok(transform) = transforms.get(entity).copied() else {
                continue;
            };
            let part = nearest_driver(entity, &drivers, &parents, torso.entity);
            bones_by_part.entry(part).or_default().push((entity, transform));
        }

        let physics_parts = [
            torso,
            upper_arm,
            forearm,
            hand,
            right_upper_arm,
            right_forearm,
            right_hand,
            head,
            left_thigh,
            left_calf,
            left_foot,
            right_thigh,
            right_calf,
            right_foot,
        ];
        if !dying {
            for part in &physics_parts {
                commands.entity(part.entity).insert(RigidBodyDisabled);
            }
        }
        let parts = physics_parts
            .map(|part| RigPart {
                entity: part.entity,
                body_part: part.body_part,
                initial: part.initial,
                bones: bones_by_part.remove(&part.entity).unwrap_or_default(),
            })
            .into();

        commands.entity(root).insert(RagdollData { bones, parts });
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
        let Some(pose) = RagdollPose::read(&ragdoll.bones, &bone_transforms) else {
            continue;
        };

        for part in &ragdoll.parts {
            let Ok((mut position, mut rotation, mut transform)) = body_transforms.get_mut(part.entity) else {
                continue;
            };
            let body_transform = pose.body_transform(part.body_part);
            position.0 = body_transform.translation;
            rotation.0 = body_transform.rotation;
            *transform = body_transform;
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
