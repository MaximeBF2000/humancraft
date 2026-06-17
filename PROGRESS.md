# HumanCraft Progress

Last updated: 2026-06-17

## Done

- Fixed inventory tooltip layering so tooltips draw after item icons, changed
  tooltip styling to dark gray, and increased tooltip padding.
- Changed fast double-left-click collection to track the clicked item rather
  than requiring the second click to land on a still-filled slot, so collecting
  matching visible stacks works after the first click picks up the source
  stack.
- Changed left-drag stack distribution to update optimistically while dragging
  by restoring the drag-start player/crafting inventory snapshot and
  redistributing across the currently dragged slots.
- Added a runtime Settings menu reachable from the main menu and pause menu,
  with a Shortcuts screen for rebinding movement, jump, crouch, inventory,
  pause, drop, hotbar previous/next, and hotbar slot selection keys.
- Routed gameplay movement, inventory toggling, dropping, and hotbar selection
  through the active shortcut bindings. Dropping now also works in gameplay for
  the selected hotbar slot, with `Ctrl` still dropping the full stack.
- Added regression coverage for rebindable movement shortcuts and shortcut menu
  row hit detection.
- Improved the open inventory and crafting table UI toward the Minecraft
  reference:
  - larger item icons and stack counts
  - stronger gray beveled slot contrast
  - hovered slot highlight
  - item-name tooltip following the cursor and clamped to the screen
- Expanded inventory interactions for visible player and crafting slots:
  - shift-click quick transfer between hotbar and main inventory
  - shift-click crafting result crafts as many results as fit
  - double-left click while carrying a stack collects matching visible stacks
  - number keys `1` through `9` swap the hovered slot with the matching hotbar
    slot
  - `Q` and `Ctrl+Q` drop one item or a full stack from the cursor or hovered
    slot
  - clicking outside the inventory while carrying a stack drops it near the
    player with a small forward impulse
  - drag distribution now works across compatible player and crafting input
    slots instead of only player inventory slots
- Added typed windowed-client inventory slot IDs and reusable helper functions
  for quick transfer, double-click collection, hotbar swaps, and stack drops.
- Added regression coverage for shift-click transfer, double-click collect,
  hotbar swapping, stack decrement/full-stack drops, and dropped inventory
  stack spawn impulse.
- Read `PRD.md` and `DEV_PHILOSOPHY.md`.
- Added reusable tool metadata to item definitions:
  - tool kind
  - tool material
  - harvest level
  - mining speed multiplier
- Added reusable block harvest metadata:
  - effective tool kind
  - optional harvest requirement with minimum level
- Added HumanCraft stick and tool content:
  - sticks
  - wooden, stone, iron, and diamond pickaxes
  - wooden, stone, iron, and diamond shovels
  - wooden, stone, iron, and diamond axes
- Added Minecraft-style crafting recipes:
  - two vertical oak planks craft four sticks
  - pickaxes use three material items above two centered sticks
  - shovels use one material item above two sticks
  - axes use three material items around two sticks in either orientation
- Copied stick, tool, and iron-ingot item textures from
  `minecraft_default_ressource_pack/assets/minecraft/textures/items`.
- Added tool-aware block breaking in the windowed client:
  - matching tools use vanilla-style speed multipliers
  - hand or wrong-tool breaking still works
  - harvest-required blocks break slowly without a matching tool
  - harvest-required blocks only drop loot with a sufficient tool
- Configured starter harvest progression:
  - stone, cobblestone, sandstone, and coal ore require a wooden pickaxe or
    better
  - iron ore requires a stone pickaxe or better
  - gold and diamond ore require an iron pickaxe or better
  - dirt, grass, and sand are shovel-effective
  - oak logs, oak planks, and crafting tables are axe-effective
- Temporarily changed iron ore to drop iron ingots directly so iron tools are
  craftable before furnace smelting exists.
- Added regression coverage for stick crafting, iron pickaxe crafting, tool
  tier break speed, harvest-gated drops, and the imported tool item textures.
- Initialized a Rust Cargo project named `humancraft`.
- Added a library/application split.
- Added generic registry primitives with duplicate-key protection.
- Added block and item definitions backed by registries.
- Added fixed-size chunk storage for `16 x 16 x 256` block IDs.
- Added a composable world generation pipeline.
- Added deterministic terrain generation.
- Added generic ore generation driven by ore definitions.
- Added initial HumanCraft content through bootstrap registration:
  - air
  - grass
  - dirt
  - stone
  - coal ore
  - iron ore
  - gold ore
  - diamond ore
  - matching starter item definitions
