# Screenshots
## Command
* `cargo screenshot` captures the default `map01` scenario.
* `cargo screenshot scenario=map01` captures map01 from the player's eyes at player start.
* `cargo screenshot scenario=gunner-colliders` captures one gunner on the inspection map.
* `cargo screenshot scenario=gunners-ragdoll` captures two gunners as one is shot.
* Add `--colliders=true` to show collider and ragdoll gizmos for any scenario.

## Output
* Output is `screenshots/<game-version>-<scenario>.png`, overwritten on each run.
* Map01 renders at 3440x1440. The gunner scenarios render at 1440x1440.

## CI
* Tests run in `nix develop .#ci-vulkan`, which renders via lavapipe (mesa software Vulkan) because GitHub runners have no GPU.
* Local runs use the default shell and the real GPU.
