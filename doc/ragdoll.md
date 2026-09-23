# Ragdoll
## Contract
* A living enemy is driven entirely by animation. The physics bodies exist only as colliders for bullet rays.
* A killing hit applies an impulse to the hit body part.
* After death, physics drives the bones.
* The limbs are a torso capsule, a head sphere, upper arms, forearms, hands, thighs, calves, and feet.

## Scenario: a gunner is shot in the head
### Spawn the gunner
* enemies::spawn_enemy() creates the gunner entity holding async asset requests
* ragdoll::setup_ragdoll() builds the bones dictionary and marks the enemy PendingRagdoll on WorldInstanceReady
* enemies::setup_external_animation() builds the animation graph (idle/moving/attack) on the armature
* enemies::attach_weapon() parents the pistol to the hand_r bone

### Spawn the limbs
* ragdoll::spawn_ragdoll_bodies() spawns one dynamic body per limb from the current pose and stores the parts and measurements in RagdollData
* while the enemy is alive, the bodies carry RigidBodyDisabled
* ragdoll::assign_drivers() maps each skeleton bone to its limb body
* ragdoll::synchronize_ragdoll_bodies() moves the limb bodies every frame to follow the animated bones

### Hit
* shooting::shoot() raycasts the enemy colliders and triggers EnemyHit on the enemy, carrying the hit body, the impulse and the hit point
* ragdoll::wake_ragdoll_bodies_on_hit() resolves the hit body to its RagdollBodyPart and stores a PendingRagdollImpactRequest on the enemy

### Death
* enemies::kill_enemy_on_hit() marks the enemy Dying, stops all animations, and detaches the weapon as a free rigid body
* on the death frame, ragdoll::synchronize_ragdoll_bodies() re-measures the limbs from the death pose into RagdollData.limbs
* ragdoll::activate_ragdoll_on_death() removes RigidBodyDisabled from the limb bodies and builds the joints from the recorded pose: spherical at the neck, shoulders, wrists, hips and ankles, and revolute with angle limits at the elbows and knees

### Impact and settle
* ragdoll::activate_ragdoll_on_death() converts PendingRagdollImpactRequest into PendingRagdollImpact on the hit body with the impulse capped by the part's mass
* ragdoll::apply_pending_impacts() applies the impulse at the hit point before the physics step
* damping and sleep thresholds bring the bodies to rest