- Added developer docs for architecture, registries, chunks, and generation.
- Added unit tests for registries, chunks, generation pipeline, terrain, ores,
  and content bootstrap.
- Added `.gitignore` for Cargo build output.
- Added renderer-neutral chunk meshing with visible-face culling.
- Added preview artifact export:
  - ASCII heightmap
  - PPM heightmap
  - OBJ chunk mesh
- Added a terminal playtest mode with movement, mining, and placing.
- Added native windowed dev mode using `winit` and `wgpu`.
- Added colored terrain rendering for one generated chunk.
- Added camera movement using French-layout controls:
  - `Z` forward
  - `Q` left
  - `S` backward
  - `D` right
- Fixed windowed movement to use logical keyboard characters instead of
  physical key positions, so AZERTY `ZQSD` works as intended.
- Changed the first windowed render from a single exposed chunk to a centered
  `5 x 5` terrain patch.
- Hid outer render-patch boundary side faces so the finite terrain patch does
  not show as a sliced chunk wall.
- Switched the camera projection to wgpu's expected depth range.
- Replaced arrow-key camera control with captured mouse-look.
- Added `Esc` pause/menu mode that releases the cursor, shows a simple overlay,
  and lets `Esc` resume/capture the cursor again.
- Added a persistent windowed client world that stores generated chunks instead
  of throwing them away after meshing.
- Added grounded player-style movement:
  - `ZQSD` horizontal movement
  - gravity
  - `Space` jump
  - automatic ground following
- Added center crosshair rendering.
- Added camera raycasting against blocks.
- Added left-click block breaking.
- Added right-click dirt placement.
- Rebuilds the chunk render mesh after block edits.
- Replaced height-only ground collision with a small player AABB moved
  axis-by-axis, so higher blocks block horizontal movement instead of snapping
  the player to the top surface.
- Added selected-block outline rendering for the current raycast target.
- Added directional face shading to make untextured blocks read better.
- Inspected `textures/blocks/grass` and `textures/blocks/dirt`; texture atlas
  support is still pending because the renderer does not yet load PNG assets.
- Added `README.md` with current run/test commands.
- Generated `textures/blocks/stone` with the `create-block-texture` skill:
  - top
  - bottom
  - front
  - back
  - left
  - right
- Added block texture metadata to `BlockDefinition` through a reusable
  `BlockTextures` value.
- Registered texture data for grass, dirt, and stone in content bootstrap.
- Added PNG loading through the `image` crate.
- Added a simple renderer-side block texture atlas for windowed `wgpu`
  rendering.
- Extended world render vertices with UV coordinates while keeping chunk
  meshing renderer-neutral.
- Updated the world shader to sample block textures and preserve directional
  face shading.
- Kept a white/magenta fallback texture path so blocks without PNG assets still
  render visibly.
- Added a unit test covering block texture metadata to asset-path mapping.
- Fixed texture loading to resolve block PNGs from `CARGO_MANIFEST_DIR` instead
  of the process working directory.
- Inset atlas UVs by half a texel to avoid edge bleeding between adjacent
  16 x 16 block textures.
- Fixed the windowed render pipeline to use the textured block shader. The
  textured and line shaders had been accidentally swapped, so the atlas loaded
  successfully but terrain still rendered as flat shaded faces.
- Kept the underlying GPU atlas texture alive inside `TextureAtlas` instead of
  storing only the view and sampler.
- Added startup atlas accounting for loaded texture keys and fallback texture
  use.
- Replaced the windowed spawn placement with a safe spawn search that validates
  the full player AABB before accepting a position.
- Added tests that verify stone texture PNGs load and safe spawn positions do
  not collide with blocks.
- Extended chunk meshing with a neighbor lookup so visible-face culling also
  removes hidden faces across chunk boundaries.
- Updated the windowed client world to mesh chunks against adjacent loaded
  chunks, so breaking a border block exposes the neighboring block face on the
  next mesh rebuild.
- Added regression tests for border-face culling and face re-exposure after a
  neighboring block is removed.
- Added simple infinite-world streaming in the windowed client:
  - `ClientWorld` owns the generation pipeline and context.
  - Missing chunks are generated deterministically around the player as they
    move.
  - Generated chunks remain resident for now; unload/save policy is still a
    future system.
  - The temporary outer-boundary render filter now follows the loaded chunk
    bounds instead of a hard-coded startup patch.
