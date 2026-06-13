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

## Renderer Notes

- Mesh quads are wound counter-clockwise when viewed from outside the block.
  The windowed renderer uses `FrontFace::Ccw` with back-face culling for
  terrain.
- The windowed renderer also applies a temporary finite-world preview filter:
  outer loaded-patch boundary faces and the Y 0 bottom boundary are hidden so
  the current finite loaded area does not render as an artificial shell.
  Legitimate underground side and ceiling faces remain visible after block
  edits.
- Runtime chunk streaming uses a small per-frame remesh/upload budget for dirty
  chunks. Missing chunks can be generated ahead of their GPU buffers; the
  renderer queues dirty chunk positions and rebuilds the closest pending chunk
  meshes first.

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
- The finite-world preview filter is a renderer-side stopgap until chunk
  streaming, caves, and underground visibility are modeled more completely.
