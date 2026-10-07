# Screenshots

## Command
* `cargo screenshot` captures map01 from the player's eyes at player start.
* `cargo screenshot -- 0,1.9,6:0,2,-9` captures from one viewpoint.

## Output
* Path is `screenshots/<game-version>-map01.png`, overwritten on each run.
* Capture is headless with no window, rendering into an offscreen target at 3440x1440.

## CI
* Tests run in `nix develop .#ci-vulkan`, which renders via lavapipe (mesa software Vulkan) because GitHub runners have no GPU.
* Local runs use the default shell and the real GPU.
