# Maps
## Contract
* A map defines player initial position, enemies, pieces.
* Pieces are built from data, using primitives.

## Lifecycle
* spawn_map inserts PlayerPosition, spawns pieces, enemies.
* advance_map: when all enemies are Dying, despawn every Arena entity and every enemy, advance CurrentMap, spawn_map.
