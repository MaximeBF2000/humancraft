//! Mesh generation systems.
//!
//! Purpose:
//! Convert world data into renderer-neutral mesh data. GPU buffers, shaders,
//! and draw calls belong in a future renderer that consumes these meshes.

pub mod chunk_mesher;