- Added tests for render-to-chunk coordinate mapping, chunk streaming, and
  deterministic distant chunk generation.
- Replaced the single global terrain GPU buffer with per-chunk render buffers
  in the windowed client.
- Added dirty-chunk remeshing for world updates:
  - interior block edits remesh only their owning chunk
  - chunk-border block edits also remesh the loaded neighboring chunk whose
    culled faces may change
  - newly streamed chunks upload only themselves and already-loaded horizontal
    neighbors affected by boundary culling
- Fixed exposed bottom faces by allowing renderer-visible `Down` quads through
  the temporary preview/render filter instead of discarding all bottom faces
  after meshing.
- Added regression tests for underside face filtering and dirty chunk selection.
- Added an expandable biome generation layer:
  - `BiomeDefinition`
  - `BiomeSource`
  - biome-driven terrain profiles
- Added initial overworld biomes:
  - plains
  - forest
  - mountains
- Added generic tree decoration generation through `TreeDefinition` and
  `TreeStage`.
- Added oak tree content using the tree stage:
  - oak log block and item
  - oak leaves block and item
  - oak sapling item drop target
- Generated oak log and oak leaves PNG textures under `textures/blocks` with
  the `create-block-texture` skill.
- Updated world-generation docs for biome and tree stages.
- Replaced raw cell value-noise terrain sampling with interpolated value noise.
- Changed biome selection to large macro-regions measured in chunks:
  - default overworld regions are 10 chunks wide
  - region borders blend over a 2-chunk transition band
  - region cores use one primary biome for decorations
- Changed terrain height to blend biome height profiles inside transition
  bands, avoiding hard cliffs at chunk and biome boundaries.
- Added regression coverage for interpolated noise, biome-region sizing,
  transition width, biome influence normalization, biome region cores, and
  chunk-border terrain continuity.
- Regenerated oak log and oak leaves textures to better match the intended
  Minecraft-style references:
  - oak logs now have ringed end caps and vertical bark strips
  - oak leaves now use cut-out alpha with transparent holes
- Added alpha discard in the textured terrain shader so transparent leaf pixels
  render as holes.
- Expanded camera pitch to allow looking almost fully up and down while
  avoiding the exact vertical singularity.
- Rebuilt the center crosshair with aspect-ratio compensation so horizontal and
  vertical bars appear the same length on widescreen displays.
- Made selected-block outlines brighter, slightly larger, and depth-independent
  so the raycast target is easier to read.
- Added regression coverage for camera pitch range, crosshair aspect
  compensation, and oak leaf cut-out alpha.
- Made oak leaves more transparent by increasing cut-out pixels in the leaf
  textures.
- Added desert biome support through biome-owned terrain layer stacks.
- Added sand, sandstone, and bedrock blocks and matching block items.
- Generated sand, sandstone, bedrock, and more transparent oak leaves textures
  with the `create-block-texture` skill.
- Added the `BedrockStage` to guarantee every generated chunk has bedrock at
  Y 0.
- Marked bedrock with an `unbreakable` tag and made windowed block breaking and
  terminal playtest mining respect that tag.
- Raised the default overworld terrain profiles so sampled surfaces start at
  Y 64 or higher, with mountains substantially higher.
- Added tests for custom biome terrain layers, desert sand/sandstone/stone
  strata, bedrock bottom layers, unbreakable block behavior, sea-level minimum
  terrain, and new block texture loading.
- Added biome terrain relief controls:
  - secondary roughness noise
  - ridge-style height contribution
  - per-biome roughness/ridge tuning
- Tuned plains, forest, desert, and mountains for stronger height variation.
- Added exposed-surface rules and configured mountains to expose stone on steep
  surfaces while keeping high flatter tops grassy.
- Added regression coverage for biome height variation and exposed mountain
  stone.
- Fixed underground rendering artifacts after digging below terrain:
  - terrain mesh quads now use outward counter-clockwise winding
  - terrain renders with `FrontFace::Ccw` and back-face culling
  - the temporary finite-world preview filter hides loaded-patch bottom
    boundary faces
  - real underground side and ceiling faces remain visible after block edits
- Reduced chunk-load frame drops by generating at most two new runtime chunks
  per frame and removing the duplicate pre-movement chunk ensure pass.
- Further reduced runtime chunk-load spikes by queuing dirty chunk remeshes and
  rebuilding/uploading at most three chunk meshes per frame.
