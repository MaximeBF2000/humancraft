# Items, Inventory, And Loot

## Purpose

Items are registry-backed content definitions. Inventory and loot systems store
item IDs and stack counts, not concrete content types.

## Current Data Model

- `ItemDefinition` lives in `src/engine/world/item.rs`.
- `ItemStack` stores an `ItemId` plus a count.
- `Inventory` owns fixed slots and merges compatible stacks before using empty
  slots.
- Player inventory is currently 36 slots, with the first 9 slots rendered as
  the always-visible hotbar.
- `RenderState` owns transient UI state for a carried cursor stack, current
  drag operation, and selected hotbar index.
- `LootEntity` lives in `src/engine/world/loot.rs` and stores an item stack,
  world position, velocity, and rotation.

Item stack sizes come from item definitions. Current starter content uses the
Minecraft-style default of 64 items per stack.

## Block Drops

Blocks declare drops as item keys on `BlockDefinition::drops`. Windowed
gameplay resolves those keys through `ItemRegistry` when a block breaks and
spawns one dropped item entity per configured drop.

Keep this path data-driven:

- Do not hard-code per-block drop behavior in the windowed controller.
- Add or change drops on block definitions first.
- Keep tests that every block drop resolves to a registered item.
- If a dropped block resource should be placeable, register it as a block item
  with a `place_block` target. Stone uses this path by dropping placeable
  cobblestone.

## Windowed Gameplay

`src/app/windowed.rs` currently owns the temporary player inventory and loot
simulation until a fuller gameplay/entity layer exists.

Current behavior:

- Holding left click advances block breaking according to
  `BlockDefinition::hardness`. A block is replaced with air and spawns
  configured loot only after its current break progress reaches the required
  duration. Blocks tagged `unbreakable` never start break progress.
- Holding left or right click repeats block breaking or selected-hotbar block
  placement at explicit cadences. Left-click breaking is continuous
  hardness-based progress; right-click placement still uses the repeat cadence
  in `src/app/windowed/constants.rs`.
- Loot falls under gravity, damps against the ground, and rotates once per
  second around the world Y axis.
- Loot spawns within the broken block's newly opened space so a solid block
  directly above the break does not trap the drop in an ungatherable collision.
- Loot is picked up when the player is close enough and the inventory can
  accept the stack.
- `E` toggles the inventory overlay. The comparison is isolated through the
  default inventory binding helper so future settings can replace it.
- Inventory clicks follow the Minecraft survival baseline:
  - left click picks up, places, merges, or swaps whole stacks
  - right click splits a slot stack or places one carried item
  - left drag distributes a carried stack over compatible slots
  - right drag places one carried item in each compatible slot
- Left and right arrows move the selected hotbar slot.
- Right-click block placement only works when the selected hotbar item has a
  `place_block` target. Successful placement consumes one item.
- `Save & Quit` and window close persist player inventory in world metadata as
  item keys and counts.

## Rendering

Block and item textures share the renderer-side texture atlas. Item definitions
use `humancraft:item/<name>` texture keys that resolve to
`textures/items/<name>.png`.

The flat UI pass renders slot frames and stack counts. Inventory slots are sized
with the window aspect ratio so they are square in screen pixels. A textured UI
pass draws item icons. Dropped loot renders as double-sided textured quads in
the world pass so depth testing works against terrain.

When no hotbar item is selected, gameplay renders a shaded three-face
lower-right player arm overlay. When a stack is selected, placeable block items
render as a larger lower-right projected block with front/right/top cube faces
using the block's south/east/top textures in the first-person overlay.
Non-block items render as angled item sprites.

## Tests

Keep regression coverage focused on player-facing behavior and data integrity:

- inventory stack merging and overflow
- inventory left-click, right-click, left-drag, and right-drag stack behavior
- selected hotbar block placement and non-placeable item rejection
- stone-dropped cobblestone placement
- held block and arm overlay geometry staying visible, non-degenerate, and
  framed in the lower-right first-person view
- held block interaction repeat cadence
- block drops resolving to registered items
- breaking blocks spawning configured loot
- loot from a broken block under another block spawning in open space and
  falling normally
- pickup adding loot to inventory
- inventory save conversion and metadata round-tripping
- square inventory slot geometry
- loot render geometry staying above its contact point and rotating around Y
- every registered item referencing a loadable texture
