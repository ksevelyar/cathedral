# Intro
## Sandfield
* The player starts in the desert before the tower entrance, so the tower is fully in frame.
* The sandfield extends at least 300m from its center.
* Low dunes give the terrain an irregular wind-shaped surface.
* The level takes place at night beneath a dark blue sky.
* A visible moon and cold moonlight illuminate the sandfield and tower.

## Tower
* A 54 m high tapered concrete tower stands above the sandfield.
* The tower is 36 m wide at its base and 28 m wide beneath its roof.
* A visible entrance opens at ground level.
* A square central well runs through the tower.
* Four wide concrete flights ascend from the entrance to the top hangar.
* Four wide concrete flights descend from the entrance into the tower foundation.
* Flights follow the tower's inner contour without crossing its walls.
* The well exposes lower flights and long firing lines.
* Every flight has stepped treads and a smooth monolithic underside.
* Turns have full concrete floor landings.
* Concrete parapets form an unbroken line around the well and its turns.
* The fourth ascending flight opens directly onto the hangar floor.
* The top floor is a hangar beneath a closed roof with 8 m of clear height.
* No parapet rises above the hangar floor at the stair opening.
* Cold lights stand on stair parapets.

## Underground entrances
* Four lower stair landings have corridor-shaped tower cutouts named `Level -1`, `Level -2`, `Level -3`, and `Level -4`.
* Each cutout spans exactly the beveled-edge segment and is 4 m high.
* Each `Level -N` marker defines the beveled face, with corridor structure only outside the tower.
* The passage floor meets each landing and corridor floor continuously without an extra connector slab.
* `assets/models/entrance_to_underground_level.py` generates the reusable underground entrance asset.
* The entrance asset has a 5 m corridor with 4 m clear height and clear width matching a beveled tower edge, leading to a concrete room with a 15 m by 15 m clear interior.
* Its `Entrance connector` marker is at the center of the corridor start.
* Entrances join tower cutouts through matching open-face markers without mesh overlap at the interface.
* Each level can later select a distinct entrance asset that provides the same centered connector marker.
