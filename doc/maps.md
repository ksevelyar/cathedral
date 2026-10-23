# Maps
## Contract
* A map defines its environment, player initial position, enemies, and objects.
* Every object implements `MapObject`.

## Object workflow
### Imported
1. Generate a low-poly initial `.blend` model with a Python script.
2. Open the model in Blender and add details.
3. Export the model and load it in the game.
4. Create static colliders from its meshes.

Examples: tower, laptop.

### Runtime generated
1. Define named parameters for the object.
2. Generate its render geometry and materials in game code.
3. Create collision appropriate to its physics behavior.
4. Update generated geometry when its source state changes.

Examples: sandfield, hanging lamp.

## Levels
* [Intro](maps/intro.md)
