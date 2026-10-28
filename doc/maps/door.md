# Door
* `assets/models/door.py` generates `door.blend` and `door.glb` with a frame and panels.
* Host models own their doorway apertures and panel pockets.
* Each host exports a `Door mount` at its doorway. The reusable door's `Door connector` aligns to that mount.
* The 6 m high aperture has width `TOWER_BASE_CORNER_CUT * math.sqrt(2.0)`.
* Its lower two-thirds are vertical and its upper third is trapezoidal.
* Closed panels meet at center and use matching center-edge chamfers.
* The panels slide into their matching host pockets below 7 m player distance and close at or beyond 7 m.
