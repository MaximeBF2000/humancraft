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
- Right click: place the selected hotbar item when it is a placeable block
- Left/Right arrows: change the selected hotbar slot
- `E`: open or close the inventory overlay
- `Esc`: open the pause overlay. `Keep Playing` resumes. `Save & Quit` flushes
  player state, inventory, and edited chunks to disk, then returns to the main
  menu.

The hotbar is always visible at the bottom of the gameplay view. It shows the
first nine inventory slots and item stack counts. One slot is selected at a
time. Empty hand renders the player arm; selecting a placeable block renders a
small 3D block in hand, while other selected items render as angled item
sprites. Pressing `E` opens the full 36-slot inventory overlay and releases the
cursor; pressing `E` or `Esc` closes it and recaptures mouse-look.

Inventory overlay controls:

- Left click a stack: pick up the whole stack.
- Left click with a carried stack: place, merge, or swap the stack.
- Right click a stack: pick up half of it, rounding up.
- Right click with a carried stack: place one item into an empty or matching
  slot.
- Left drag with a carried stack: distribute items evenly over compatible
  slots.
- Right drag with a carried stack: place one item in each compatible slot.

Breaking a block drops its configured loot into the world as a Y-axis rotating
item entity. Walk close to the dropped item to pick it up. The item is added to
the first compatible inventory stack, up to the current 64-item stack limit,
then to the first empty slot. Stone currently drops cobblestone, and the
cobblestone stack can be selected and placed as a block.

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
