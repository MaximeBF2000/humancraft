# Generating Textures

HumanCraft should use accurate reference textures when matching an existing
Minecraft-era block, item, mob, or UI element. Prefer copying and adapting the
default resource pack already present in the repository over hand-generating a
lookalike.

## Source Assets

The current reference pack lives at:

`minecraft_default_ressource_pack/assets/minecraft/textures`

Important subfolders:

- `blocks`: block faces, break overlays, plants, and terrain masks.
- `items`: inventory sprites for resources, tools, drops, and non-block items.
- `gui`: container and widget textures for menus, inventory, hotbar, and HUD.
- `entity`: mob and entity skins when mobs are introduced.
- `colormap`: tint maps used by the original game for grass and foliage.

## Block Textures

For a block that exists in the reference pack:

1. Find the closest original block PNG under `textures/blocks`.
2. Copy the real 16 x 16 PNG into `textures/blocks/<block_name>/`.
3. Duplicate the same file to `top.png`, `bottom.png`, `front.png`, `back.png`,
   `left.png`, and `right.png` only when the original block uses one texture on
   every face.
4. For multi-face blocks, copy each original face to the matching HumanCraft
   face file and register distinct `BlockTextures` metadata.

Examples:

- Stone, dirt, sand, cobblestone, bedrock, and ore blocks use one original PNG
  copied to all six faces.
- Oak logs use `log_oak_top.png` for top/bottom and `log_oak.png` for sides.
- Sandstone uses `sandstone_top.png`, `sandstone_bottom.png`, and
  `sandstone_normal.png`.
- Crafting tables use `crafting_table_top.png`, `planks_oak.png`,
  `crafting_table_front.png`, and `crafting_table_side.png`.
- Chests currently use generated opaque full-block faces under
  `textures/blocks/chest/` with distinct front, back, left, right, top, and
  bottom files. The side faces must remain opaque because the renderer treats
  the chest as a static full cube.

## Tint Masks

Some original textures are grayscale masks, not final colors. Grass tops and
oak leaves are examples. The full Minecraft renderer applies biome color maps
at runtime, but HumanCraft does not yet have that tint pass.

Until biome tinting exists, pre-tint those masks when importing them. The
current approach uses ImageMagick multiply colorization:

```sh
magick grass_top.png \( +clone -fill '#6fa12f' -colorize 100 \) -compose Multiply -composite textures/blocks/grass/top.png
magick leaves_oak.png \( +clone -fill '#5b9637' -colorize 100 \) -compose Multiply -composite textures/blocks/oak_leaves/top.png
```

Keep alpha from the original mask so cut-out leaves still render with holes.

## Item Textures

For non-block items, copy the matching original sprite from `textures/items`.
Examples: `coal.png`, `diamond.png`, `iron_ingot.png`, `gold_ingot.png`, and
`sapling_oak.png`.

Tools should also use the original item sprites when available. Current
HumanCraft tool textures are copied from the default pack for:

- `stick.png`
- `wood_pickaxe.png`, `stone_pickaxe.png`, `iron_pickaxe.png`,
  `diamond_pickaxe.png`
- `wood_shovel.png`, `stone_shovel.png`, `iron_shovel.png`,
  `diamond_shovel.png`
- `wood_axe.png`, `stone_axe.png`, `iron_axe.png`, `diamond_axe.png`

For block items, keep `textures/items/<block>.png` available for atlas coverage,
but render inventory icons as block meshes when possible. The UI should sample
the block's own south/east/top textures so item icons stay consistent with
world rendering.

## UI Textures

Use original GUI textures as layout references even when the renderer cannot
yet draw the full bitmap directly. The inventory screen uses the active area of
`textures/gui/container/inventory.png`, which is a 176 x 166 pixel panel inside
the 256 x 256 PNG. Preserve the original slot coordinates:

- Player inventory rows start at `(8, 84)`.
- Hotbar starts at `(8, 142)`.
- Inventory 2 x 2 crafting grid starts at `(88, 26)`.
- Inventory crafting result slot starts at `(144, 36)`.
- Crafting table 3 x 3 grid starts at `(30, 17)` in
  `textures/gui/container/crafting_table.png`.
- Crafting table result slot starts at `(124, 35)`.

When drawing with flat UI primitives, convert these source pixels to UI
coordinates instead of placing slots by eye. That keeps layout close to the
reference and makes future bitmap GUI rendering straightforward.

## Break Overlays

Use the original `textures/blocks/destroy_stage_0.png` through
`destroy_stage_9.png` files for block damage. HumanCraft stores them under
`textures/overlays/` and loads them into the same 16 x 16 atlas as block and
item textures. Do not hand-code crack patterns when the original stage textures
are available.

## First-Person Overlays

First-person held blocks should use the selected block's real face textures
instead of separate overlay artwork. Render them as projected cube faces so the
in-hand block matches world and inventory rendering.

For the empty hand, use a small 16 x 16 overlay texture in
`textures/overlays/player_hand.png`. Keep it pixel-art, high contrast, and
skin-toned; sample the original player skin palette from `textures/entity`
when possible, but crop or generate a dedicated overlay tile if the entity UVs
do not map cleanly to the first-person hand geometry.

## Mobs And Entities

When mobs are added, start from `textures/entity`. Copy the original skin into a
dedicated HumanCraft asset path and document the expected model UV layout next
to the entity renderer. Do not generate a skin by hand when a reference skin
exists.

If a mob or entity does not yet exist in HumanCraft but exists in the original
pack, import the texture only when the model, UVs, or sprite renderer can
actually use it.

## Verification

After importing textures:

1. Confirm every PNG is 16 x 16 for block and item atlas entries.
2. Run `cargo test` so block and item texture coverage catches missing paths.
3. Inspect a montage or in-game screenshot when changing visual assets.
4. Update `PROGRESS.md` and `CHANGELOG.md` with the imported asset scope.
