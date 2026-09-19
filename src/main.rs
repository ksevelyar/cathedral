use bevy::log::{BoxedLayer, LogPlugin};
use bevy::prelude::*;
use cathedral::GamePlugin;
use std::fs::File;
use std::sync::Arc;

fn log_file_layer() -> Option<BoxedLayer> {
    let file = File::create("last-run.log").ok()?;
    Some(Box::new(
        bevy::log::tracing_subscriber::fmt::layer()
            .with_writer(Arc::new(file))
            .with_ansi(false),
    ))
}

fn main() {
    App::new()
        .add_plugins((
            DefaultPlugins.set(LogPlugin {
                custom_layer: |_| log_file_layer(),
                ..default()
            }),
            GamePlugin,
        ))
        .run();
}
