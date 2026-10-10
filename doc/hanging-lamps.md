# Hanging lamps
## Contract
* A lamp is a heavy loft style shade hanging on a thick power cable from a ceiling mount.
* The cable is a chain of capsule rigid bodies linked by spherical joints, 10 links per meter so the visual wire mesh bends smoothly.
* The shade is a dynamic body jointed to the last cable link.
* A shot on the shade or the cable makes the whole lamp swing around the ceiling mount like a pendulum.

## Physics
* The pendulum period depends only on cable length and gravity: T = 2π√(L/g), independent of mass.
* For the 2.2 m cable of map01 the measured period is about 3 s.
* After a shot the lamp makes at least five decaying swings within 15 s and settles within 30 s.

## Scenario: a lamp is shot
### Spawn
* maps::pieces::spawn_lamp() spawns the static ceiling anchor, the cable link chain, the shade body, and the joints.
* The wire mesh entity (LampWire) follows the link transforms every frame.
* The shade carries a reflector, an emissive diffuser, and one shadow-casting downward spot light as children.

### Hit
* shooting::shoot() raycasts the shade collider and the link colliders.
* The hit body receives the bullet momentum as an impulse at the hit point.

### Swing and settle
* The lamp swings horizontally through the rest position with alternating sign and a period of about 3 s.
* Linear damping bleeds pendulum energy until the bodies sleep.
