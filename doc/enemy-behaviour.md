# Enemy behaviour
## Requirements
* Enemies patrol a cyclic route through points placed on the map, waiting between patrol points.
* Each enemy has a vision cone and a hearing radius.
* An enemy detects a continuously visible player after N seconds.
* Gunshots within hearing range draw an enemy to their source. The enemy turns toward the sound while moving there, then resumes patrolling on arrival if it has not engaged the player.
* Players can hide from enemies behind level geometry and objects.
* Damage to an enemy immediately switches it to the engaged state.

## User Scenarios
* An enemy follows its patrol route when the player is out of sight and no audible gunshot interrupts it, pausing for three to five seconds between patrol points.
* A fighter detects a player after two seconds of continuous visibility; a gunner takes four seconds. Losing visibility before detection resets progress.
* A player outside the vision cone, beyond vision range, or hidden behind level geometry is not detected.
* An enemy that hears a gunshot investigates its source. A louder or newer equally loud shot can replace its current investigation target.
* An enemy resumes its patrol route on reaching a gunshot's source if it has not engaged the player.
* An enemy that detects or is damaged by the player remains engaged after losing sight.
* A gunner turns toward the player while engaged.

## Implementation
* `EnemyBehavior` represents patrol, investigation, and engaged states.
* Patrol destinations cycle through the enemy's configured map points. Enemies wait for a randomized three to five seconds between points.
* Enemy senses define vision range, vision cone, detection duration, and hearing range.
* Vision range and facing are checked in the XZ plane; vertical angle is unlimited. These checks run before raycasting.
* Vision rays start at the enemy's ragdoll head and test player body positions at the head and pelvis. A player is visible if at least one ray reaches a player-owned body without hitting level geometry.
* Gunshots are handled through weapon-fired messages. Ranged weapon loudness scales the enemy's hearing range; when multiple shots are audible, the loudest by loudness relative to distance is selected.
* Enemies move toward investigation targets and face their movement direction. On reaching the target, they immediately resume patrolling unless they have engaged the player.
* Ranged weapons have a shot loudness value alongside their damage and magazine capacity. Louder shots can be heard from farther away.

