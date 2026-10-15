## Contract
* Bullets leave blood marks where they hit an enemy.
* Each mark is a unique small circular blood spray with a dark circular entry wound at its center.
* Wound size is calculated from bullet caliber. Each weapon defines one caliber.
* The spray is irregular, dense near the wound, and fades at its edge.
* Marks stack when bullets hit the same area.
* Marks remain until their enemy is removed.
* Marks are visible at every distance.
* Each enemy has at most 24 blood marks. A new mark replaces the oldest mark.

## Implementation
* Intersect the bullet ray with the enemy's posed skinned meshes.
* If no mesh intersects, log an error and create no mark.
* Build a circular surface patch centered on the ray hit.
* Preserve the source mesh skinning data so the patch follows animation.
* Use bullet caliber for the patch diameter.
* Use a deterministic seed to vary the spray.
* Draw a dark circular entry wound surrounded by an irregular red circular spray.
* Keep each mark as a child of its hit skinned mesh.
* Remove the oldest mark when an enemy exceeds 16 marks.
