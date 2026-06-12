//! HumanCraft engine library.
//!
//! The library is split into reusable engine systems and project-specific
//! content registration. Rendering, input, and persistence will sit on top of
//! these systems instead of being embedded inside them.

pub mod app;
pub mod content;
pub mod debug;
pub mod engine;
