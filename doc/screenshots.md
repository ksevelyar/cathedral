# Screenshots

## Command
* `cargo screenshot` captures map01 from the player's eyes at player start.
* `cargo screenshot -- 0,1.9,6:0,2,-9` captures from one viewpoint.
* `cargo screenshot -- --colliders` captures one idle fighter on the collider inspection map with collider gizmos visible.
* `cargo screenshot -- --muzzle-flash` captures the revolver firing, including its flame and light.

## Output
* Path is `screenshots/<game-version>-map01.png`, overwritten on each run.
* Collider inspection output is `screenshots/<game-version>-collider-inspection.png`.
* Muzzle flash output is `screenshots/<game-version>-revolver-muzzle-flash.png`.
* Capture is headless with no window, rendering into an offscreen target at 3440x1440, collider inspection and muzzle flash at 1440x1440.

## CI
* Tests run in `nix develop .#ci-vulkan`, which renders via lavapipe (mesa software Vulkan) because GitHub runners have no GPU.
* Local runs use the default shell and the real GPU.
