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
