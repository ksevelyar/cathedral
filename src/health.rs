use bevy::prelude::*;

use crate::ragdoll::RagdollBodyPart;

pub struct HealthPlugin;

impl Plugin for HealthPlugin {
    fn build(&self, app: &mut App) {
        app.add_message::<CharacterDamaged>()
            .add_message::<CharacterDied>()
            .add_systems(Update, apply_damage_to_health);
    }
}

#[derive(Component)]
pub struct Health {
    current: f32,
}

impl Health {
    pub fn new(starting_health: f32) -> Self {
        Self {
            current: starting_health,
        }
    }

    pub fn get_current(&self) -> f32 {
        self.current
    }
}

#[derive(Message)]
pub struct CharacterDamaged {
    pub character: Entity,
    pub amount: f32,
    pub attacker: Option<Entity>,
    pub body_part: Option<RagdollBodyPart>,
}

#[derive(Message)]
pub struct CharacterDied {
    pub character: Entity,
}

fn apply_damage_to_health(
    mut damaged: MessageReader<CharacterDamaged>,
    mut healths: Query<(&mut Health, Option<&Name>)>,
    names: Query<&Name>,
    mut died: MessageWriter<CharacterDied>,
) {
    for damage in damaged.read() {
        let Ok((mut health, victim_name)) = healths.get_mut(damage.character) else {
            continue;
        };
        health.current = (health.current - damage.amount).max(0.0);
        let attacker_name = damage
            .attacker
            .and_then(|attacker| names.get(attacker).ok())
            .map_or_else(|| "?".to_string(), ToString::to_string);
        let victim_name = victim_name.map_or_else(|| format!("{}", damage.character), ToString::to_string);
        info!(
            "{} {:?} damaged by {:?}, {} health left",
            victim_name, damage.body_part, attacker_name, health.current
        );
        if health.current == 0.0 {
            died.write(CharacterDied {
                character: damage.character,
            });
        }
    }
}