- Generated and registered coal, iron, gold, and diamond ore textures so ore
  blocks no longer fall back to unicolor missing-texture faces.
- Added a broad texture coverage test requiring every registered non-air block
  to use loadable texture assets.
- Changed player-facing placement so a block cannot be placed inside the
  player's entity AABB, including both leg and head space.
- Reworked windowed player movement around Minecraft-style fixed-tick physics:
  - 20 Hz physics stepping
  - acceleration-based horizontal movement
  - ground friction and air control
  - Minecraft jump velocity, gravity, and air damping
  - diagonal movement normalization through normalized input vectors
  - sprint-jump horizontal boost
- Added sprint movement through a configurable double-`Z` tap window.
- Added Shift sneaking with slower movement, lowered eye height, and edge
  protection so a sneaking player cannot walk off an unsupported block edge.
- Added sprint FOV widening and normal 70 degree camera FOV.
- Added focused regression tests for sprint double-tap timing and sneak edge
  protection.
- Fixed one-block jump clearance by applying the initial jump impulse before
  gravity/drag on the jump-start tick and preserving blocked horizontal
  velocity while airborne, allowing the player to keep pushing toward a ledge
  until the jump is high enough to clear it.
- Added regression coverage for jumping onto a one-block ledge.
- Tuned movement away from strict Minecraft values for better early playability:
  stronger walking acceleration, stronger air control, and a higher jump
  impulse make one-block jumps much more forgiving while preserving the
  fixed-tick acceleration/friction model.
- Strengthened one-block jump regression coverage to start from a normal
  approach distance instead of almost touching the ledge.
- Added player-facing movement documentation in `docs/player/controls.md`.
- Added developer movement-system documentation in
  `docs/developer/systems/player_movement.md`.
- Added `AGENTS.md` with movement documentation pointers for future coding
  agents.
- Added regression coverage for underground preview filtering, loaded-world
  bottom boundary filtering, chunk-load budgeting, player-occupied placement,
  texture coverage, and high grassy mountain tops.
- Added a startup main menu and world-management flow before gameplay starts.
- Added world create, rename, delete, and load controls in the windowed client.
- Added per-world generation seeds, with numeric seed entry for reproducible
  terrain and automatic seeds for quick world creation.
- Added `WorldSaveStore` for versioned world metadata and binary chunk files
  under `saves/worlds`.
- Changed windowed chunk streaming to load saved chunk files before falling
  back to deterministic generation from the world's seed.
- Saved edited chunks after block changes and saved player position/camera
  orientation when pausing, losing focus, or closing the window.
- Added developer documentation for world saves.
- Added data-driven crafting recipes through `CraftingRecipeDefinition` and
  `CraftingRecipeRegistry`, with shapeless and shaped matching.
- Added oak planks and crafting table blocks/items through content bootstrap.
- Generated oak plank and crafting table block/item textures under
  `textures/blocks` and `textures/items`.
- Added a 2 x 2 crafting grid to the inventory overlay and a 3 x 3 crafting
  table overlay opened by right clicking a crafting table block.
- Added the first craft: one oak log anywhere in the crafting grid produces
  four oak planks.
- Added regression coverage for shapeless and shaped recipe matching, plus
  existing texture coverage for the newly registered blocks/items.
- Added the crafting table recipe: a 2 x 2 square of oak planks crafts one
  crafting table.
- Reworked full-inventory UI slot sizing and layout so crafting controls and
  player inventory slots do not overlap at common 4:3 and 16:9 window aspects.
- Added hardness-based survival block breaking in the windowed client:
  - break duration is derived from `BlockDefinition::hardness`
  - holding left click accumulates progress on the targeted block
  - releasing left click or changing targets clears current progress
  - blocks tagged `unbreakable` never start break progress
  - successful breaks still use the existing data-driven drop path
- Added staged crack overlay lines for the currently attacked block.
- Added regression coverage for hardness break timing, target-change progress
  reset, unbreakable break progress rejection, held-left-click cadence, and
  staged crack overlay geometry.
- Replaced the line-based break overlay with a Minecraft-style staged 16 x 16
  pixel crack mask rendered as filled block-face quads.
- Extracted break overlay mesh generation to
  `src/app/windowed/block_break_overlay.rs`.
- Extracted windowed WGSL shader strings to `src/app/windowed/shaders.rs`.
- Extracted windowed camera movement, key input, and shared render vertex types
  to focused `camera`, `input`, and `render_types` modules.
