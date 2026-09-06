use bevy::prelude::*;

use super::arena;
use crate::enemies::{EnemyKind, Fighter, Gunner, spawn_enemy};

pub(super) fn load(
    commands: &mut Commands,
    asset_server: &AssetServer,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
) {
    arena(commands, meshes, materials);
    spawn_enemies(commands, asset_server);
}

pub(super) fn spawn_enemies(commands: &mut Commands, asset_server: &AssetServer) {
    for (translation, kind) in [
        (Vec3::new(-8.0, 0.0, -8.0), EnemyKind::Gunner(Gunner::default())),
        (Vec3::new(8.0, 0.0, -8.0), EnemyKind::Gunner(Gunner::default())),
        (Vec3::new(0.0, 0.0, -10.0), EnemyKind::Fighter(Fighter::default())),
    ] {
        spawn_enemy(
            commands,
            asset_server,
            kind,
            Transform::from_translation(translation),
            true,
        );
    }
}
