# Terminal Playtest

## Purpose

The terminal playtest gives the project a runnable interaction loop before the
windowed renderer exists.

## Responsibilities

- Render a generated chunk as a small ASCII height map.
- Track a player position inside one chunk.
- Allow simple movement.
- Allow mining and placing the surface block.

## Inputs

- Generated `Chunk`
- `BlockRegistry`
- Starter `BlockIds`
- Line-based terminal commands

## Outputs

- Updated in-memory chunk state.
- ASCII map after each command.

## Dependencies

- Chunk system.
- World generation.
- Block registry.

## Extension Points

- Add inventory checks before placing blocks.
- Add block selection separate from player position.
- Replace with a windowed client once rendering is available.

## Known Limitations

- It only operates inside one chunk.
- It is line-based, not real-time.
- It has no collisions, health, hunger, inventory, lighting, or persistence.
