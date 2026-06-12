use humancraft::app::windowed::run_windowed_game;
use humancraft::content::{bootstrap_content, default_generation_pipeline};
use humancraft::debug::playtest::run_playtest;
use humancraft::debug::preview::{build_chunk_preview, write_preview_files};
use humancraft::engine::mesh::chunk_mesher::ChunkMesher;
use humancraft::engine::world::generation::GenerationContext;
use humancraft::engine::world::{BlockPosition, CHUNK_HEIGHT, ChunkPosition};
use std::env;
use std::path::Path;

fn main() {
    let command = env::args().nth(1).unwrap_or_else(|| "dev".to_string());
    if command == "dev" {
        run_windowed_game();
        return;
    }

    let content = bootstrap_content().expect("default content must register cleanly");
    let pipeline = default_generation_pipeline(content.block_ids);
    let context = GenerationContext {
        seed: 1,
        air: content.block_ids.air,
    };
    let mut chunk = pipeline.generate_chunk(ChunkPosition { x: 0, z: 0 }, &context);
    let mesh = ChunkMesher.mesh_chunk(&chunk, &content.blocks);
    let solid_blocks = chunk
        .blocks()
        .iter()
        .filter(|block| **block != content.block_ids.air)
        .count();
    let surface_y = (0..CHUNK_HEIGHT)
        .rev()
        .find(|y| chunk.block(BlockPosition { x: 0, y: *y, z: 0 }) != Some(content.block_ids.air))
        .unwrap_or(0);

    match command.as_str() {
        "preview" => {
            let preview_dir = Path::new("out/preview");
            write_preview_files(preview_dir, &chunk, &content.blocks, &mesh)
                .expect("preview files should be writable");
            let preview = build_chunk_preview(&chunk, &content.blocks);

            println!("HumanCraft preview");
            println!("registered blocks: {}", content.blocks.len());
            println!("registered items: {}", content.items.len());
            println!("generation stages: {}", pipeline.stage_names().join(", "));
            println!("sample chunk solid blocks: {solid_blocks}");
            println!("sample surface height at local 0,0: {surface_y}");
            println!("mesh quads: {}", mesh.quads.len());
            println!("mesh triangles: {}", mesh.triangle_count());
            println!("preview files:");
            println!("  {}", preview_dir.join("heightmap.txt").display());
            println!("  {}", preview_dir.join("heightmap.ppm").display());
            println!("  {}", preview_dir.join("chunk.obj").display());
            println!();
            println!("{}", preview.ascii_map);
        }
        "stats" => {
            println!("HumanCraft engine bootstrap");
            println!("registered blocks: {}", content.blocks.len());
            println!("registered items: {}", content.items.len());
            println!("generation stages: {}", pipeline.stage_names().join(", "));
            println!("sample chunk solid blocks: {solid_blocks}");
            println!("sample surface height at local 0,0: {surface_y}");
            println!("mesh quads: {}", mesh.quads.len());
            println!("mesh triangles: {}", mesh.triangle_count());
        }
        "play" => {
            run_playtest(&mut chunk, &content.blocks, content.block_ids)
                .expect("terminal playtest should run");
        }
        _ => {
            eprintln!("Unknown command: {command}");
            eprintln!("Usage: cargo run -- [preview|stats|play]");
            std::process::exit(2);
        }
    }
}