- Updated `DEV_PHILOSOPHY.md` and windowed-client developer docs with an
  explicit 250-line file-size target and a rule to split responsibilities
  before adding behavior to oversized files.
- Split HumanCraft content bootstrap by domain:
  - `src/content/blocks.rs`
  - `src/content/items.rs`
  - `src/content/generation.rs`
- Split windowed-client implementation by ownership:
  - `src/app/windowed/client_world.rs` for loaded chunks, raycasts, block
    edits, collision checks, spawn search, and loot updates
  - `src/app/windowed/inventory_interaction.rs` for inventory click/drag and
    save conversion behavior
  - `src/app/windowed/constants.rs` for explicit windowed-client tuning values
- Further split the client-world layer so `src/app/windowed/client_world.rs`
  only owns client chunk state, streaming, raycasts, block edits, and render
  mesh preparation:
  - `src/app/windowed/player_collision.rs` owns spawn search, player AABB
    collision, ground support, and occupied-placement rejection
  - `src/app/windowed/spatial.rs` owns world/render coordinate conversion,
    block positions, AABB helpers, and dirty-neighbor chunk selection
  - `src/app/windowed/loot.rs` owns dropped loot spawning, physics, and pickup
- Continued the `src/app/windowed.rs` refactor by extracting app input,
  frame update/render/remesh orchestration, session state, world lifecycle,
  texture atlas loading, world render mesh conversion, HUD geometry, menu UI,
  inventory UI, UI glyph building, and windowed-client regression tests.
- Reduced `src/app/windowed.rs` from 4,830 lines to 720 lines in this pass.
- Documented remaining oversized windowed-client files as follow-up extraction
  candidates before new behavior is added there.
- Added `docs/developer/systems/windowed_client.md` to document current
  windowed-client module boundaries and future split candidates.
- Updated architecture documentation to point future client work at the new
  module boundaries.
- Added a reusable item stack and inventory model:
  - `ItemStack`
  - 36-slot player inventory
  - 9-slot hotbar view
  - stack merging up to item-defined max stack sizes, currently 64 by default
- Added `LootEntity` as a lightweight world item entity with stack, position,
  velocity, and rotation data.
- Extended item definitions with texture keys and registered item textures for
  all current block items and resource drops.
- Added a `create-item-texture` project skill with a generator script for
  16 x 16 inventory and loot item icons.
- Generated item textures under `textures/items` for current blocks and drops:
  dirt, grass, stone, cobblestone, ores, coal, raw ore, diamond, oak log,
  oak leaves, oak sapling, sand, sandstone, and bedrock.
- Added a hotbar that is always rendered at the bottom of gameplay.
- Added the `E` inventory overlay:
  - opens a 36-slot inventory grid
  - releases the cursor while open
  - closes with `E` or `Esc`
  - keeps input comparison isolated behind a default binding helper for future
    configurable shortcuts
- Changed windowed block breaking to spawn configured block drops as rotating,
  gravity-affected loot entities.
- Added player pickup of nearby loot into inventory stacks.
- Added shared atlas support for item textures, textured inventory icons, and
  world-space loot billboards.
- Fixed dropped loot rendering so the item sprite stays above its ground
  contact point instead of dipping into terrain.
- Changed dropped loot sprite animation to rotate around the world Y axis.
- Made hotbar and inventory slots square in screen pixels and slightly larger
  by sizing slot width from the current window aspect ratio.
- Added inventory persistence to world metadata using item keys and stack
  counts, so `Save & Quit` and reload preserve collected items.
- Added Minecraft-style inventory manipulation for the current player
  inventory:
  - left click picks up, places, merges, or swaps full stacks
  - right click splits a stack or places one carried item
  - left drag distributes a carried stack over compatible slots
  - right drag places one carried item into each compatible slot
- Added a selected hotbar slot changed with the left and right arrow keys.
- Changed right-click placement to use the selected hotbar item only when that
  item has a placeable block target, consuming one item on successful
  placement.
- Added held left/right block interaction with an explicit repeat cadence, so
  holding right click continues placing selected hotbar blocks while placement
  remains valid and inventory remains available.
- Added a shaded player arm overlay when the selected hotbar slot is empty.
- Changed the in-hand overlay to render placeable block items as projected
  three-face block meshes and non-block items as angled item sprites.
- Fixed first-person held block and empty-hand geometry so both use stable
  three-face cuboid projections instead of sliver-like quads.
