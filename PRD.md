# Project PRD + TAD

# Project: Voxel Survival Game

Name: HumanCraft
Version: 0.1

# Vision

This project is the beginning of a long-term voxel sandbox game inspired by Minecraft Beta-era gameplay.

The objective is **not to create a strict clone**, but to build a highly extensible game and engine that initially reproduces the core survival experience while allowing future evolution, new mechanics, and eventually modding support.

The game should provide:

- Exploration
- Resource gathering
- Mining progression
- Building
- Crafting
- Farming
- Hunger and survival
- Animals and mobs
- Long-term progression

Every system must be designed with future expansion in mind.

# Core Design Principles

## Modular First

Never implement features specifically for one block, item or mob.

Always:

1. Design the generic mechanic.
2. Make the mechanic extensible.
3. Add content using the mechanic.

Example:

Bad:

"Implement iron ore."

Good:

- Implement generic ore generation.
- Add iron ore as data.

## Content Should Be Data

The engine contains systems.

Content uses those systems.

Examples:

- Blocks
- Items
- Recipes
- Animals
- Mobs
- Biomes
- Structures
- Loot tables

Should eventually become data-driven.

## Separation Between Engine and Content

Engine:

- Rendering
- Physics
- Chunk management
- World generation framework
- Inventory system
- Recipe system
- AI framework

Content:

- Stone block
- Coal ore
- Oak tree
- Cow
- Zombie
- Plains biome

## Future Modding Support

The architecture should make it possible later to:

- Load blocks dynamically
- Register new items
- Register recipes
- Register entities
- Register structures
- Register dimensions

Modding is not implemented initially, but decisions should not prevent it.

# Gameplay Scope

Target experience:

Minecraft Beta 1.7 style survival.

# Core Gameplay Loop

Explore

↓

Gather resources

↓

Craft tools

↓

Mine better resources

↓

Acquire food

↓

Build shelter

↓

Explore further

↓

Progress

# Initial Features

## World Generation

Biomes:

- Plains
- Forest
- Mountains
- Desert

Terrain:

- Heightmap Perlin noise
- Overhangs
- Caves
- Lakes

Resources:

- Coal
- Iron
- Gold
- Diamond

Decorations:

- Trees
- Flowers
- Grass
- Cacti

## Blocks

Terrain:

- Grass
- Dirt
- Stone
- Sand
- Gravel
- Water

Wood:

- Log
- Planks
- Leaves

Utility:

- Crafting table
- Furnace

Agriculture:

- Farmland
- Wheat

## Player

Movement:

- Walking
- Sprinting
- Jumping

Needs:

- Health
- Hunger

Interaction:

- Break blocks
- Place blocks

## Inventory

- Hotbar
- Main inventory
- Stack sizes

## Tools

Materials:

- Wood
- Stone
- Iron
- Diamond

Types:

- Pickaxe
- Axe
- Shovel

Properties:

- Durability
- Mining level
- Mining speed

## Crafting

2×2 inventory crafting.

3×3 crafting table.

Recipe system.

## Furnace

Input slot.

Fuel slot.

Output slot.

Smelting timers.

## Farming

Seeds.

Wheat growth.

Farmland.

## Animals

Cow.

Pig.

Sheep.

Chicken.

Simple wandering AI.

## Hostile Mobs

Zombie.

Skeleton.

Creeper.

Basic state machine AI.

# Technical Stack

Language:

Rust

Renderer:

wgpu

Window/Input:

winit

Math:

glam

Noise:

noise

Textures:

image

Serialization:

serde

Persistence:

bincode

Random:

rand

Logging:

tracing

UI:

egui (debug tools only)

# World Representation

World

→ Chunks

→ Blocks

Chunk size:

16×16×256

Chunks own:

- block ids
- light values
- metadata

Chunks are independent.

Chunk meshing is separate.

Chunk storage is separate.

Chunk generation is separate.

# Block System

Blocks are IDs.

Properties belong to block definitions.

Example:

Stone:

- hardness
- texture
- drops
- transparency

Grass:

- hardness
- top texture
- side texture

Blocks should not contain behavior.

Behavior belongs to systems.

# Item System

Items are definitions.

Properties:

- stack size
- icon
- durability
- tool type

Tools are items.

Food is items.

Blocks as inventory items are items.

# Entity System

All creatures derive from a generic entity framework.

