//! Block definitions and IDs.
//!
//! Purpose:
//! Describe block properties without embedding behavior in block values.
//!
//! Extension points:
//! New blocks are registered as definitions. Systems such as mining, meshing,
//! lighting, and drops consume properties from this registry.

use crate::engine::registry::{Definition, Registry};
use crate::engine::world::ToolKind;

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub struct BlockId(usize);

impl BlockId {
    pub const fn raw(self) -> usize {
        self.0
    }
}

impl From<usize> for BlockId {
    fn from(value: usize) -> Self {
        Self(value)
    }
}

impl From<BlockId> for usize {
    fn from(value: BlockId) -> Self {
        value.0
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct BlockDefinition {
    pub key: String,
    pub display_name: String,
    pub hardness: f32,
    pub transparent: bool,
    pub solid: bool,
    pub drops: Vec<String>,
    pub tags: Vec<String>,
    pub textures: BlockTextures,
    pub effective_tool: Option<ToolKind>,
    pub harvest_requirement: Option<BlockHarvestRequirement>,
    pub placement: PlacementRuleKind,
    pub shape: BlockShape,
    pub behavior: BlockBehavior,
}

impl BlockDefinition {
    pub fn new(key: impl Into<String>, display_name: impl Into<String>) -> Self {
        Self {
            key: key.into(),
            display_name: display_name.into(),
            hardness: 1.0,
            transparent: false,
            solid: true,
            drops: Vec::new(),
            tags: Vec::new(),
            textures: BlockTextures::missing(),
            effective_tool: None,
            harvest_requirement: None,
            placement: PlacementRuleKind::Simple,
            shape: BlockShape::FullCube,
            behavior: BlockBehavior::default(),
        }
    }

    pub fn hardness(mut self, hardness: f32) -> Self {
        self.hardness = hardness;
        self
    }

    pub fn transparent(mut self, transparent: bool) -> Self {
        self.transparent = transparent;
        self
    }

    pub fn solid(mut self, solid: bool) -> Self {
        self.solid = solid;
        self
    }

    pub fn drops(mut self, drops: impl IntoIterator<Item = impl Into<String>>) -> Self {
        self.drops = drops.into_iter().map(Into::into).collect();
        self
    }

    pub fn tags(mut self, tags: impl IntoIterator<Item = impl Into<String>>) -> Self {
        self.tags = tags.into_iter().map(Into::into).collect();
        self
    }

    pub fn textures(mut self, textures: BlockTextures) -> Self {
        self.textures = textures;
        self
    }

    pub fn effective_tool(mut self, tool: ToolKind) -> Self {
        self.effective_tool = Some(tool);
        self
    }

    pub fn harvest_requirement(mut self, tool: ToolKind, min_level: u8) -> Self {
        self.harvest_requirement = Some(BlockHarvestRequirement { tool, min_level });
        self
    }

    pub fn placement(mut self, placement: PlacementRuleKind) -> Self {
        self.placement = placement;
        self
    }

    pub fn shape(mut self, shape: BlockShape) -> Self {
        self.shape = shape;
        self
    }

    pub fn behavior(mut self, behavior: BlockBehavior) -> Self {
        self.behavior = behavior;
        self
    }

    pub fn has_tag(&self, tag: &str) -> bool {
        self.tags.iter().any(|candidate| candidate == tag)
    }
}

impl Definition for BlockDefinition {
    fn key(&self) -> &str {
        &self.key
    }
}

pub type BlockRegistry = Registry<BlockId, BlockDefinition>;

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub struct BlockState {
    pub block: BlockId,
    pub properties: BlockProperties,
}

impl BlockState {
    pub const fn new(block: BlockId) -> Self {
        Self {
            block,
            properties: BlockProperties::None,
        }
    }

    pub const fn with_properties(block: BlockId, properties: BlockProperties) -> Self {
        Self { block, properties }
    }
}

impl From<BlockId> for BlockState {
    fn from(block: BlockId) -> Self {
        Self::new(block)
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub enum BlockProperties {
    None,
    HorizontalFacing {
        facing: HorizontalDirection,
    },
    Furnace {
        facing: HorizontalDirection,
        lit: bool,
    },
    Axis {
        axis: Axis,
    },
    Slab {
        orientation: SlabOrientation,
    },
    Stairs {
        facing: HorizontalDirection,
        half: StairHalf,
    },
    Leaves {
        persistent: bool,
    },
    Sapling {
        stage: u8,
    },
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub enum HorizontalDirection {
    North,
    South,
    East,
    West,
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub enum Axis {
    X,
    Y,
    Z,
}

impl HorizontalDirection {
    pub const fn opposite(self) -> Self {
        match self {
            Self::North => Self::South,
            Self::South => Self::North,
            Self::East => Self::West,
            Self::West => Self::East,
        }
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub enum SlabOrientation {
    Bottom,
    Top,
    North,
    South,
    East,
    West,
}

impl SlabOrientation {
    pub const fn opposite(self) -> Self {
        match self {
            Self::Bottom => Self::Top,
            Self::Top => Self::Bottom,
            Self::North => Self::South,
            Self::South => Self::North,
            Self::East => Self::West,
            Self::West => Self::East,
        }
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub enum StairHalf {
    Bottom,
    Top,
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub enum PlacementRuleKind {
    Simple,
    AxisFromClickedFace,
    FacePlayerHorizontal,
    Slab,
    Stairs,
    PersistentLeaves,
    Sapling,
}

#[derive(Debug, Copy, Clone, PartialEq)]
pub enum BlockShape {
    Empty,
    FullCube,
    Slab,
    Stairs,
    Cross,
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct BlockBehavior {
    pub gravity: bool,
    pub leaf_decay: Option<LeafDecayBehavior>,
    pub grass_spread: Option<GrassSpreadBehavior>,
    pub sapling_growth: Option<SaplingGrowthBehavior>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct LeafDecayBehavior {
    pub log_tag: String,
    pub max_distance: u8,
    pub sapling_drop: ChanceDrop,
}

#[derive(Debug, Clone, PartialEq)]
pub struct GrassSpreadBehavior {
    pub target_block_key: String,
    pub attempts_per_random_tick: u8,
    pub horizontal_range: i32,
    pub down_range: i32,
    pub up_range: i32,
}

#[derive(Debug, Clone, PartialEq)]
pub struct SaplingGrowthBehavior {
    pub grow_on_tags: Vec<String>,
    pub trunk_block_key: String,
    pub leaves_block_key: String,
    pub min_trunk_height: usize,
    pub max_trunk_height: usize,
    pub canopy_radius: usize,
    pub required_light: u8,
    pub required_clearance: usize,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ChanceDrop {
    pub item_key: String,
    pub chance: f32,
}

#[derive(Debug, Copy, Clone, PartialEq)]
pub struct BlockAabb {
    pub min: [f32; 3],
    pub max: [f32; 3],
}

impl BlockAabb {
    pub const fn new(min: [f32; 3], max: [f32; 3]) -> Self {
        Self { min, max }
    }
}

#[derive(Debug, Copy, Clone, PartialEq)]
pub struct BlockAabbs {
    values: [BlockAabb; 2],
    len: usize,
}

impl BlockAabbs {
    pub const fn empty() -> Self {
        Self {
            values: [BlockAabb::new([0.0, 0.0, 0.0], [0.0, 0.0, 0.0]); 2],
            len: 0,
        }
    }

    pub const fn one(aabb: BlockAabb) -> Self {
        Self {
            values: [aabb, BlockAabb::new([0.0, 0.0, 0.0], [0.0, 0.0, 0.0])],
            len: 1,
        }
    }

    pub const fn two(first: BlockAabb, second: BlockAabb) -> Self {
        Self {
            values: [first, second],
            len: 2,
        }
    }

    pub fn iter(&self) -> impl Iterator<Item = BlockAabb> + '_ {
        self.values[..self.len].iter().copied()
    }
}

pub fn block_state_aabbs(definition: &BlockDefinition, state: BlockState) -> BlockAabbs {
    match definition.shape {
        BlockShape::Empty => BlockAabbs::empty(),
        BlockShape::FullCube => BlockAabbs::one(BlockAabb::new([0.0, 0.0, 0.0], [1.0, 1.0, 1.0])),
        BlockShape::Cross => BlockAabbs::empty(),
        BlockShape::Slab => match state.properties {
            BlockProperties::Slab { orientation } => slab_aabb(orientation),
            _ => BlockAabbs::one(BlockAabb::new([0.0, 0.0, 0.0], [1.0, 0.5, 1.0])),
        },
        BlockShape::Stairs => match state.properties {
            BlockProperties::Stairs { facing, half } => stair_aabbs(facing, half),
            _ => stair_aabbs(HorizontalDirection::North, StairHalf::Bottom),
        },
    }
}

fn slab_aabb(orientation: SlabOrientation) -> BlockAabbs {
    let aabb = match orientation {
        SlabOrientation::Bottom => BlockAabb::new([0.0, 0.0, 0.0], [1.0, 0.5, 1.0]),
        SlabOrientation::Top => BlockAabb::new([0.0, 0.5, 0.0], [1.0, 1.0, 1.0]),
        SlabOrientation::North => BlockAabb::new([0.0, 0.0, 0.0], [1.0, 1.0, 0.5]),
        SlabOrientation::South => BlockAabb::new([0.0, 0.0, 0.5], [1.0, 1.0, 1.0]),
        SlabOrientation::West => BlockAabb::new([0.0, 0.0, 0.0], [0.5, 1.0, 1.0]),
        SlabOrientation::East => BlockAabb::new([0.5, 0.0, 0.0], [1.0, 1.0, 1.0]),
    };
    BlockAabbs::one(aabb)
}

fn stair_aabbs(facing: HorizontalDirection, half: StairHalf) -> BlockAabbs {
    let full_bottom = BlockAabb::new([0.0, 0.0, 0.0], [1.0, 0.5, 1.0]);
    let full_top = BlockAabb::new([0.0, 0.5, 0.0], [1.0, 1.0, 1.0]);
    match (half, facing) {
        (StairHalf::Bottom, HorizontalDirection::North) => BlockAabbs::two(
            full_bottom,
            BlockAabb::new([0.0, 0.5, 0.0], [1.0, 1.0, 0.5]),
        ),
        (StairHalf::Bottom, HorizontalDirection::South) => BlockAabbs::two(
            full_bottom,
            BlockAabb::new([0.0, 0.5, 0.5], [1.0, 1.0, 1.0]),
        ),
        (StairHalf::Bottom, HorizontalDirection::West) => BlockAabbs::two(
            full_bottom,
            BlockAabb::new([0.0, 0.5, 0.0], [0.5, 1.0, 1.0]),
        ),
        (StairHalf::Bottom, HorizontalDirection::East) => BlockAabbs::two(
            full_bottom,
            BlockAabb::new([0.5, 0.5, 0.0], [1.0, 1.0, 1.0]),
        ),
        (StairHalf::Top, HorizontalDirection::North) => {
            BlockAabbs::two(full_top, BlockAabb::new([0.0, 0.0, 0.5], [1.0, 0.5, 1.0]))
        }
        (StairHalf::Top, HorizontalDirection::South) => {
            BlockAabbs::two(full_top, BlockAabb::new([0.0, 0.0, 0.0], [1.0, 0.5, 0.5]))
        }
        (StairHalf::Top, HorizontalDirection::West) => {
            BlockAabbs::two(full_top, BlockAabb::new([0.5, 0.0, 0.0], [1.0, 0.5, 1.0]))
        }
        (StairHalf::Top, HorizontalDirection::East) => {
            BlockAabbs::two(full_top, BlockAabb::new([0.0, 0.0, 0.0], [0.5, 0.5, 1.0]))
        }
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub struct BlockHarvestRequirement {
    pub tool: ToolKind,
    pub min_level: u8,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BlockTextures {
    pub top: String,
    pub bottom: String,
    pub north: String,
    pub south: String,
    pub east: String,
    pub west: String,
}

impl BlockTextures {
    pub fn missing() -> Self {
        Self::all("humancraft:missing")
    }

    pub fn all(texture: impl Into<String>) -> Self {
        let texture = texture.into();
        Self {
            top: texture.clone(),
            bottom: texture.clone(),
            north: texture.clone(),
            south: texture.clone(),
            east: texture.clone(),
            west: texture,
        }
    }

    pub fn top_bottom_sides(
        top: impl Into<String>,
        bottom: impl Into<String>,
        sides: impl Into<String>,
    ) -> Self {
        let sides = sides.into();
        Self {
            top: top.into(),
            bottom: bottom.into(),
            north: sides.clone(),
            south: sides.clone(),
            east: sides.clone(),
            west: sides,
        }
    }
}
