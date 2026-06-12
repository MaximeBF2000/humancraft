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
- Optional outside-neighbor block lookup for cross-chunk face culling.

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
- Add chunk-section dirty tracking so border edits can remesh only affected
  chunks.

## Known Limitations

- This is not greedy meshing yet.
- Quads do not include UVs, normals, lighting, or material IDs yet.
