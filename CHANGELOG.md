# Changelog

All notable project changes are tracked here. `PROGRESS.md` remains the working
status file for verification notes, next steps, and implementation details.

## Unreleased

### Latest Session Summary

- Read the project PRD, development philosophy, and progress notes before
  continuing implementation.
- Added `docs/generating_textures.md` to document importing accurate block,
  item, mob, and UI textures from the default resource pack, including current
  grass/foliage tint-mask handling.
- Changed chunk persistence to an `HCCNK002` block-key palette format and made
  legacy raw-ID chunks regenerate from the world seed, fixing saved worlds that
  could show diamond-ore tree trunks, log canopies, or wrong terrain blocks
  after content registration order changed.
- Queued initially streamed chunks for save so loading and saving a world
  rewrites chunks in the keyed format.
- Reworked the open inventory and crafting table overlay around the original
  176 x 166 Minecraft container coordinates for closer panel, slot, crafting,
  result, and hotbar placement.
- Tightened inventory slot bevels to source-pixel-like edges and changed block
  item icons to a centered isometric projection so block stacks no longer look
  stretched in the inventory or crafting table.
- Changed dropped placeable block loot to render as small rotating textured
  cubes while non-block item drops remain flat rotating sprites.
- Replaced the procedural block-breaking crack pattern with the original
  `destroy_stage_0` through `destroy_stage_9` textures loaded from
  `textures/overlays`.
- Added a generated `textures/overlays/player_hand.png` first-person hand
  texture, registered it in the atlas, and render the empty selected hotbar
  slot through the textured UI pass.
- Enlarged and lowered the selected hotbar block overlay so placeable blocks
  sit partially off-screen in the lower-right first-person view, matching the
  Minecraft-style held-block framing more closely.
- Replaced current block, ore, item-resource, and sapling texture PNGs with
  matching default resource-pack artwork, with pre-tinted grass-top and
  oak-leaf masks for the renderer's current no-biome-tint path.
- Registered proper multi-face texture metadata for sandstone and crafting
  tables so their world and held-block rendering use distinct top, side,
  bottom, and front faces where available.
- Updated inventory and crafting item rendering so placeable block stacks draw
  as small three-face block meshes while non-block items remain flat sprites.
- Restyled inventory slot frames and stack counts toward Minecraft-like gray
  beveled slots with dark count shadows.
- Generated and added stone block face textures under `textures/blocks/stone`
  with the `create-block-texture` skill.
- Added block texture metadata through `BlockTextures` and registered texture
  references for grass, dirt, and stone.
- Added PNG texture loading with the `image` crate.
- Built the first renderer-side block texture atlas for the windowed `wgpu`
  client.
- Extended world render vertices with UV coordinates while keeping chunk
  meshing renderer-neutral.
- Updated the terrain shader to sample block textures with directional face
  shading.
- Fixed texture path resolution so assets load from the project root regardless
  of the process working directory.
- Fixed atlas sampling and lifetime details by insetting UVs by half a texel
  and keeping the underlying GPU atlas texture alive.
- Fixed the shader pipeline swap that made terrain render as flat white/dark
  gray faces even though the atlas had loaded correctly.
- Replaced unsafe spawn placement with a safe spawn search that validates the
  full player AABB before accepting a spawn position.
- Extended visible-face culling across loaded chunk boundaries in the windowed
  renderer, including re-exposing faces after neighboring border blocks are
  removed.
- Added simple infinite-world streaming to the windowed client: chunks are
  generated deterministically around the player as they move, and the temporary
  outer-boundary render filter now follows the loaded chunk bounds.
- Replaced the single merged terrain buffer with per-chunk render buffers so
  block edits and streamed chunk loads no longer remesh and reupload every
  loaded chunk.
- Added dirty-chunk remeshing for block edits and chunk streaming, including
  neighbor refreshes only where boundary culling can change.
- Fixed exposed bottom faces after block removal by preserving renderer-visible
  `Down` quads instead of filtering every bottom face after meshing.
- Reduced runtime chunk-load frame drops by throttling new chunk generation to
  a small per-frame budget.
- Reduced chunk-load remesh/upload spikes by queuing dirty chunks and applying
  a small per-frame mesh rebuild budget.
