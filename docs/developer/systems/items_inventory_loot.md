# Items, Inventory, And Loot

## Purpose

Items are registry-backed content definitions. Inventory and loot systems store
item IDs and stack counts, not concrete content types.

## Current Data Model

- `ItemDefinition` lives in `src/engine/world/item.rs`.
- Tools are item definitions with optional `ToolDefinition` metadata. The
  metadata records tool kind, material, harvest level, and vanilla-style speed
  multiplier.
- `ItemStack` stores an `ItemId`, count, and optional stack metadata. Metadata
  is used for non-stackable stateful items such as broken chests carrying their
  runtime inventory.
- `Inventory` owns fixed slots and merges compatible stacks before using empty
  slots.
- `CraftingRecipeDefinition` lives in `src/engine/world/crafting.rs`.
  Recipes are registered in a `CraftingRecipeRegistry` and can be shapeless or
  shaped. Shapeless recipes match ingredients anywhere in the grid; shaped
  recipes match an exact pattern inside the available grid.
- `SmeltingRecipeDefinition` lives in `src/engine/world/smelting.rs`.
  Smelting recipes map one input item key to one output stack and a cook
  duration.
- Item definitions can declare `fuel_ticks`; furnaces consume that metadata
  instead of matching hard-coded item keys.
- Player inventory is currently 36 slots, with the first 9 slots rendered as
  the always-visible hotbar.
- `RenderState` owns transient UI state for a carried cursor stack, current
  drag operation, selected hotbar index, the 2 x 2 inventory crafting grid,
  the 3 x 3 crafting table grid, and the current crafting result.
- `LootEntity` lives in `src/engine/world/loot.rs` and stores an item stack,
  world position, velocity, and rotation.

Item stack sizes come from item definitions. Current starter content uses the
Minecraft-style default of 64 items per stack.

## Block Drops

Blocks declare drops as item keys on `BlockDefinition::drops`. Windowed
gameplay resolves those keys through `ItemRegistry` when a block breaks and
spawns one dropped item entity per configured drop. Blocks can also declare a
`harvest_requirement`; if the selected item does not provide a matching tool
kind and sufficient harvest level, the block still breaks but configured drops
are suppressed.

Keep this path data-driven:

- Do not hard-code per-block drop behavior in the windowed controller.
- Add or change drops on block definitions first.
- Keep tests that every block drop resolves to a registered item.
- If a dropped block resource should be placeable, register it as a block item
  with a `place_block` target. Stone uses this path by dropping placeable
  cobblestone.
- Plant-like placeables use the same item path. Oak saplings are item
  definitions with `place_block = humancraft:oak_sapling`, while the block
  definition owns the cross shape and growth behavior.
- Keep harvest requirements on block definitions. Do not branch on concrete
  block keys from the windowed controller.

## Tools

Current HumanCraft tool content includes wooden, stone, iron, and diamond
pickaxes, shovels, and axes. Tool material data follows the vanilla speed
multiplier progression used by the current mining system:

- wood: harvest level 1, speed multiplier 2
- stone: harvest level 2, speed multiplier 4
- iron: harvest level 3, speed multiplier 6
- diamond: harvest level 4, speed multiplier 8

Break duration is computed from block hardness. Blocks with a matching
effective tool use `hardness * 1.5 / tool_speed_multiplier` seconds. Blocks
without a selected matching tool keep hand speed when no harvest tool is
required, or use the slower inefficient path of `hardness * 5.0` seconds when a
harvest tool is required.

Current block tool data:

- shovel-effective: grass, dirt, sand
- axe-effective: oak logs, oak planks, crafting tables
- pickaxe-required level 1: stone, cobblestone, sandstone, coal ore
- pickaxe-required level 2: iron ore
- pickaxe-required level 3: gold ore, diamond ore

Iron ore now drops `humancraft:raw_iron`, which smelts into
`humancraft:iron_ingot`.

## Crafting

Recipes are content data, not windowed-client branches. Add or update
HumanCraft recipes in `src/content/recipes.rs`; keep the engine matcher generic
and resolve item keys through `ItemRegistry`.

Current starter crafting content:

- shapeless `humancraft:oak_planks_from_oak_log`: one `humancraft:oak_log`
  anywhere in a 2 x 2 or 3 x 3 grid produces four `humancraft:oak_planks`.
- shaped `humancraft:crafting_table_from_oak_planks`: a filled 2 x 2 square of
  `humancraft:oak_planks` produces one `humancraft:crafting_table`.
- shaped `humancraft:sticks_from_oak_planks`: two vertical
  `humancraft:oak_planks` produce four `humancraft:stick`.
- shaped wooden, stone, iron, and diamond tool recipes use the original
  Minecraft 3 x 3 pickaxe, shovel, and axe shapes. Axes include both left and
  right orientations.
- shaped `humancraft:chest_from_oak_planks`: eight oak planks around an empty
  center produce one chest.
- shaped `humancraft:furnace_from_cobblestone`: eight cobblestone around an
  empty center produce one furnace.
- shaped wooden stair recipes use six oak planks in either stair orientation
  and produce four wooden stairs.
- shaped `humancraft:wooden_slab_from_oak_planks`: three horizontal oak planks
  produce six wooden slabs.

Current starter smelting content:

- `humancraft:sand` smelts into `humancraft:glass`.
- `humancraft:raw_iron` smelts into `humancraft:iron_ingot`.
- Coal, oak planks, sticks, oak logs, wooden stairs, and wooden slabs are
  registered as fuels through item metadata.
