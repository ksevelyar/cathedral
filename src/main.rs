use bevy::prelude::*;
use cathedral::GamePlugin;

fn main() {
    App::new()
        .add_plugins((DefaultPlugins, GamePlugin))
        .run();
}