- Added a reusable biome generation layer with deterministic biome lookup and
  biome-driven terrain profiles.
- Added initial HumanCraft overworld biomes: plains, forest, and mountains.
- Added a generic tree decoration stage driven by tree definitions.
- Added oak log and oak leaves blocks/items, plus generated PNG textures for
  both block types.
- Smoothed terrain generation by interpolating value noise, sizing biomes as
  large chunk-based regions, and blending terrain height inside biome
  transition bands.
- Regenerated oak log and oak leaves textures to better match reference
  Minecraft-style bark and cut-out leaves.
- Fixed transparent leaf rendering by discarding near-transparent texture
  pixels in the terrain shader.
- Expanded camera pitch so the player can look almost fully up and down.
- Fixed the center crosshair proportions on widescreen displays.
- Made the selected-block outline brighter and easier to see.
- Made oak leaves more transparent by increasing cut-out pixels.
- Added desert biome support with sand and sandstone terrain strata.
- Added sand, sandstone, and bedrock blocks/items with generated textures.
- Added a bedrock generation stage that guarantees bedrock at Y 0.
- Made bedrock unbreakable through block tags respected by mining/breaking
  systems.
- Raised default overworld surfaces to start at Y 64 or higher.
- Added biome relief controls for roughness and ridge-style height variation.
- Tuned plains, forest, desert, and mountains for stronger terrain variation.
- Added exposed mountain stone on high or steep surfaces.
- Changed mountain exposed-stone placement to be slope-led so flatter high
  mountaintops remain grassy.
- Fixed underground rendering artifacts by winding terrain mesh faces outward,
  enabling counter-clockwise terrain back-face culling, and limiting the
  temporary preview filter to artificial loaded-world boundary faces.
- Added ore textures and texture coverage tests so registered blocks do not
  silently render with missing-texture fallback faces.
- Prevented player-facing block placement inside the player entity, including
  both feet/legs and head space.
- Reworked windowed player movement to use Minecraft-style fixed 20 Hz
  acceleration/friction physics, jump gravity/drag values, normalized diagonal
  input, sprint-jump boost, and sprint FOV widening.
- Added sprint activation through a configurable double-`Z` tap window.
- Added Shift sneaking with slower movement, lowered eye height, and edge
  protection to prevent sneaking players from walking off unsupported block
  edges.
- Fixed one-block jump clearance by applying the initial jump impulse before
  jump-start gravity/drag and preserving horizontal velocity while airborne
  against a ledge until the player rises high enough to move onto it.
- Tuned movement for more forgiving early playability by increasing walking
  acceleration, air control, and jump impulse relative to strict Minecraft
  values.
- Added player movement documentation for players, developers, and future
  coding agents.
- Added a startup main menu that leads into world management before gameplay
  starts.
- Added windowed world create, rename, delete, and load controls.
- Added per-world generation seeds, including numeric seed entry for
  reproducible terrain and automatic seed generation for quick creation.
- Added versioned save metadata and binary chunk files under `saves/worlds`.
- Changed chunk streaming so saved chunks override deterministic generation,
  preserving block edits across sessions.
- Saved player coordinates and camera orientation when pausing, losing focus,
  or closing the game.
- Added world-save developer documentation and regression coverage for
  metadata, chunk round-tripping, saved player state, and saved chunk reloads.
- Split HumanCraft content bootstrap into block, item, and overworld generation
  modules while preserving the public bootstrap API.
- Split windowed-client internals into client-world, inventory-interaction, and
  constants modules, reducing the app shell's ownership of gameplay helpers.
- Further split the client-world layer into spatial helpers, player collision,
  and dropped-loot behavior so the loaded-world module stays focused on chunk
  state, streaming, raycasts, block edits, and mesh preparation.
- Further reduced the windowed app shell by extracting input handling, frame
  update/render/remesh orchestration, session state, world lifecycle actions,
  texture atlas loading, world render conversion, HUD geometry, menu UI,
  inventory UI, and UI glyph building into focused modules.
- Moved windowed-client regression tests out of `src/app/windowed.rs` and into
  `src/app/windowed/tests.rs`.
- Added developer documentation for the windowed-client module boundaries and
  updated the architecture overview with the new structure.
