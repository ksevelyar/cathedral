* don't use #[allow] to bypass clippy
* rust edition is 2024, no mod.rs allowed
* verify changes with `nix develop -c cargo test` and `nix develop -c cargo clippy --all-targets -- -D warnings`
