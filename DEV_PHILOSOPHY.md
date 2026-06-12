# ENGINE_PHILOSOPHY.md

Version 1.0

---

# Purpose

This document defines the engineering philosophy and development rules of the project.

Its purpose is to ensure that the game remains:

- Maintainable
- Extensible
- Understandable
- Friendly to AI agents
- Friendly to future contributors
- Compatible with future modding

These rules take precedence over short-term convenience.

---

# Fundamental Principle

We are not building a game.

We are building a platform capable of hosting a game.

The initial game is inspired by Minecraft Beta, but the engine should not assume that future mechanics will resemble Minecraft.

Every system should be designed as if dozens of future mechanics will build on top of it.

---

# Golden Rule

Never implement content.

Implement mechanics.

Then create content using those mechanics.

---

# The Three-Step Rule

Whenever adding a feature:

## Step 1

Think about the mechanic.

Example:

"Ore generation."

Not:

"Iron ore."

---

## Step 2

Design a reusable and extensible system.

Example:

Generic ore generation pipeline.

---

## Step 3

Add actual content.

Example:

Coal.

Iron.

Gold.

Diamond.

---

# Avoid Special Cases

Special cases are technical debt.

Bad:

```rust
if block == Block::IronOre {
    // ...
}
```

Good:

```rust
if block_definition.is_ore {
    // ...
}
```

Even better:

```rust
ore_generator.generate(ore_definition);
```

---

# Content Must Be Data

Whenever possible:

Behavior belongs to systems.

Properties belong to definitions.

Content should eventually become serializable.

Examples:

Blocks.

Items.

Recipes.

Biomes.

Animals.

Mobs.

Structures.

Loot tables.

Plants.

Dimensions.

---

# Systems Own Behavior

Bad:

StoneBlock::update()

Cow::wander()

Zombie::attack()

---

Good:

BlockSystem

AISystem

CombatSystem

MovementSystem

GrowthSystem

WeatherSystem

---

Objects should contain data.

Systems should contain logic.

---

# Favor Composition Over Inheritance

Bad:

Animal

↓

Cow

↓

SuperCow

↓

FlyingSuperCow

---

Good:

Entity

- HealthComponent

- MovementComponent

- HungerComponent

- WanderBehavior

- AttackBehavior

- InventoryComponent

---

Complex inheritance trees should be avoided.

---

# Separate Engine From Content

Engine:

- Rendering
- Physics
- World generation framework
- AI framework
- Recipe framework
- Save system

Content:

- Stone
- Oak tree
- Cow
- Zombie
- Plains biome

Content should never leak into engine code.

---

# Separate Logic From Rendering

Rendering should know nothing about:

- Hunger
- Crafting
- AI
- Recipes
- Combat

Game logic should know nothing about:

- GPU
- Meshes
- Buffers
- Shaders

The renderer is a client of the game state.

---

# Separate Generation Stages

World generation should be a pipeline.

Terrain

↓

Biomes

↓

Caves

↓

Ore placement

↓

Water

↓

Trees

↓

Decorations

↓

Structures

Stages should be independent.

Adding one stage should not require rewriting others.

---

# Use Registries Everywhere

Everything should eventually be registered.

BlockRegistry

ItemRegistry

BiomeRegistry

RecipeRegistry

EntityRegistry

StructureRegistry

DimensionRegistry

ParticleRegistry

EffectRegistry

Registries make:

- Modding easier
- Serialization easier
- Documentation easier
- AI understanding easier

---

# Prefer Generic Systems, Allow Specific Content Behaviors

Bad:

- `cow_ai.rs`
- `iron_generator.rs`
- `birch_tree.rs`

When these files hardcode one piece of content directly into the engine.

Good:

- `animal_ai.rs`
- `ore_generation.rs`
- `tree_generation.rs`
- `behavior_registry.rs`

Specific content should usually be added through definitions.

However, some entities may need unique behavior.

Examples:

- Creeper explosion behavior
- Skeleton ranged attack behavior
- Spider climbing behavior
- Chicken slow falling behavior

In those cases, create reusable behavior modules:

- `explode_near_target.rs`
- `ranged_attack.rs`
- `climb_walls.rs`
- `slow_fall.rs`

Then attach those behaviors to entity definitions.

Example:

````rust
Zombie = EntityDefinition {
    behaviors: [Wander, ChaseTarget, MeleeAttack],
}

Skeleton = EntityDefinition {
    behaviors: [Wander, KeepDistance, RangedAttack],
}

Creeper = EntityDefinition {
    behaviors: [Wander, ChaseTarget, ExplodeNearTarget],
}

---

# Make Dependencies Explicit

Each system should document:

Purpose.

Inputs.

Outputs.

Dependencies.

Extension points.

Example:

```markdown
# FurnaceSystem

Purpose:
Smelts items using fuel.

Inputs:

- Inventory
- Smelting recipes

Outputs:

- New items

Dependencies:

- RecipeRegistry
- ItemRegistry

Extension Points:

- New fuels
- New recipes
````

---

# Keep Systems Small

Prefer:

Many simple systems.

Avoid:

Huge god objects.

Bad:

GameManager

WorldManager

MasterSystem

EverythingController

---

Good:

ChunkSystem

LightingSystem

AISystem

CombatSystem

GrowthSystem

WeatherSystem

RecipeSystem

---

# Document Before Expanding

When adding a new mechanic:

1. Understand the mechanic.
2. Document it.
3. Design it.
4. Implement it.

Documentation is part of development.

---

# Player Documentation

Player documentation explains:

- Mechanics
- Progression
- Blocks
- Recipes
- Biomes
- Creatures

Eventually this becomes a website.

---

# Developer Documentation

Developer documentation explains:

- Architecture
- Data flow
- Save format
- Registries
- Systems
- Dependencies

---

# AI Documentation

Every system should have:

Purpose.

Responsibilities.

Inputs.

Outputs.

Dependencies.

Extension points.

Examples.

Known limitations.

An AI agent should be able to understand any system without reading the entire codebase.

---

# Files Should Be Understandable In Isolation

A developer should understand most files by reading only:

- That file.
- Nearby files.
- The corresponding documentation.

Avoid hidden assumptions.

---

# Keep APIs Stable

Internal APIs should change slowly.

When changing APIs:

Prefer extending.

Avoid breaking.

Small interfaces are preferred.

---

# Optimize Late

Priority order:

Correctness

↓

Clarity

↓

Modularity

↓

Performance

Premature optimization is discouraged.

Once correctness is achieved:

Profile.

Measure.

Optimize.

---

# Avoid Clever Code

Code should be boring.

Code should be obvious.

Future maintainability is more important than elegance.

A slightly repetitive implementation is preferable to a clever but fragile abstraction.

---

# Every Mechanic Must Be Extendable

Whenever implementing something, ask:

"What if there are 50 versions of this?"

Examples:

50 ores.

50 animals.

50 biomes.

50 dimensions.

50 weather effects.

50 foods.

50 tools.

The architecture should survive this growth.

---

# Build Foundations Before Content

Priority:

Mechanic

↓

Framework

↓

Content

Never the opposite.

---

# Long-Term Goal

The engine should eventually support:

- Hundreds of blocks
- Hundreds of items
- Many dimensions
- Modding
- Multiplayer
- Dedicated servers
- Plugins
- Procedural structures
- Scientific mechanics
- Entirely new gameplay loops

Without requiring fundamental rewrites.

Architecture quality is more important than development speed.

Short-term convenience should never compromise long-term extensibility.
