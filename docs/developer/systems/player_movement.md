# Player Movement System

## Purpose

The windowed client currently owns the player movement controller in
`src/app/windowed.rs`. The controller is intentionally small, but the mechanics
should remain explicit and test-covered because movement feel is player-facing.

## Current Inputs

- `InputState`: logical `ZQSD` movement, `Space` jump, `Shift` sneak, and
  double-`Z` sprint detection.
- `ClientWorld`: collision, ground support, spawn safety, and block occupancy.
- `Camera`: temporary player body and view state until a fuller entity system
  exists.

## Mechanics

Movement runs on a fixed 20 Hz physics tick. HumanCraft uses Minecraft-style
mechanics, but the current values are tuned slightly more forgiving than strict
Java Edition movement because early playtests found one-block jumps too tight.

- Horizontal movement uses per-tick acceleration.
- Ground friction is applied after horizontal movement.
- Air control uses lower acceleration than grounded movement.
- Diagonal input is normalized before acceleration.
- Jumping applies a vertical impulse, then gravity and air damping each tick.
- On the tick where a jump begins, the initial jump impulse moves the player
  before gravity and drag are applied. This preserves one-block jump clearance.
- Sprinting multiplies acceleration and adds a forward boost when jumping.
- Sneaking slows acceleration, lowers eye height, and blocks unsupported
  horizontal movement from carrying the player off an edge.
- Horizontal collision preserves airborne velocity so a player can keep pushing
  toward a one-block ledge while rising through a jump.

## Constants

The movement constants live near the top of `src/app/windowed.rs` beside the
player dimensions. They currently use Minecraft-style units with HumanCraft
playability tuning:

- 20 Hz tick rate
- 0.6 block player width
- 1.8 block player height
- 1.62 standing eye height
- 1.54 sneaking eye height
- 0.13 ground acceleration
- 0.03 air acceleration
- 0.546 ground friction
- 0.46 jump velocity
- 0.08 gravity per tick
- 0.98 vertical air drag
- 1.3 sprint multiplier
- 0.3 sneak multiplier

## Collision Notes

Collision uses the player AABB and moves each axis separately. Grounded
horizontal movement can attempt a small step-up, but clearing a full block is
handled by jumping. Sneak edge protection probes below the player AABB after a
horizontal move and rejects the move when no supporting block remains.

## Tests

Movement regressions live in the `src/app/windowed.rs` test module. Keep tests
focused on observable behavior, especially:

- logical `ZQSD` input
- double-`Z` sprint timing
- Shift sneaking edge protection
- one-block jump clearance
- player AABB collision
