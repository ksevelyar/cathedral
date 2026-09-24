# Maps
## Contract
* A map defines player start, enemies, pieces.
* Pieces are built from data, using primitives.

## Lifecycle
* spawn_map inserts PlayerStartPosition, spawns pieces, enemies.
* advance_map: when all enemies are Dying, despawn every Arena entity and every enemy, advance CurrentMap, spawn_map.

## Pieces
* A piece is a Transform plus items specified in its local space. An item is a cuboid or a light.
* Every cuboid item spawns a mesh, a static cuboid collider, component Arena. Every light item spawns a point light with shadows, component Arena.
* Spawning bakes the piece transform into item transforms, without entity hierarchy.
* Vocabulary: cuboid is the only geometric primitive, maps author floors, walls, blocks as explicit cuboid calls. Stair, door, wall torch are computed composites. Oriented pieces are laid out facing -Z.
