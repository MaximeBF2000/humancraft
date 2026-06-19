# Chunk System

## Purpose

Chunks own block states for a fixed area of the world.

## Responsibilities

- Store `BlockState` values for a `16 x 16 x 256` volume.
- Provide bounded get/set access.
- Expose immutable block storage for systems that need to scan a chunk.
- Preserve id-only accessors for systems that do not care about state.

## Inputs

- `ChunkPosition`
- `BlockPosition`
- `BlockId`
- `BlockState`

## Outputs

- Stored block IDs.
- Stored block states.
- Bounds errors for invalid writes.

## Dependencies

- `BlockId`

## Extension Points

- Add light values.
- Add more compact state storage if block-state volume memory becomes a
  problem.
- Add serialization helpers.
- Add chunk sectioning if performance requires it.

## Known Limitations

- Chunks currently store one flat `Vec<BlockState>`.
- Neighbor chunk access is not represented yet.
- No lighting or entities are stored inside chunks yet.