- Changed first-person held blocks to sample the visible south/east/top block
  textures.
- Tuned first-person held arm, block, and item overlay geometry toward a
  larger lower-right Minecraft-style framing.
- Fixed dropped loot spawning under a solid block above the break position by
  spawning drops in the newly opened block space and resolving loot collision
  per axis.
- Added cobblestone as a real block with generated texture coverage and made
  the cobblestone item place that block, fixing stone-drop placement.
- Adjusted the inventory overlay to separate the main inventory rows from the
  hotbar and use Minecraft-style gray slot framing.
- Added regression coverage for stack merging, overflow, registered block
  drops, loot spawning, pickup, inventory save round-tripping, square slot
  geometry, loot Y-axis render geometry, and item texture loading.
- Added regression coverage for inventory left-click, right-click, drag
  distribution, right-drag placement, selected-hotbar block placement, and
  cobblestone placement from a hotbar stack.
- Added regression coverage that held block and player arm overlay faces stay
  visible and non-degenerate.
- Added regression coverage for held block interaction cadence, lower-right
  held block framing, and loot from a block broken directly below another block
  falling normally.
- Added developer documentation for items, inventory, and loot.
- Added regression coverage for world metadata, saved player coordinates,
  chunk save/load round-tripping, and saved chunks overriding generation.
- Replaced the first-pass anonymous menu rectangles with screen-specific UI
  panels and bitmap text labels for:
  - main menu
  - manage worlds
  - configure new world
  - rename world
  - in-game pause overlay
- Added clickable pause overlay actions:
  - `Keep Playing`
  - `Save & Quit`
- Changed world saving to keep edited chunks dirty in memory during gameplay
  and flush them on `Save & Quit` or window close, avoiding frame drops from
  file writes during rendering/update.
- Buffered chunk-file writes into one byte buffer per chunk during flush.
- Replaced the active block, ore, resource, and sapling PNGs with matching
  default resource-pack artwork under `textures/blocks` and `textures/items`.
- Pre-tinted grayscale default grass-top and oak-leaf masks because the current
  renderer does not yet apply biome color maps.
- Registered distinct sandstone top/bottom/side textures and crafting-table
  top/bottom/front/side textures instead of using one face for every side.
- Changed inventory slot chrome and stack counts toward Minecraft-style gray
  beveled slots with dark count shadows.
- Changed inventory and crafting block item icons to render as small three-face
  block meshes using the same block textures as world rendering.
- Updated item/inventory rendering docs and added regression coverage for
  three-face inventory block icon geometry.
- Added `docs/generating_textures.md` with the default resource-pack import
  workflow, tint-mask handling, block/item/gui guidance, and verification
  expectations.
- Changed chunk saves from raw block IDs to keyed block palettes using
  `HCCNK002`, so saved worlds survive block registration order changes.
- Ignored legacy `HCCNK001` raw-ID chunk files and regenerated them from the
  world seed, preventing old saved chunks from showing diamond-ore trunks,
  log canopies, or wood-textured terrain after content IDs shift.
- Queued initially streamed chunks for save so `Save & Quit` writes the current
  keyed chunk format after loading or regenerating legacy chunks.
- Reworked the open inventory and crafting table UI layout around the original
  176 x 166 Minecraft container coordinates for closer panel, slot, crafting,
  result, and hotbar placement.
- Updated world-save and inventory rendering docs and added regression coverage
  for keyed chunk save/load and legacy raw-ID chunk regeneration.
- Tightened inventory slot bevel rendering to one-to-two-pixel source-style
  edges instead of percentage-thick nested rectangles.
- Reworked inventory block item icons to use a centered isometric block
  projection so block stacks no longer look stretched or skewed.
- Changed dropped placeable block loot to render as small rotating textured
  cubes while keeping non-block item loot as flat rotating sprites.
- Imported `destroy_stage_0.png` through `destroy_stage_9.png` into
  `textures/overlays` and changed block breaking to render those real staged
  damage textures instead of procedural crack pixels.
- Added a textured world-overlay render pipeline for block damage overlays and
  regression coverage for destroy-stage texture loading and stage selection.
- Added `textures/overlays/player_hand.png` as the first-person empty-hand
  texture and registered it in the renderer atlas.
- Changed the empty selected hotbar slot to render a textured lower-right
  player hand through the textured UI pass instead of a flat-color arm mesh.
- Enlarged and lowered selected block rendering so placeable hotbar blocks sit
  partially off-screen in the lower-right first-person view, closer to the
  Minecraft held-block framing.
