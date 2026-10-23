# Screenshots
## Command
* `cargo screenshot` captures the first viewpoint of the default `intro` scenario.
* `cargo screenshot scenario=intro viewpoint=dunes` captures the Intro map's `dunes` viewpoint.
* Viewpoint names are the same labels used by in-game viewpoint navigation. If omitted, the first viewpoint for the scenario is selected.
* An unknown viewpoint reports the valid labels for that scenario.
* `cargo screenshot scenario=gunner-colliders` captures one gunner on the inspection map.
* `cargo screenshot scenario=gunners-ragdoll` captures two gunners as one is shot.
* Add `--colliders=true` to show collider and ragdoll gizmos for any scenario.

## Output
* Output is `screenshots/<game-version>-<scenario>-<viewpoint>.png`, overwritten on each run.
* Intro renders at 3440x1440. The gunner scenarios render at 1440x1440.

## CI
* Tests run in `nix develop .#ci-vulkan`, which renders via lavapipe (mesa software Vulkan) because GitHub runners have no GPU.
* Local runs use the default shell and the real GPU.