- Furnace recipes use the vanilla 200 game tick cook duration, which is 10
  seconds at the current 20 Hz gameplay tick rate.

The windowed client owns only transient crafting inputs. Closing inventory,
pausing, saving, or switching back to menus attempts to return crafting-grid
stacks to the player inventory. Crafting output is recomputed from recipe data
after every input-grid change.

## Windowed Gameplay

`src/app/windowed.rs` currently owns the temporary player inventory and loot
simulation until a fuller gameplay/entity layer exists.

Current behavior:

- Holding left click advances block breaking according to
  `BlockDefinition::hardness`, selected-item tool metadata, and the target
  block's effective tool. A block is replaced with air only after its current
  break progress reaches the required duration. Configured loot spawns only if
  the selected item satisfies the target block's harvest requirement. Blocks
  tagged `unbreakable` never start break progress.
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
  - shift-left click transfers player stacks between hotbar and main inventory
    or shift-crafts as many results as fit
  - with a chest open, player shift-left click moves stacks into the chest and
    container shift-left click moves stacks back into the player inventory
  - with a furnace open, player shift-left click routes fuels to the fuel slot
    and smeltable items to the input slot; furnace output remains take-only
  - left drag distributes a carried stack over compatible player or crafting
    slots
  - left or right drag also works across compatible chest/furnace slots; furnace
    output rejects manual placement
  - double-left click while carrying a stack collects matching visible stacks
    up to the item stack limit
  - number keys `1` through `9` swap a hovered player or crafting slot with the
    matching hotbar slot
  - `Q` and `Ctrl+Q` drop one item or a full stack from the cursor or hovered
    slot into the world with a small forward impulse
  - left-drag distribution restores the drag-start inventory snapshot and
    reapplies the distribution on every newly entered slot so the UI updates
    optimistically while dragging
- Left and right arrows move the selected hotbar slot.
- The current UI has no separate armor, offhand, or creative layout yet.
  Runtime chest and furnace block entities use a shared container layout:
  chests expose 27 slots, furnaces expose vanilla-positioned input, fuel, and
  output slots, and furnace output rejects manual placement. `InventorySlotId`
  keeps the player, crafting, and container behavior typed so future slot
  regions can be added without binding rules to screen coordinates.
- Right-click block placement only works when the selected hotbar item has a
  `place_block` target. Successful placement consumes one item.
- Placing oak saplings writes a stage-0 sapling block state. Sapling growth is
  handled by the block behavior tick system, not by inventory or placement
  code.
- Right clicking a block tagged `crafting_table` opens the 3 x 3 crafting table
  UI instead of placing a block against it.
- Breaking a chest removes its block entity without spilling its slots and
  drops a non-stackable chest item carrying that inventory as runtime stack
  metadata. Placing that chest item restores the carried inventory into the new
  chest block entity.
- `Save & Quit` and window close persist player inventory in world metadata as
  item keys and counts. Placed chest and furnace block entities are persisted in
  `block_entities.txt` with item keys/counts and furnace timers. Stack metadata
  is runtime-only for now, so carried chest inventories still need durable save
  support before they survive saving inside player inventory or dropped loot.

## Rendering

Block and item textures share the renderer-side texture atlas. Item definitions
use `humancraft:item/<name>` texture keys that resolve to
`textures/items/<name>.png`.

The flat UI pass lays out the open inventory and crafting table from the
original 176 x 166 container texture coordinates, then renders Minecraft-style
slot frames with one-to-two-pixel bevels, hover highlights, larger item stack
counts with a dark text shadow, and item-name tooltips. Tooltips are drawn in a
final solid UI pass after textured item icons so they stay above items and slot
contents. Inventory slots are sized with the window aspect ratio so they are
square in screen pixels. A textured UI pass draws non-block item icons as
sprites and placeable block item icons as larger small isometric block meshes
using the block's south/east/top textures. Dropped loot renders non-block items
as double-sided textured quads, while placeable block drops render as small
rotating cubes with block face textures.

When no hotbar item is selected, gameplay renders a textured three-face
lower-right player hand overlay from `textures/overlays/player_hand.png`. When
a stack is selected, placeable block items render as a larger lower-right
projected block with front/right/top cube faces using the block's
south/east/top textures in the first-person overlay. The held block is allowed
to extend beyond the bottom-right screen edge so it reads like Minecraft's
first-person held-block framing. Non-block items render as angled item sprites.

## Tests

Keep regression coverage focused on player-facing behavior and data integrity:

- inventory stack merging and overflow
- recipe matching for shapeless and shaped crafting
- inventory left-click, right-click, left-drag, and right-drag stack behavior
- shift-click transfer, double-click collect, hotbar key swap, and inventory
  drop helpers
- selected hotbar block placement and non-placeable item rejection
- stone-dropped cobblestone placement
- held block and arm overlay geometry staying visible, non-degenerate, and
  framed in the lower-right first-person view
- player hand overlay texture loading
- held block interaction repeat cadence
- block drops resolving to registered items
- breaking blocks spawning configured loot
- tool tiers controlling break speed
- harvest requirements suppressing drops for hand or under-tier tools
- loot from a broken block under another block spawning in open space and
  falling normally
- pickup adding loot to inventory
- inventory save conversion and metadata round-tripping
- broken chest items preserving their runtime inventory when picked up and
  placed again
- square inventory slot geometry
- loot render geometry staying above its contact point and rotating around Y
- every registered item referencing a loadable texture
