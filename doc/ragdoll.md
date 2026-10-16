# Ragdoll
## Contract
* A living enemy is driven by animation. The physics bodies exist only as colliders for bullet rays.
* A killing hit applies an impulse to the hit body part.
* After death, physics drives the bones.

```
pelvis
├─ thigh_l
│  └─ calf_l
│     └─ foot_l
├─ thigh_r
│  └─ calf_r
│     └─ foot_r
└─ spine_01
  └─ spine_02
    └─ spine_03
       ├─ neck_01
       │  └─ Head
       ├─ clavicle_l
       │  └─ upperarm_l
       │     └─ lowerarm_l
       │        └─ hand_l
       └─ clavicle_r
          └─ upperarm_r
             └─ lowerarm_r
                └─ hand_r
```
Each body part owns the skin whose strongest influence is its start joint (head, hands and feet own their whole joint subtree).
