//! Terminal playtest loop.
//!
//! This is a temporary, renderer-free way to exercise world state. It should
//! stay small and disappear once a real windowed client exists.

use std::io::{self, Write};

use crate::content::BlockIds;
use crate::engine::world::{BlockPosition, BlockRegistry, CHUNK_HEIGHT, CHUNK_SIZE, Chunk};

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub struct PlaytestPlayer {
    pub x: usize,
    pub z: usize,
}

impl Default for PlaytestPlayer {
    fn default() -> Self {
        Self {
            x: CHUNK_SIZE / 2,
            z: CHUNK_SIZE / 2,
        }
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum PlaytestCommand {
    North,
    South,
    East,
    West,
    Mine,
    Place,
    Quit,
    Help,
    Unknown,
}

pub fn run_playtest(
    chunk: &mut Chunk,
    blocks: &BlockRegistry,
    block_ids: BlockIds,
) -> io::Result<()> {
    let mut player = PlaytestPlayer::default();
    println!("HumanCraft terminal playtest");
    println!("Commands: w/a/s/d move, m mine surface, p place dirt, h help, q quit");

    loop {
        println!("{}", render_playtest_map(chunk, blocks, player));
        print!("> ");
        io::stdout().flush()?;

        let mut input = String::new();
        if io::stdin().read_line(&mut input)? == 0 {
            break;
        }

        match parse_command(&input) {
            PlaytestCommand::North => player.z = player.z.saturating_sub(1),
            PlaytestCommand::South => player.z = (player.z + 1).min(CHUNK_SIZE - 1),
            PlaytestCommand::East => player.x = (player.x + 1).min(CHUNK_SIZE - 1),
            PlaytestCommand::West => player.x = player.x.saturating_sub(1),
            PlaytestCommand::Mine => mine_surface(chunk, blocks, block_ids.air, player.x, player.z),
            PlaytestCommand::Place => {
                place_surface(chunk, blocks, block_ids.dirt, player.x, player.z)
            }
            PlaytestCommand::Help => {
                println!("w/a/s/d move, m mine top solid block, p place dirt above ground, q quit");
            }
            PlaytestCommand::Quit => break,
            PlaytestCommand::Unknown => println!("Unknown command. Type h for help."),
        }
    }

    Ok(())
}

pub fn render_playtest_map(
    chunk: &Chunk,
    blocks: &BlockRegistry,
    player: PlaytestPlayer,
) -> String {
    let mut output = String::new();
    for z in 0..CHUNK_SIZE {
        for x in 0..CHUNK_SIZE {
            if player.x == x && player.z == z {
                output.push('@');
            } else {
                let height = surface_at(chunk, blocks, x, z).unwrap_or(0);
                output.push(height_char(height));
            }
        }
        output.push('\n');
    }
    let height = surface_at(chunk, blocks, player.x, player.z).unwrap_or(0);
    output.push_str(&format!(
        "player: x={} y={} z={}\n",
        player.x,
        height + 1,
        player.z
    ));
    output
}

pub fn parse_command(input: &str) -> PlaytestCommand {
    match input.trim().to_ascii_lowercase().as_str() {
        "w" | "north" => PlaytestCommand::North,
        "s" | "south" => PlaytestCommand::South,
        "d" | "east" => PlaytestCommand::East,
        "a" | "west" => PlaytestCommand::West,
        "m" | "mine" => PlaytestCommand::Mine,
        "p" | "place" => PlaytestCommand::Place,
        "q" | "quit" => PlaytestCommand::Quit,
        "h" | "help" => PlaytestCommand::Help,
        _ => PlaytestCommand::Unknown,
    }
}

pub fn mine_surface(
    chunk: &mut Chunk,
    blocks: &BlockRegistry,
    air: crate::engine::world::BlockId,
    x: usize,
    z: usize,
) {
    if let Some(y) = surface_at(chunk, blocks, x, z) {
        let Some(block) = chunk.block(BlockPosition { x, y, z }) else {
            return;
        };
        if blocks
            .get(block)
            .map(|definition| definition.has_tag("unbreakable"))
            .unwrap_or(false)
        {
            return;
        }
        chunk
            .set_block(BlockPosition { x, y, z }, air)
            .expect("surface positions stay inside chunk bounds");
    }
}

pub fn place_surface(
    chunk: &mut Chunk,
    blocks: &BlockRegistry,
    block: crate::engine::world::BlockId,
    x: usize,
    z: usize,
) {
    let y = surface_at(chunk, blocks, x, z).unwrap_or(0) + 1;
    if y < CHUNK_HEIGHT {
        chunk
            .set_block(BlockPosition { x, y, z }, block)
            .expect("surface positions stay inside chunk bounds");
    }
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::content::{bootstrap_content, default_generation_pipeline};
    use crate::engine::world::ChunkPosition;
    use crate::engine::world::generation::GenerationContext;

    #[test]
    fn parses_basic_commands() {
        assert_eq!(parse_command("w\n"), PlaytestCommand::North);
        assert_eq!(parse_command("mine"), PlaytestCommand::Mine);
        assert_eq!(parse_command("q"), PlaytestCommand::Quit);
        assert_eq!(parse_command("???"), PlaytestCommand::Unknown);
    }

    #[test]
    fn mine_and_place_change_surface_height() {
        let content = bootstrap_content().unwrap();
        let pipeline = default_generation_pipeline(content.block_ids);
        let mut chunk = pipeline.generate_chunk(
            ChunkPosition { x: 0, z: 0 },
            &GenerationContext {
                seed: 1,
                air: content.block_ids.air,
            },
        );
        let player = PlaytestPlayer::default();
        let before = surface_at(&chunk, &content.blocks, player.x, player.z).unwrap();

        mine_surface(
            &mut chunk,
            &content.blocks,
            content.block_ids.air,
            player.x,
            player.z,
        );
        let after_mine = surface_at(&chunk, &content.blocks, player.x, player.z).unwrap();
        place_surface(
            &mut chunk,
            &content.blocks,
            content.block_ids.dirt,
            player.x,
            player.z,
        );
        let after_place = surface_at(&chunk, &content.blocks, player.x, player.z).unwrap();

        assert_eq!(after_mine + 1, before);
        assert_eq!(after_place, before);
    }

    #[test]
    fn map_marks_player_position() {
        let content = bootstrap_content().unwrap();
        let pipeline = default_generation_pipeline(content.block_ids);
        let chunk = pipeline.generate_chunk(
            ChunkPosition { x: 0, z: 0 },
            &GenerationContext {
                seed: 1,
                air: content.block_ids.air,
            },
        );

        let map = render_playtest_map(&chunk, &content.blocks, PlaytestPlayer::default());

        assert!(map.contains('@'));
        assert!(map.contains("player:"));
    }
}
