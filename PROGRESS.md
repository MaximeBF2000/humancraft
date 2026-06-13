# HumanCraft Progress

Last updated: 2026-06-13

## Done

- Read `PRD.md` and `DEV_PHILOSOPHY.md`.
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

## Verified

- `cargo fmt`
- `cargo test`
- `cargo run -- preview`
- `printf 'q\n' | cargo run -- play`
- `cargo check`
- `cargo check` after adding PNG texture loading.
- `cargo test` after adding texture metadata, atlas sampling, and the texture
  path test.
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

## Next Concrete Steps

1. Add a cross-chunk decoration/feature placement pass so trees and future
   structures can span chunk boundaries.
2. Add texture assets and metadata for coal, iron, gold, and diamond ore.
3. Add hotbar/inventory-backed placement instead of infinite dirt.
4. Add more complete swept collision and step-up behavior.
5. Add lightweight in-game render diagnostics for mesh rebuild counts, dirty
   chunks, frame time, and loaded chunk count.
6. Replace face-per-block meshing with greedy meshing or atlas-aware batching
   once render distance grows.
7. Replace temporary value noise with the planned `noise` crate.
8. Add an unload/save policy for generated chunks before render distance grows
   much further.
9. Add a small in-game debug overlay showing selected block key, player chunk,
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