- Replaced anonymous menu rectangles with readable screen-specific UI labels
  and clickable actions for main menu, world management, new-world config,
  rename, and pause overlay.
- Changed gameplay saves to dirty in-memory chunk/player state and flush only
  on `Save & Quit` or window close, avoiding disk writes during frame updates.
- Buffered chunk save writes into one contiguous write per chunk during flush.
- Added generic item stacks and a 36-slot player inventory with a 9-slot
  always-visible hotbar.
- Added the `E` inventory overlay with cursor release and close handling through
  `E` or `Esc`, using an isolated default binding helper for future
  configurable shortcuts.
- Added dropped loot entities for block breaks. Drops come from block
  definitions, fall under gravity, rotate continuously, and can be picked up
  into inventory stacks.
- Extended item definitions with texture keys and expanded the renderer atlas
  to include item icons for inventory UI and world-space loot billboards.
- Generated item textures for all current block items and starter resource
  drops under `textures/items`.
- Added the project-local `create-item-texture` skill and generator script for
  16 x 16 item icon PNGs.
- Added regression coverage for inventory stack merging, unresolved block
  drops, loot spawning, pickup, and registered item texture loading.
- Fixed dropped loot sprite placement so items render above their ground contact
  point instead of clipping into terrain.
- Changed dropped loot animation to rotate around the world Y axis.
- Made hotbar and inventory slots pixel-square and slightly larger on widescreen
  windows.
- Persisted player inventory in world metadata using item keys and stack counts,
  so collected items survive `Save & Quit` and world reload.
- Added regression coverage for inventory save conversion, metadata inventory
  round-tripping, square slot geometry, and loot Y-axis render geometry.
- Added Minecraft-style survival inventory manipulation for the current player
  inventory: whole-stack left click, half-stack right click, one-at-a-time right
  click placement, left-drag stack distribution, and right-drag one-per-slot
  placement.
- Added selected hotbar state changed with left and right arrow keys.
- Changed right-click block placement to use and consume the selected hotbar
  item only when that item is placeable.
- Added held left/right block interactions with an explicit repeat cadence, so
  holding right click continues placing selected hotbar blocks while placement
  remains valid.
- Added a shaded player arm overlay for an empty selected hand.
- Changed selected hand rendering so placeable blocks draw as projected
  three-face block meshes and non-block items draw as angled item sprites.
- Fixed first-person held block and arm projections to use stable visible
  three-face cuboid geometry, with held blocks sampling south/east/top block
  textures.
- Tuned first-person held arm, block, and item overlay geometry toward a
  larger lower-right Minecraft-style framing.
- Fixed dropped loot from blocks broken directly under another block so drops
  spawn in the opened block space, fall normally, and remain gatherable.
- Added cobblestone as a registered block with generated block textures, and
  made the cobblestone item place that block so stone drops can be placed.
- Updated the inventory and hotbar styling with Minecraft-like gray slot
  framing and a separated hotbar row in the full inventory overlay.
- Added regression coverage for click/drag inventory behavior and selected
  hotbar placement, including cobblestone placement.
- Added regression coverage for first-person held block and arm overlay
  geometry.
- Added regression coverage for held block interaction cadence and
  blocked-above loot drops.
- Added survival-style block resistance: holding left click now accumulates
  break progress from registered block hardness, resets when the target changes
  or input stops, respects unbreakable blocks, and shows staged crack overlay
  lines on the attacked block.
- Replaced block break overlay lines with Minecraft-style staged 16 x 16 crack
  pixels, extracted the overlay builder, shader sources, camera movement, key
  input, and shared render types from `windowed.rs`, and documented the
  250-line file-size target in the development philosophy.
- Added data-driven crafting recipes with shapeless and shaped matching.
- Added oak planks and crafting table block/item content with generated block
  and item textures.
- Added a 2 x 2 inventory crafting grid, a 3 x 3 crafting table UI opened by
  right clicking a crafting table block, and the first recipe: one oak log
  crafts four oak planks from any crafting slot.
- Added a crafting table recipe from a 2 x 2 square of oak planks.
- Fixed full-inventory UI layout overlap by shrinking open-inventory slots and
  separating the crafting area from the player inventory rows.
- Updated `PROGRESS.md` throughout the work with implementation notes,
  verification results, and next steps.