- Added regression coverage for the player-hand overlay texture and updated
  held block/hand projection tests for the larger first-person framing.

## Verified

- `cargo fmt` after tightening inventory slots, block icons, block loot, and
  destroy overlays.
- `cargo test` after tightening inventory slots, block icons, block loot, and
  destroy overlays.
- `cargo fmt` after adding the textured first-person hand and selected-block
  overlay framing.
- `cargo test` after adding the textured first-person hand and selected-block
  overlay framing.
- `cargo fmt` after documenting texture generation, fixing keyed chunk saves,
  and reworking the inventory layout.
- `cargo test` after documenting texture generation, fixing keyed chunk saves,
  and reworking the inventory layout.
- `cargo fmt` after importing default-pack textures and adjusting inventory UI.
- `cargo test` after importing default-pack textures and adjusting inventory UI.
- `cargo fmt` after splitting content and windowed-client modules.
- `cargo check` after splitting content and windowed-client modules.
- `cargo test` after splitting content and windowed-client modules.
- `cargo fmt` after further splitting client-world spatial, collision, and
  loot modules.
- `cargo check` after further splitting client-world spatial, collision, and
  loot modules.
- `cargo test` after further splitting client-world spatial, collision, and
  loot modules.
- `cargo fmt` after adding inventory, loot, item textures, and docs.
- `cargo test` after adding inventory, loot, item textures, and docs.
- `cargo fmt` after fixing loot render geometry, square inventory slots, and
  inventory saves.
- `cargo test` after fixing loot render geometry, square inventory slots, and
  inventory saves.
- `cargo fmt` after adding inventory click/drag behavior, selected hotbar
  placement, and in-hand rendering.
- `cargo test` after adding inventory click/drag behavior, selected hotbar
  placement, and in-hand rendering.
- `cargo fmt` after improving held item rendering, inventory styling, and
  cobblestone placement.
- `cargo test --quiet` after improving held item rendering, inventory styling,
  and cobblestone placement.
- `cargo fmt` after correcting first-person held block and arm cuboid
  geometry.
- `cargo test --quiet` after correcting first-person held block and arm cuboid
  geometry.
- `cargo fmt` after adding held block interactions, first-person overlay
  tuning, and blocked-above loot fixes.
- `cargo test` after adding held block interactions, first-person overlay
  tuning, and blocked-above loot fixes.
- `quick_validate.py .agents/skills/create-item-texture`
- `cargo fmt`
- `cargo test`
- `cargo run -- preview`
- `printf 'q\n' | cargo run -- play`
- `cargo check`
- `cargo check` after adding PNG texture loading.
- `cargo test` after adding texture metadata, atlas sampling, and the texture
  path test.
- `cargo test` after adding world saves and menu-driven world loading.
- `cargo test` after fixing menu rendering and deferring save writes.
- `cargo test` after the Minecraft-style movement, sprint, and sneaking
  changes.
- `cargo test player_can_jump_onto_one_block_ledge -- --nocapture`
- `cargo check` and `cargo test` after fixing airborne one-block ledge jumps
  and adding movement documentation.
- `cargo run` launched the native game loop successfully and was stopped with
  Ctrl-C after smoke testing.
- `cargo run` launched successfully after the multi-chunk render and AZERTY
  input fixes.
- `cargo run` launched successfully after mouse capture and menu overlay changes.
- `cargo run` launched successfully after grounded movement and block
  interaction changes.
- `cargo run` launched successfully after player AABB collision, selected-block
  outline, and shading changes.
- `cargo run` launched successfully after texture atlas integration and was
  stopped with Ctrl-C after startup smoke testing.
- `cargo test` after fixing texture path resolution and safe spawn placement.
- `cargo run` launched successfully after the texture/spawn regression fix and
  was stopped with Ctrl-C after startup smoke testing.
- `cargo test` after fixing the shader pipeline swap and atlas lifetime.
- `cargo run` launched successfully after the textured shader pipeline fix and
  was stopped with Ctrl-C after startup smoke testing.
