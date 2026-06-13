# Agent Notes

Read the project documents before making gameplay or engine changes:

- `PRD.md`
- `DEV_PHILOSOPHY.md`
- `PROGRESS.md`
- `CHANGELOG.md`

For player movement work, also read:

- `docs/player/controls.md`
- `docs/developer/systems/player_movement.md`

Movement currently lives in `src/app/windowed.rs`. Preserve the separation
between gameplay logic and rendering where practical, but do not introduce a
new entity framework just to adjust the current controller. Keep movement
constants explicit, add regression tests for player-facing behavior, and update
player, developer, progress, and changelog documentation when controls or
movement mechanics change.
