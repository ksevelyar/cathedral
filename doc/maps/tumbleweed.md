# Tumbleweeds
## Contract
* Tumbleweeds are visual decoration owned by a sandfield and do not exist on other maps.
* A sandfield configuration has a fixed seed, so initial tumbleweed variants and positions are repeatable.
* Tumbleweeds use several low-poly visual variants.
* Tumbleweeds have no colliders, rigid bodies, gameplay interactions, or navigation effects.
* Each tumbleweed rests on the sandfield height at its horizontal position.

## Wind
* The sandfield has one prevailing wind direction shared by dune generation and tumbleweed movement.
* The prevailing direction remains stable during play.
* Gusts vary speed and add bounded lateral drift without reversing the prevailing flow.

## Lifecycle
* Active tumbleweeds roll downwind and rotate according to distance travelled.
* A tumbleweed deactivates when it crosses the downwind sandfield boundary.
* An inactive tumbleweed may respawn at a randomized upwind perimeter position.
* A respawn position must be within the sandfield, outside terrain openings, and far enough from the player to hide its appearance.
* A tumbleweed remains inactive when no valid respawn position is available.

Plan:
 1. Update doc/maps/intro.md to link its sandfield section to doc/maps/tumbleweed.md. Keep doc/maps/tumbleweed.md as the contract for scope, wind, lifecycle, and player-safe respawning.

 2. Refactor src/maps/sandfield.rs so dune generation receives its prevailing wind direction from SandField configuration rather than defining it inside terrain_height:
     - add a default sandfield wind configuration with direction, base speed, gust amplitude, gust frequency, and lateral-drift limit
     - change terrain height sampling to use the configured direction
     - expose instance height sampling from SandField for map setup and tumbleweed placement
     - update the intro tower-base calculation and terrain tests to sample from the configured field

 3. Add tumbleweed configuration to SandField:
     - fixed decoration seed
     - instance count
     - model asset paths
     - scale and rolling-speed ranges
     - player-safe respawn distance
     - builder methods that preserve the existing SandField::create(...).with_opening(...) style

 4. Add private sandfield components in src/maps/sandfield.rs:
     - SandFieldEnvironment on the terrain root, containing the radius, openings, wind settings, and tumbleweed configuration
     - Tumbleweed on each tumbleweed scene root, containing deterministic instance seed, selected variant, gust phase, roll distance, respawn count, and active state

 5. Add three compact low-poly tumbleweed GLB variants under assets/models/tumbleweed/:
     - keep each model origin at its ground contact point
     - use dry brown, rough, non-emissive materials
     - include no colliders, rigid bodies, lights, cameras, or animation
     - retain source Blender assets alongside the exported models, following the existing imported-model workflow

 6. Change SandField::spawn to create the terrain root entity first, attaching its render mesh, static terrain collider, transform, and SandFieldEnvironment. Use a locally seeded RNG to create tumbleweed
    scene-root children of that entity:
     - choose a model variant, scale, yaw, rolling speed, and gust phase
     - sample positions in the field’s local space
     - reject positions outside the radius or in an opening
     - set local Y from the same configured terrain-height function
     - attach a Name such as Tumbleweed to support black-box feature tests

 7. Add a pure wind-velocity function that accepts wind settings, local horizontal position, elapsed time, and per-instance phase. It must preserve the configured prevailing direction while adding smooth speed
    gusts and bounded lateral drift. Make this function independent of ECS so future sand particles can use it unchanged.

 8. Register one normal Update system from MapsPlugin for tumbleweed movement:
     - query tumbleweed scene roots, their ChildOf relation, and the parent SandFieldEnvironment
     - move active tumbleweeds in local sandfield space using the pure wind function
     - rotate them from distance travelled to simulate rolling
     - resample local Y after movement
     - deactivate them after crossing the downwind boundary, without reversing or wrapping their direction

 9. In the same system, reactivate inactive tumbleweeds only through upwind respawning:
     - transform the player position into the owning sandfield’s local space
     - use each tumbleweed’s fixed seed and incremented respawn count to create deterministic candidate positions
     - reject candidates outside the radius, inside openings, and within the configured player-safe distance
     - leave the tumbleweed inactive when the bounded candidate search cannot find a safe position
     - never teleport an active tumbleweed or respawn one near the player

 10. Configure src/maps/intro.rs with a fixed tumbleweed seed, conservative instance count, and wind direction matching the dunes. Tune values from the existing tower, dunes, and iris-door viewpoints.

 11. Extend tests/features/intro_map.rs with feature tests that verify:
 - the deterministic initial tumbleweed count and variants
 - initial tumbleweeds are within the sandfield and outside the tower opening
 - advancing time moves them downwind, rolls them, and keeps them terrain-relative
 - an instance deactivates after a downwind exit
 - respawn candidates are upwind, valid, and outside the player-safe distance
 - recreating the app with the same map configuration produces equal initial tumbleweed transforms