Entity:

- transform
- velocity
- health

Animals and mobs extend it.

Future:

- villagers
- bosses
- projectiles

# AI System

Behavior should be modular.

State machine:

Idle

Wander

Chase

Attack

Flee

Future entities reuse states.

# World Generation

Generation should be a pipeline.

Terrain

↓

Biomes

↓

Caves

↓

Ore generation

↓

Water

↓

Trees

↓

Decorations

↓

Structures

Every stage should be independent.

Adding a new biome should not require rewriting terrain generation.

# Recipe System

Recipes should be data-driven.

Recipe:

Input pattern

↓

Output item

Future:

- shapeless recipes
- smelting recipes
- brewing recipes

# Registry Philosophy

Everything should eventually be registered.

BlockRegistry

ItemRegistry

RecipeRegistry

BiomeRegistry

EntityRegistry

StructureRegistry

This greatly simplifies mod support later.

# Rendering

Pipeline:

Chunks

↓

Mesh generation

↓

GPU buffers

↓

Renderer

Rendering and world logic must remain separated.

Chunk meshing should use:

Greedy meshing.

Future:

- ambient occlusion
- transparency
- water rendering

# Folder Structure

src/

main.rs

app/

renderer/

camera.rs

texture_atlas.rs

mesh/

chunk_mesher.rs

greedy_mesher.rs

world/

world.rs

chunk.rs

block.rs

block_registry.rs

chunk_storage.rs

generation/

terrain.rs

biomes.rs

caves.rs

ores.rs

trees.rs

decorations.rs

player/

player.rs

movement.rs

inventory.rs

interaction/

raycast.rs

block_breaking.rs

block_placing.rs

items/

item.rs

item_registry.rs

tools.rs

food.rs

crafting/

recipe.rs

recipe_registry.rs

crafting_table.rs

furnace/

furnace.rs

smelting_recipe.rs

entities/

entity.rs

animals/

cow.rs

pig.rs

sheep.rs

chicken.rs

mobs/

zombie.rs

skeleton.rs

creeper.rs

ai/

state_machine.rs

wander.rs

chase.rs

attack.rs

save/

world_save.rs

chunk_save.rs

debug/

profiler.rs

ui.rs

assets/

textures/

blocks/

items/

entities/

docs/

player/

developer/

architecture/

systems/

# Ordered Development Plan

## Step 1

Initialize Rust project.

Setup:

- wgpu
- winit
- glam
- tracing

Goal:

Window and render loop.

## Step 2

Camera and movement.

Goal:

Fly around.

## Step 3

Block definitions.

Goal:

Render cubes.

## Step 4

Chunk system.

Goal:

Render chunk meshes.

## Step 5

Texture atlas.

Goal:

Textured blocks.

## Step 6

Greedy meshing.

Goal:

Efficient chunk rendering.

## Step 7

World generation.

Terrain using Perlin noise.

## Step 8

Biomes.

## Step 9

Caves.

## Step 10

Ore generation.

## Step 11

Trees and decorations.

## Step 12

Player physics.

Gravity and collisions.

## Step 13

Raycasting.

Block selection.

## Step 14

Breaking and placing blocks.

## Step 15

Inventory.

## Step 16

Item system.

## Step 17

Tool system.

## Step 18

Crafting.

## Step 19

Furnaces.

## Step 20

Saving/loading.

## Step 21

Health and hunger.

## Step 22

Animals.

## Step 23

Day/night cycle.

## Step 24

Hostile mobs.

## Documentation Strategy

Documentation is a first-class citizen.

Every mechanic must be documented.

# Player Documentation

Purpose:

Explain gameplay.

Future destination:

Website.

Examples:

Blocks.

Items.

Recipes.

Biomes.

Animals.

Mobs.

Progression.

# Developer Documentation

Purpose:

Explain architecture.

Examples:

Chunk system.

Renderer.

Generation pipeline.

Registries.

Entity system.

Recipe system.

Save format.

# AI Agent Documentation

Purpose:

Allow coding agents to understand the project.

Every system should have:

Purpose.

Responsibilities.

Dependencies.

Extension points.

Examples.

Known limitations.

# Long-Term Goals

Eventually support:

- More biomes
- More ores
- Structures
- Modding
- Scientific mecanics

The engine should always prioritize extensibility over short-term convenience.
