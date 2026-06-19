# Block Placement And Shapes

## Purpose

Block placement now produces a `BlockState`, not only a `BlockId`. The state
keeps content definitions data-driven while allowing individual placements of
the same block type to carry facing, slab orientation, stair half, and furnace
lit state, or log axis.

## Data Model

- `BlockDefinition::placement` selects a reusable placement rule.
- `BlockDefinition::shape` selects the collision and meshing shape family.
- `BlockState` stores the block id plus `BlockProperties`.
- Chunks expose `block()` for legacy id-only callers and `block_state()` for
  state-aware systems.

Current placement rules:

- `Simple`: no properties.
- `FacePlayerHorizontal`: chests and furnaces face the player.
- `AxisFromClickedFace`: logs use the clicked face to choose X, Y, or Z axis.
- `Slab`: wooden slabs use the clicked face to choose horizontal or vertical
  orientation.
- `Stairs`: wooden stairs use player facing plus hit height or clicked face to
  choose top/bottom half. Stairs rise in the player's horizontal look
  direction.

Current shape families:

- `Empty`: air.
- `FullCube`: regular blocks, chest, furnace, and glass.
- `Slab`: one half-block AABB in one of six orientations.
- `Stairs`: two AABBs, with no automatic inner or outer corner connection.

## Runtime Behavior

Raycasts record the hit block, previous block cell, clicked face, and hit
position. Placement uses that context plus the player look direction.

Right-clicking a utility block opens its UI unless the player is holding
`Shift`; sneaking right-clicks continue through the normal placement path so
players can place blocks against crafting tables, chests, and furnaces.

Partial block AABBs are used for collision and mesh generation. Full opaque
cubes still occlude neighbor faces; partial blocks do not occlude adjacent
blocks yet, which keeps the first implementation visually correct at the cost
of extra hidden faces.

Wooden slabs merge only with the opposite wooden slab orientation in the same
cell. The merged block is oak planks.

## Known Limits

- Saved chunks persist block-state properties in the `HCCNK003` chunk format,
  so placed stair, slab, log, chest, and furnace orientation survives
  save/load. Placed chest and furnace block-entity inventories are persisted in
  the world save's `block_entities.txt` file.
- Chest and furnace block entities have a shared container UI. Furnace output
  is take-only, player shift-click routes fuels and smeltables into the
  matching furnace slots, and the furnace UI renders fuel/cook progress from
  block entity timers.
- Breaking a chest stores its block entity inventory on the dropped chest item
  as runtime stack metadata. Placing that item restores the contents into the
  new chest entity.
- Stair corner connection, waterlogging, support rules, and partial-face
  attachment are intentionally out of scope.
