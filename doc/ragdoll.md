# Ragdoll
## Contract
* while alive, animation drives the bones and the mesh; physics bodies exist only as invisible teleporting ghosts so bullet hit detection has colliders to ray against
* every limb body collides only with the world, never with other limbs
* while alive, joints and bodies are inert: they never simulate, collide with each other, or resist animation; they wake up only at death
* on death, the impulse is capped so the hit body's velocity change never exceeds MAX_HIT_SPEED, computed from the body's mass
* at death, joints are rebuilt from the re-measured animated pose: static angle limits come from the LIMBS specification, anchors come from the current pose
* after death, physics drives the bones: each bone transform is the composition of its body's transform with the part's initial transform recorded at spawn

## Scenario: gunner shot in the head
### Spawn the gunner
* enemies::spawn_enemy() creates the gunner entity holding async asset requests
* `setup_ragdoll` and `prepare_enemy_animation` run on `WorldInstanceReady`
* `kill_enemy_on_hit` runs on `EnemyHit`

### Collect bones
* ragdoll::setup_ragdoll() collects every named descendant of the scene root into the BoneMap component
* the PendingRagdoll marker is inserted to enemy

### Spawn limb bodies
* ragdoll::spawn_ragdoll_bodies() consumes PendingRagdoll and measures 14 limbs (ragdoll::LIMBS via ragdoll::measure_limb) from current bone transforms
* limbs: torso capsule, head sphere, upper arms, forearms, hands, thighs, calves, feet
* each limb spawns as a dynamic rigid body (ragdoll::spawn_body)
* measurements are stored in the RagdollData component

### Build joints
* ragdoll::spawn_joint() anchors limbs together
* spherical joints (ragdoll::spawn_spherical_joint) at neck, shoulders, wrists, hips, ankles
* revolute joints with angle limits (ragdoll::spawn_revolute_joint) at elbows and knees
* JointDisabled on every joint and RigidBodyDisabled on every body keep them inert

### Assign drivers
* ragdoll::assign_drivers() assigns every skeleton bone to the nearest limb body
* the mapping is stored in RigPart.bones, so the mesh can later follow physics

### Set up animation and weapon
* enemies::setup_external_animation() builds the animation graph (idle/moving/attack) on the armature
* enemies::attach_weapon() parents the pistol to the hand_r bone

### Animate and keep colliders aligned
* enemies::enemy_behavior() moves the gunner with obstacle avoidance (Fighter/Gunner::update switch animation states)
* ragdoll::synchronize_ragdoll_bodies() teleports limb bodies every frame to follow the animated bones

### Detect the hit
* shooting::shoot() casts a ray against RAGDOLL_GROUP colliders and triggers EnemyHit on the enemy
* the kill_enemy_on_hit observer from the first step fires

### Start dying
* kill_enemy_on_hit inserts the Dying marker
* ragdoll::drop_weapon detaches the pistol as a free rigid body
* animations stop

### Activate the ragdoll
* ragdoll::synchronize_ragdoll_bodies() re-measures all limbs from the animated pose into RagdollData.limbs on the death frame
* ragdoll::activate_ragdoll_on_death() rebuilds joints from RagdollData.limbs, removes JointDisabled and RigidBodyDisabled
* ragdoll::wake_ragdoll_bodies_on_hit inserts PendingRagdollImpactRequest, activate_ragdoll_on_death converts it to PendingRagdollImpact with the mass-capped impulse
* ragdoll::apply_pending_impacts applies the impulse to the hit body at the impact point

### Drive the mesh from physics
* ragdoll::apply_ragdoll_pose() recomputes bone transforms from the physics bodies each frame
* the mesh falls with the ragdoll and the head kicks back from the impulse

### Settle and advance
* damping and sleep thresholds bring the bodies to rest; the mesh keeps following them
* maps::advance_map() despawns the settled ragdoll and loads the next map
