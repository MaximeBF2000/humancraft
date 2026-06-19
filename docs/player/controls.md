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
- Main menu Settings: open the shortcuts screen and click an action row, then
  press a key to change that shortcut. Shortcut changes are saved and reused
  next time the game starts.

In-game controls:

- `Z`: move forward
- `S`: move backward
- `Q`: move left
- `D`: move right
- Double-tap `Z`: sprint
- Hold `Shift`: sneak
- `Space`: jump
- Mouse: look around
- Hold left click: damage the targeted block until its material strength is
  exhausted. Softer blocks break quickly; stone and ores take longer. The
  selected tool changes break speed, and blocks that need a tool only drop loot
  when the selected tool meets the harvest requirement. Releasing the button or
  aiming at another block resets the current damage.
- Right click or hold right click: place the selected hotbar item when it is a
  placeable block. Right clicking a crafting table, chest, or furnace opens
  its UI instead. Hold `Shift` while right clicking one of those utility blocks
  to place against it instead of opening it.
- Left/Right arrows: change the selected hotbar slot
- `E`: open or close the inventory overlay
- `Esc`: open the pause overlay. `Keep Playing` resumes. `Save & Quit` flushes
  player state, inventory, and edited chunks to disk, then returns to the main
  menu. The pause overlay also has Settings for changing shortcuts.

The hotbar is always visible at the bottom of the gameplay view. It shows the
first nine inventory slots and item stack counts. One slot is selected at a
time. Empty hand renders the player arm; selecting a placeable block renders a
larger lower-right 3D block in hand, while other selected items render as
angled item sprites. Pressing `E` opens the full 36-slot inventory overlay and
releases the cursor; pressing `E` or `Esc` closes it and recaptures mouse-look.

The inventory overlay includes a 2 x 2 crafting grid. Put one oak log in any
crafting slot to produce four oak planks. Put oak planks in all four 2 x 2
crafting slots to craft a crafting table. Put two oak planks vertically to
craft four sticks. Right click a placed crafting table to open the 3 x 3
crafting table grid. The 3 x 3 grid crafts wooden, stone, iron, and diamond
pickaxes, shovels, and axes with the original Minecraft-style tool shapes:
pickaxes use three material items over two centered sticks, shovels use one
material item over two sticks, and axes use three material items around two
sticks in either left or right orientation. Iron ore now drops raw iron, which
is smelted into iron ingots by the furnace system. Crafting inputs are returned
to the player inventory when the UI closes, space permitting.

Additional 3 x 3 crafting recipes:

- Chest: oak planks around the edge with the center empty.
- Furnace: cobblestone around the edge with the center empty.
- Wooden stairs: six oak planks in a stair shape, producing four stairs.
- Wooden slabs: three oak planks in a horizontal row, producing six slabs.

Inventory and crafting overlay controls:

- Left click a stack: pick up the whole stack.
- Left click with a carried stack: place, merge, or swap the stack.
- Right click a stack: pick up half of it, rounding up.
- Right click with a carried stack: place one item into an empty or matching
  slot.
- Shift-left click a player inventory or hotbar stack: quick-transfer it
  between the hotbar and the main inventory, filling matching stacks first.
- Shift-left click while a chest is open: move stacks between the player
  inventory and chest storage.
- Shift-left click while a furnace is open: move fuel into the fuel slot,
  smeltable items into the input slot, or move furnace contents back to the
  player inventory.
- Left click a crafting result: take the result and consume one set of
  ingredients from the crafting grid.
- Shift-left click a crafting result: craft as many results as possible into
  the player inventory.
- Left drag with a carried stack: distribute items evenly over compatible
  player, crafting, chest, or furnace slots. Slot counts update while dragging.
- Right drag with a carried stack: place one item in each compatible player,
  crafting, chest, or furnace slot.
- Double-left click while carrying a stack: collect matching visible stacks into
  the carried stack up to the item's stack limit.
- Number keys `1` through `9` while hovering a slot: swap that slot with the
  matching hotbar slot.
- `Q`: drop one item from the carried stack, or from the hovered slot if the
  cursor is empty.
- `Ctrl+Q`: drop the full carried stack or hovered slot stack.
- Hovering an item shows its display name in a tooltip.
- Clicking outside the inventory while carrying a stack drops it near the
  player with a small forward toss.
- Press the drop shortcut during gameplay to drop one item from the selected
  hotbar slot, or hold `Ctrl` with it to drop the full selected stack.

Breaking a block drops its configured loot into the world as a Y-axis rotating
item entity. Walk close to the dropped item to pick it up. The item is added to
the first compatible inventory stack, up to the current 64-item stack limit,
then to the first empty slot. Stone currently drops cobblestone, and the
cobblestone stack can be selected and placed as a block. Stone, cobblestone,
sandstone, coal ore, iron ore, gold ore, and diamond ore require a sufficient
pickaxe to drop loot. Diamond and gold ore require an iron pickaxe or better.

New placeable utility and shape blocks:

- Chests are full-block, static storage blocks. They face the player when
  placed and their inventory item does not stack. Breaking a chest keeps its
  contents inside the dropped chest item during the current play session;
  placing that chest item restores the contents.
- Furnaces are full-block utility blocks. They face the player when placed.
  Right click one to open its input, fuel, and output slots in a
  Minecraft-style furnace layout. Burning furnaces show a lit front texture,
  and the furnace UI shows fuel and cook progress. Smelting one item takes 200
  game ticks, or 10 seconds at the current 20 Hz tick rate.
- Placed chest and furnace inventories survive `Save & Quit` and world reloads.
  Furnace burn time and partially completed cook progress are saved as well.
- Glass is produced by smelting sand.
- Coal, oak planks, sticks, oak logs, wooden stairs, and wooden slabs are
  furnace fuels.
- Wooden stairs use oak plank textures and place as top or bottom stairs based
  on the clicked face or hit height. The stair rises in the player look
  direction. Stair orientation is saved with the chunk and survives reloading.
- Wooden slabs use oak plank textures. Clicking a top or bottom face places a
  horizontal slab; clicking a side face places a vertical slab against the
  clicked block. Opposite wooden slabs in the same cell merge into oak planks.
- Oak logs orient from the clicked face: top or bottom clicks make vertical
  logs, east or west clicks make east-west logs, and north or south clicks make
  north-south logs.
- Oak leaves generated as part of trees decay over time after their supporting
  logs are removed. Decaying or broken oak leaves have a 15% chance to drop oak
  saplings. Oak leaves placed by the player do not decay.
- Oak saplings can be placed on soil. They render as crossed flat plant
  textures and grow into oak trees over time when there is enough open space
  above them.
- Dirt near grass can turn into grass over time when the dirt block has open
  space above it.
- Sand falls downward when unsupported, using a slower accelerating fall rather
  than dropping one full block every tick.

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
