# World Generation System

## Purpose

World generation composes independent stages into deterministic chunk
generation.

## Current Stages

- `TerrainStage`: fills stone, dirt, and grass using deterministic value noise.
- `OreStage`: places arbitrary ore definitions inside matching host blocks.

## Responsibilities

- Start each chunk filled with air.
- Apply stages in configured order.
- Keep stage behavior generic.
- Let content supply block IDs and ore distribution definitions.

## Inputs

- `GenerationContext`
- `ChunkPosition`
- Registered block IDs supplied by content bootstrap.

## Outputs

- Populated `Chunk`

## Dependencies

- Chunk system.
- Block IDs from block registry bootstrap.

## Extension Points

- Replace temporary value noise with the planned `noise` crate.
- Add biome, cave, water, tree, decoration, and structure stages.
- Move generation settings to serialized data files.

## Known Limitations

- Noise is intentionally simple and dependency-free.
- Terrain does not interpolate between sample points.
- Ore distribution is value-noise based, not vein based.
- There are no biomes, caves, water, trees, or decorations yet.
