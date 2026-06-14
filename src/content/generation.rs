//! HumanCraft overworld generation content.

use crate::content::BlockIds;
use crate::engine::world::generation::GenerationPipeline;
use crate::engine::world::generation::bedrock::BedrockStage;
use crate::engine::world::generation::biome::{
    BiomeDefinition, BiomeSource, ExposedSurfaceRule, TerrainLayer,
};
use crate::engine::world::generation::ore::{OreDefinition, OreStage};
use crate::engine::world::generation::terrain::TerrainStage;
use crate::engine::world::generation::tree::{TreeDefinition, TreeStage};

pub fn default_generation_pipeline(blocks: BlockIds) -> GenerationPipeline {
    let biome_source = overworld_biome_source(blocks);
    GenerationPipeline::new()
        .add_stage(TerrainStage::new(biome_source.clone()))
        .add_stage(OreStage::new(vec![
            OreDefinition {
                key: "humancraft:coal_ore".to_string(),
                block: blocks.coal_ore,
                replaces: blocks.stone,
                min_y: 4,
                max_y: 96,
                threshold: 0.965,
            },
            OreDefinition {
                key: "humancraft:iron_ore".to_string(),
                block: blocks.iron_ore,
                replaces: blocks.stone,
                min_y: 4,
                max_y: 64,
                threshold: 0.982,
            },
            OreDefinition {
                key: "humancraft:gold_ore".to_string(),
                block: blocks.gold_ore,
                replaces: blocks.stone,
                min_y: 4,
                max_y: 32,
                threshold: 0.992,
            },
            OreDefinition {
                key: "humancraft:diamond_ore".to_string(),
                block: blocks.diamond_ore,
                replaces: blocks.stone,
                min_y: 4,
                max_y: 16,
                threshold: 0.996,
            },
        ]))
        .add_stage(BedrockStage::new(blocks.bedrock))
        .add_stage(TreeStage::new(
            biome_source,
            vec![
                TreeDefinition::new("humancraft:oak_tree", blocks.oak_log, blocks.oak_leaves)
                    .grow_on([blocks.grass])
                    .replaceable([blocks.air, blocks.oak_leaves])
                    .biomes(["humancraft:forest"])
                    .shape(4, 6, 2)
                    .density(0.055),
                TreeDefinition::new(
                    "humancraft:plains_oak_tree",
                    blocks.oak_log,
                    blocks.oak_leaves,
                )
                .grow_on([blocks.grass])
                .replaceable([blocks.air, blocks.oak_leaves])
                .biomes(["humancraft:plains"])
                .shape(4, 5, 2)
                .density(0.008),
            ],
        ))
}

pub(crate) fn overworld_biome_source(blocks: BlockIds) -> BiomeSource {
    BiomeSource::new(vec![
        BiomeDefinition::new("humancraft:plains", blocks.grass, blocks.dirt, blocks.stone)
            .terrain(66, 9, 4, 80, 24)
            .relief(2.5, 1.5, 56),
        BiomeDefinition::new("humancraft:forest", blocks.grass, blocks.dirt, blocks.stone)
            .terrain(67, 12, 4, 78, 22)
            .relief(3.5, 2.0, 48),
        BiomeDefinition::new(
            "humancraft:mountains",
            blocks.grass,
            blocks.dirt,
            blocks.stone,
        )
        .terrain(84, 44, 3, 104, 20)
        .relief(12.0, 12.0, 52)
        .exposed_surface(ExposedSurfaceRule::new(blocks.stone, None, 3)),
        BiomeDefinition::new(
            "humancraft:desert",
            blocks.sand,
            blocks.sandstone,
            blocks.stone,
        )
        .terrain(66, 13, 4, 86, 20)
        .relief(5.5, 5.0, 44)
        .layers([
            TerrainLayer::new(blocks.sand, 5),
            TerrainLayer::new(blocks.sandstone, 6),
        ]),
    ])
    .with_min_region_chunks(10)
    .with_transition_chunks(2)
}
