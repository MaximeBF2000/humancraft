//! Preview artifact writers.

use std::fmt::Write as _;
use std::fs;
use std::io;
use std::path::Path;

use crate::engine::mesh::chunk_mesher::ChunkMesh;
use crate::engine::world::{BlockPosition, BlockRegistry, CHUNK_HEIGHT, CHUNK_SIZE, Chunk};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChunkPreview {
    pub height_map: [[usize; CHUNK_SIZE]; CHUNK_SIZE],
    pub ascii_map: String,
}

pub fn build_chunk_preview(chunk: &Chunk, blocks: &BlockRegistry) -> ChunkPreview {
    let mut height_map = [[0; CHUNK_SIZE]; CHUNK_SIZE];
    let mut ascii_map = String::new();

    for z in 0..CHUNK_SIZE {
        for x in 0..CHUNK_SIZE {
            let surface = surface_at(chunk, blocks, x, z).unwrap_or(0);
            height_map[z][x] = surface;
            ascii_map.push(height_char(surface));
        }
        ascii_map.push('\n');
    }

    ChunkPreview {
        height_map,
        ascii_map,
    }
}

pub fn write_preview_files(
    directory: &Path,
    chunk: &Chunk,
    blocks: &BlockRegistry,
    mesh: &ChunkMesh,
) -> io::Result<()> {
    fs::create_dir_all(directory)?;
    let preview = build_chunk_preview(chunk, blocks);
    fs::write(directory.join("heightmap.txt"), &preview.ascii_map)?;
    fs::write(directory.join("heightmap.ppm"), heightmap_ppm(&preview))?;
    fs::write(directory.join("chunk.obj"), mesh_to_obj(mesh, blocks))?;
    Ok(())
}

pub fn mesh_to_obj(mesh: &ChunkMesh, blocks: &BlockRegistry) -> String {
    let mut output = String::new();
    output.push_str("# HumanCraft chunk mesh preview\n");
    output.push_str("# One quad per visible block face; renderer optimization comes later.\n");

    for quad in &mesh.quads {
        let name = blocks
            .get(quad.block)
            .map(|definition| definition.key.as_str())
            .unwrap_or("unknown:block");
        let _ = writeln!(output, "o {}", sanitize_obj_name(name));
        for vertex in quad.vertices {
            let _ = writeln!(
                output,
                "v {:.3} {:.3} {:.3}",
                vertex[0], vertex[1], vertex[2]
            );
        }
    }

    for index in 0..mesh.quads.len() {
        let first = index * 4 + 1;
        let _ = writeln!(
            output,
            "f {} {} {} {}",
            first,
            first + 1,
            first + 2,
            first + 3
        );
    }

    output
}

fn heightmap_ppm(preview: &ChunkPreview) -> String {
    let mut output = String::new();
    let _ = writeln!(output, "P3");
    let _ = writeln!(output, "{} {}", CHUNK_SIZE, CHUNK_SIZE);
    let _ = writeln!(output, "255");

    for z in 0..CHUNK_SIZE {
        for x in 0..CHUNK_SIZE {
            let height = preview.height_map[z][x] as u8;
            let green = 80u8.saturating_add(height.saturating_mul(2));
            let _ = write!(output, "40 {} 40 ", green);
        }
        output.push('\n');
    }

    output
}

fn surface_at(chunk: &Chunk, blocks: &BlockRegistry, x: usize, z: usize) -> Option<usize> {
    (0..CHUNK_HEIGHT).rev().find(|y| {
        chunk
            .block(BlockPosition { x, y: *y, z })
            .and_then(|block| blocks.get(block))
            .map(|definition| definition.solid)
            .unwrap_or(false)
    })
}

fn height_char(height: usize) -> char {
    match height {
        0..=58 => '.',
        59..=64 => ',',
        65..=70 => '^',
        71..=76 => 'A',
        77..=82 => 'M',
        _ => '#',
    }
}

fn sanitize_obj_name(name: &str) -> String {
    name.chars()
        .map(|character| match character {
            'a'..='z' | 'A'..='Z' | '0'..='9' | '_' | '-' => character,
            _ => '_',
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::content::{bootstrap_content, default_generation_pipeline};
    use crate::engine::mesh::chunk_mesher::ChunkMesher;
    use crate::engine::world::ChunkPosition;
    use crate::engine::world::generation::GenerationContext;

    #[test]
    fn preview_contains_one_line_per_chunk_row() {
        let content = bootstrap_content().unwrap();
        let pipeline = default_generation_pipeline(content.block_ids);
        let chunk = pipeline.generate_chunk(
            ChunkPosition { x: 0, z: 0 },
            &GenerationContext {
                seed: 1,
                air: content.block_ids.air,
            },
        );

        let preview = build_chunk_preview(&chunk, &content.blocks);

        assert_eq!(preview.ascii_map.lines().count(), CHUNK_SIZE);
        assert!(
            preview
                .ascii_map
                .lines()
                .all(|line| line.len() == CHUNK_SIZE)
        );
    }

    #[test]
    fn obj_export_contains_faces() {
        let content = bootstrap_content().unwrap();
        let pipeline = default_generation_pipeline(content.block_ids);
        let chunk = pipeline.generate_chunk(
            ChunkPosition { x: 0, z: 0 },
            &GenerationContext {
                seed: 1,
                air: content.block_ids.air,
            },
        );
        let mesh = ChunkMesher.mesh_chunk(&chunk, &content.blocks);

        let obj = mesh_to_obj(&mesh, &content.blocks);

        assert!(obj.contains("v "));
        assert!(obj.contains("f "));
    }
}
