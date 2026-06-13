# Player Controls

This page documents the current windowed client controls and player movement
behavior.

## Controls

- `Z`: move forward
- `S`: move backward
- `Q`: move left
- `D`: move right
- Double-tap `Z`: sprint
- Hold `Shift`: sneak
- `Space`: jump
- Mouse: look around
- Left click: break the targeted block
- Right click: place dirt against the targeted block
- `Esc`: pause or resume, releasing or capturing the mouse cursor

## Movement

Movement uses Minecraft-style fixed-tick physics with HumanCraft-specific
playability tuning. Walking and sprinting build horizontal velocity through
acceleration instead of instantly setting speed. Ground friction slows the
player after each tick, while airborne movement uses lower acceleration so
jumps still allow steering without feeling weightless.

Jumping uses the Minecraft-style initial vertical impulse, gravity, and air
damping. A normal jump can clear and land on a one-block ledge from a short
run-up distance when the player keeps moving toward it.

Sneaking lowers the camera eye height, slows movement, and prevents the player
from walking off an unsupported block edge.

Sprinting is triggered by pressing `Z` twice quickly. Sprinting increases
movement acceleration, widens the field of view, and adds a forward horizontal
boost when jumping.