- `cargo fmt` after adding chunk-boundary visible-face culling.
- `cargo test` after adding chunk-boundary visible-face culling.
- `cargo fmt` after adding simple infinite-world chunk streaming.
- `cargo test` after adding simple infinite-world chunk streaming.
- `cargo fmt` after switching to per-chunk render buffers and dirty remeshing.
- `cargo check` after switching to per-chunk render buffers and dirty remeshing.
- `cargo test` after switching to per-chunk render buffers and dirty remeshing.
- `cargo fmt` after adding biome terrain and tree generation.
- Verified oak log and oak leaves texture PNGs are `16 x 16` RGBA images.
- `cargo test` after adding biome terrain and tree generation.
- `cargo fmt` after smoothing terrain and biome transitions.
- `cargo test` after smoothing terrain and biome transitions.
- `cargo run -- preview` after smoothing terrain and biome transitions.
- Regenerated oak log and oak leaves textures with the `create-block-texture`
  skill.
- Verified oak log and oak leaves texture PNGs are `16 x 16` RGBA images after
  regeneration.
- `cargo fmt` after texture, camera, crosshair, and outline fixes.
- `cargo test` after texture, camera, crosshair, and outline fixes.
- Regenerated sand, sandstone, bedrock, and oak leaves textures with the
  `create-block-texture` skill.
- Verified oak leaves, sand, sandstone, and bedrock texture PNGs are `16 x 16`
  RGBA images.
- `cargo fmt` after desert, bedrock, sea-level, and leaf transparency changes.
- `cargo test` after desert, bedrock, sea-level, and leaf transparency changes.
- `cargo fmt` after terrain relief and mountain stone exposure tuning.
- `cargo test` after terrain relief and mountain stone exposure tuning.
- `cargo run -- preview` after terrain relief and mountain stone exposure
  tuning.
- `cargo fmt` after mountain surface and underground render filtering fixes.
- `cargo test` after mountain surface and underground render filtering fixes.
- `cargo run -- preview` after mountain surface and underground render
  filtering fixes.
- `cargo run` launched successfully after terrain back-face culling changes and
  was stopped with Ctrl-C after startup validation.
- `cargo fmt` after mesh winding and runtime chunk-load throttling fixes.
- `cargo test` after mesh winding and runtime chunk-load throttling fixes.
- `cargo run -- preview` after mesh winding and runtime chunk-load throttling
  fixes.
- `cargo run` launched successfully after mesh winding and runtime chunk-load
  throttling fixes and was stopped with Ctrl-C after startup validation.
- Generated coal ore, iron ore, gold ore, and diamond ore textures with the
  `create-block-texture` skill.
- `cargo fmt` after remesh throttling, texture coverage, and placement fixes.
- `cargo test` after remesh throttling, texture coverage, and placement fixes.
- `cargo run -- preview` after remesh throttling, texture coverage, and
  placement fixes.
- `cargo run` launched successfully after remesh throttling, texture coverage,
  and placement fixes and was stopped with Ctrl-C after startup validation.
- `cargo fmt` after adding hardness-based survival block breaking.
- `cargo test` after adding hardness-based survival block breaking.
- `cargo fmt` after replacing the break overlay and extracting windowed modules.
- `cargo test` after replacing the break overlay and extracting windowed modules.
- `cargo fmt` after the windowed-client responsibility split.
- `cargo test` after the windowed-client responsibility split.

## Next Concrete Steps

1. Add a cross-chunk decoration/feature placement pass so trees and future
   structures can span chunk boundaries.
2. Add texture assets and metadata for coal, iron, gold, and diamond ore.
3. Add more complete swept collision and step-up behavior.
4. Add lightweight in-game render diagnostics for mesh rebuild counts, dirty
   chunks, frame time, and loaded chunk count.
5. Replace face-per-block meshing with greedy meshing or atlas-aware batching
   once render distance grows.
6. Replace temporary value noise with the planned `noise` crate.
7. Add an unload/save policy for generated chunks before render distance grows
   much further.
8. Add a small in-game debug overlay showing selected block key, player chunk,
   loaded chunk count, and selected block position.

## Notes

- The project directory name contains a dot, so Cargo was initialized with
  `--name humancraft`.
- Cargo VCS initialization was disabled because the sandbox would not allow
  creating `.git`.
- Rendering now samples PNG textures for registered non-air blocks in windowed
  mode. A test enforces that gameplay blocks do not silently use the missing
  texture fallback.
- The current atlas is intentionally simple: all block faces are expected to be
  16 x 16 RGBA PNGs.
- Tree origins are currently kept inside chunk margins so generated trees are
  complete within one chunk. This should become a cross-chunk feature placement
  system before larger structures are added.
- Biome region size and transition width are engine settings supplied by
  content. Current HumanCraft values favor stable regions over frequent biome
  changes.
