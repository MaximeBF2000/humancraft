# Player Controls

This page documents the current windowed client controls and player movement
behavior.

## Controls

Startup opens the main menu. Click the center button or press `Enter` to open
world management.

World management controls:

- `Enter`: load the selected world, or start creating one if no worlds exist
- `N`: create a new world
- `R`: rename the selected world
- `Delete`: delete the selected world
- Arrow keys: change selected world
- New world config: edit world name and seed on the same screen. `Tab` switches
  fields, `Enter` creates the world, and a blank seed uses an automatic seed.

In-game controls:

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
- `Esc`: open the pause overlay. `Keep Playing` resumes. `Save & Quit` flushes
  player state and edited chunks to disk, then returns to the main menu.

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
