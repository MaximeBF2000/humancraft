# Windowed Client Structure

The windowed client is still the main development harness for gameplay, input,
rendering, and save integration. Keep its modules split by ownership so new
features do not accumulate in the event-loop shell.

## Modules

- `src/app/windowed.rs`: winit/wgpu app shell, render pipelines, menu flow,
  frame update orchestration, and draw submission.
- `src/app/windowed/client_world.rs`: loaded client chunks, save-backed chunk
  streaming, raycasts, block edits, and chunk mesh preparation.
- `src/app/windowed/player_collision.rs`: safe spawn search, player AABB
  collision checks, ground support checks, and occupied-block placement
  rejection.
- `src/app/windowed/spatial.rs`: render/world coordinate conversion, world block
  positions, player AABB construction, and chunk-neighbor dirtying helpers.
- `src/app/windowed/loot.rs`: dropped item spawning, gravity/drag updates, block
  collision, rotation, and pickup into the player inventory.
- `src/app/windowed/inventory_interaction.rs`: player inventory click, drag,
  cursor-stack, and save conversion helpers.
- `src/app/windowed/constants.rs`: explicit windowed-client tuning constants
  for movement, chunk budgets, inventory layout, and loot behavior.

## Rules

- Keep renderer-owned code responsible for GPU resources and draw ordering.
- Keep world mutation in `ClientWorld` or an engine system, not in mesh/UI
  builders.
- Keep inventory stack semantics in `inventory_interaction.rs`; render code
  should only ask for slot rectangles and icon meshes.
- Keep constants explicit. If a value affects player-facing behavior, update
  player and developer docs when changing it.
- Prefer moving cohesive private helpers into a module before adding more
  behavior to `windowed.rs`.

## Future Split Candidates

- Move texture atlas loading and texture-key resolution into a renderer asset
  module.
- Move menu state and text entry helpers into a menu module.
- Move UI mesh construction, glyph drawing, crosshair, and held-item overlay
  code into a UI renderer module.
- Move player camera/movement to a dedicated controller module once the
  controller boundary is clearer.
- Move chunk streaming into its own module if save-backed loading grows beyond
  the current simple render-distance policy.
