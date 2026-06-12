# Chunk System

## Purpose

Chunks own block IDs for a fixed area of the world.

## Responsibilities

- Store block IDs for a `16 x 16 x 256` volume.
- Provide bounded get/set access.
- Expose immutable block storage for systems that need to scan a chunk.

## Inputs

- `ChunkPosition`
- `BlockPosition`
- `BlockId`

## Outputs

- Stored block IDs.
- Bounds errors for invalid writes.

## Dependencies

- `BlockId`

## Extension Points

- Add light values.
- Add compact metadata storage.
- Add serialization helpers.
- Add chunk sectioning if performance requires it.

## Known Limitations

- Chunks currently store one flat `Vec<BlockId>`.
- Neighbor chunk access is not represented yet.
- No lighting, metadata, entities, or persistence are stored yet.
