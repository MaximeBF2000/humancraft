# Chunk Meshing System

## Purpose

The chunk mesher converts chunk block IDs into renderer-neutral visible faces.

## Responsibilities

- Iterate over blocks in a chunk.
- Skip invisible or non-solid air-like blocks.
- Emit faces where a neighboring block does not occlude the face.
- Keep GPU, texture, and draw-call details out of the world layer.

## Inputs

- `Chunk`
- `BlockRegistry`

## Outputs

- `ChunkMesh`
- `MeshQuad`

## Dependencies

- Chunk system.
- Block definitions for solidity and transparency.

## Extension Points

- Replace face-per-block output with greedy meshing.
- Add texture atlas coordinates.
- Add lighting values.
- Add ambient occlusion data.
- Add neighbor-chunk lookups so borders can be culled against adjacent chunks.

## Known Limitations

- This is not greedy meshing yet.
- Chunk borders are always considered exposed.
- Quads do not include UVs, normals, lighting, or material IDs yet.
