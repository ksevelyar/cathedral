{
  inputs = {
    nixpkgs.url = "github:nixos/nixpkgs/nixos-26.05";

    rust-overlay.url = "github:oxalica/rust-overlay";
    rust-overlay.inputs.nixpkgs.follows = "nixpkgs";
  };

  outputs = {
    nixpkgs,
    rust-overlay,
    flake-utils,
    ...
  }:
    flake-utils.lib.eachDefaultSystem (
      system: let
        overlays = [(import rust-overlay)];
        pkgs = import nixpkgs {
          inherit system overlays;
        };
      in
        with pkgs; {
          devShells.default = mkShell {
            buildInputs = [
              # openssl
              alsa-lib
              vulkan-loader
              vulkan-tools
              mesa
              libGL
              libudev-zero
              wayland
              libxkbcommon

              # wasm
              # wasm-bindgen-cli
              # wasm-pack
              # binaryen

              pkg-config
              cargo-watch
              rust-analyzer

              jq
              python3
              blender
              (
                rust-bin.nightly.latest.default.override {
                  extensions = ["rust-src"];
                  targets = ["wasm32-unknown-unknown"];
                }
              )

              (writeShellScriptBin "ci" ''
                set -euo pipefail
                cargo fmt --all -- --check --color always
                cargo clippy --all-features --workspace -- -D warnings
                cargo test -- --nocapture
              '')
            ];

            LD_LIBRARY_PATH = lib.makeLibraryPath [
              alsa-lib
              vulkan-loader
              libudev-zero
              wayland
              libxkbcommon
              mesa
              libGL
            ];
          };

          devShells.ci-vulkan = mkShell {
            buildInputs = [alsa-lib vulkan-loader libudev-zero wayland libxkbcommon mesa libGL pkg-config];

            WGPU_BACKEND = "vulkan";
            VK_DRIVER_FILES = "${mesa}/share/vulkan/icd.d/lvp_icd.x86_64.json";
            LD_LIBRARY_PATH = lib.makeLibraryPath [
              alsa-lib
              vulkan-loader
              libudev-zero
              wayland
              libxkbcommon
              mesa
              libGL
            ];
          };
        }
    );
}