- Added and ran tests covering texture metadata mapping, real stone PNG
  loading, safe spawn placement, chunk streaming, deterministic distant chunk
  generation, underside face rendering, and dirty chunk selection.

### Project Setup

- Initialized the Rust Cargo project as `humancraft`.
- Added the library/application split.
- Added `.gitignore` for Cargo build output and generated preview output.
- Added `README.md` with current run, debug, and test commands.
- Added `PROGRESS.md` for ongoing implementation tracking.

### Core Engine

- Added generic registry primitives with duplicate-key protection.
- Added block and item definitions backed by registries.
- Added fixed-size chunk storage for `16 x 16 x 256` block IDs.
- Added a composable world generation pipeline.
- Added deterministic terrain generation.
- Added generic ore generation driven by ore definitions.
- Added initial content registration for air, grass, dirt, stone, coal ore,
  iron ore, gold ore, diamond ore, oak log, oak leaves, sand, sandstone,
  bedrock, and starter item definitions.

### World Rendering

- Added renderer-neutral chunk meshing with visible-face culling.
- Added preview artifact export for generated chunks:
  - ASCII heightmap
  - PPM heightmap
  - OBJ chunk mesh
- Added native `winit` + `wgpu` windowed dev mode.
- Added colored terrain rendering.
- Changed windowed rendering from one exposed chunk to a centered `5 x 5`
  terrain patch.
- Hid outer render-patch boundary faces so the temporary finite terrain area
  does not render as a sliced column.
- Hid finite loaded-patch Y 0 bottom boundary faces while keeping legitimate
  underground side and ceiling faces visible after block edits.
- Switched terrain rendering to outward counter-clockwise mesh winding with
  back-face culling, preventing top faces from rendering from underneath.
- Switched the camera projection to the depth range expected by `wgpu`.
- Added directional face shading for clearer untextured block faces.
- Added PNG loading through the `image` crate.
- Added reusable block texture metadata via `BlockTextures`.
- Registered texture metadata for grass, dirt, and stone.
- Generated `textures/blocks/stone` with the `create-block-texture` skill.
- Added a simple renderer-side block texture atlas.
- Extended world render vertices with UV coordinates while keeping chunk
  meshing renderer-neutral.
- Updated the terrain shader to sample block textures while preserving
  directional face shading.
- Added fallback texture handling for blocks without PNG assets.
- Fixed texture path resolution to use `CARGO_MANIFEST_DIR`.
- Inset atlas UVs by half a texel to reduce texture bleeding.
- Fixed a shader pipeline swap that caused textured terrain to still render as
  flat shaded faces.
- Kept the GPU atlas texture alive inside `TextureAtlas`.
- Added startup atlas accounting for loaded texture keys and fallback use.
- Extended the renderer-neutral chunk mesher with an optional outside-neighbor
  lookup so systems can cull hidden faces across chunk boundaries without
  moving visibility rules into GPU code.
- Added a simple client-side chunk streaming loop that keeps a square chunk
  window loaded around the player and generates missing chunks through the
  reusable world generation pipeline.
- Added per-frame runtime chunk-load throttling so movement into new areas does
  not generate the full render-distance patch in one frame.
- Added a per-frame dirty chunk remesh/upload budget for streamed chunks.
- Changed the windowed renderer to own GPU buffers per chunk instead of as one
  world-sized mesh, allowing small world updates to rebuild only affected chunk
  meshes.
- Added texture atlas coverage for oak log and oak leaves.
- Added alpha cut-out handling for transparent block texture pixels.
- Added texture atlas coverage for sand, sandstone, and bedrock.
- Added texture atlas coverage for coal ore, iron ore, gold ore, and diamond
  ore.

### World Generation

- Added biome definitions and a deterministic biome source.
- Changed terrain generation to read biome terrain profiles instead of one
  global terrain profile.
- Added plains, forest, and mountains biome content.
- Added desert biome content.
- Added biome-owned terrain layer stacks, allowing sand/sandstone/stone desert
  strata without hard-coding desert logic into the terrain stage.
- Added bedrock generation after terrain and ores so the deepest layer is
  always bedrock.
- Raised default terrain profiles to keep overworld surfaces at or above Y 64.
- Added biome relief and ridge controls, plus exposed-surface rules for
  mountain stone.
