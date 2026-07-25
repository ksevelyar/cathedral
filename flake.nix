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
              (
                rust-bin.nightly.latest.default.override {
                  extensions = ["rust-src"];
                  targets = ["wasm32-unknown-unknown"];
                }
              )
            ];

            LD_LIBRARY_PATH = lib.makeLibraryPath [
              alsa-lib
              vulkan-loader
              libudev-zero
              wayland
              libxkbcommon
            ];
          };
        }
    );
}