- Added chunk-sized biome region controls with configurable transition bands.
- Changed terrain noise from raw cell sampling to interpolated sampling.
- Changed biome-border terrain to blend neighboring biome height profiles.
- Added a tree decoration stage that places configurable trees by biome,
  allowed ground block, replaceable block set, shape, and density.
- Added oak tree generation in forest biomes, with sparse oak trees in plains.

### Player And Interaction

- Added a terminal playtest mode for early movement, mining, and placing.
- Added French-layout `ZQSD` movement using logical keyboard characters.
- Replaced arrow-key camera control with captured mouse-look.
- Added `Esc` pause/menu behavior that releases and recaptures the mouse cursor.
- Added a persistent windowed client world that stores generated chunks.
- Added grounded player-style movement with gravity and `Space` jump.
- Replaced height-only ground collision with an axis-separated player AABB.
- Added center crosshair rendering.
- Added camera raycasting against blocks.
- Added selected-block outline rendering for the current raycast target.
- Added left-click block breaking and right-click dirt placement.
- Rebuilt the chunk render mesh after block edits.
- Scoped block-edit render rebuilds to the edited chunk plus loaded horizontal
  neighbors when the edit touches a chunk boundary.
- Replaced initial spawn placement with a safe spawn search that validates the
  full player AABB.
- Reworked windowed player motion around a fixed 20 Hz physics tick with
  acceleration, ground friction, air control, Minecraft jump/gravity/drag
  values, sprint-jump boost, and diagonal movement normalization.
- Added double-`Z` sprint input with configurable timing.
- Added Shift sneaking with slower movement, lowered eye height, and
  unsupported-edge prevention.
- Added sprint FOV widening from the normal 70 degree view.
- Fixed jump-start vertical integration and airborne horizontal collision
  handling so jumping against a one-block ledge does not lose the height or
  movement needed to land on it.
- Increased movement forgiveness so one-block jumps work from a normal
  approach distance instead of requiring near-perfect positioning.
- Added a 9-slot hotbar, full inventory overlay on `E`, block-break loot drops,
  gravity-affected rotating dropped items, and pickup into player inventory.
- Fixed dropped item render placement, changed dropped item rotation to the Y
  axis, and made inventory slots pixel-square.
- Added selected-hotbar placement, selected hand rendering, and left/right arrow
  hotbar selection.
- Added cobblestone block placement for stone drops and improved first-person
  held item rendering.

### Items And Inventory

- Added `ItemStack` and `Inventory` as reusable engine-side item containers.
- Added `LootEntity` as lightweight world item data for dropped stacks.
- Added item texture metadata to item definitions.
- Registered missing starter item definitions needed by existing block drops,
  including cobblestone.
- Added texture coverage for all registered starter items.
- Persisted player inventory to world metadata as item keys and counts.
- Added player-inventory click and drag manipulation behavior.

### Skills And Tooling

- Added and repaired the `create-block-texture` skill under
  `.agents/skills/create-block-texture`.
- Moved the block texture generator into the standard skill resource path:
  `.agents/skills/create-block-texture/scripts/generate_block_texture.py`.
- Added `.agents/skills/create-block-texture/agents/openai.yaml`.
- Validated the `create-block-texture` skill with `quick_validate.py`.
- Smoke-tested the moved texture generator and confirmed it emits six `16 x 16`
  RGBA PNG block faces.

### Documentation

- Added developer documentation for:
  - architecture overview
  - registry system
  - chunk system
  - world generation
  - chunk meshing
  - terminal playtest
- Updated `README.md` as the run loop evolved.

### Tests And Verification

- Added unit tests for registries, chunks, generation pipeline, terrain, ores,
  content bootstrap, meshing, preview export, terminal playtest, AZERTY input,
  render filtering, coordinate splitting, mutable client-world block edits,
  raycasting, player AABB collision, selected-block outline geometry, texture
  metadata mapping, texture loading, safe spawn placement, biome lookup, biome
  registration, biome-region sizing, terrain continuity, camera pitch range,
  crosshair aspect compensation, oak leaf cut-out alpha, and tree generation.
- Verified the project repeatedly with:
  - `cargo fmt`
  - `cargo check`
  - `cargo test`
  - `cargo run -- preview`
  - `cargo run`
