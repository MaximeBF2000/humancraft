use std::io::Read;

use crate::engine::world::{
    Axis, BlockProperties, HorizontalDirection, SlabOrientation, StairHalf,
};

use super::WorldSaveError;

pub(super) fn read_i32(reader: &mut impl Read) -> Result<i32, WorldSaveError> {
    let mut bytes = [0_u8; 4];
    reader.read_exact(&mut bytes)?;
    Ok(i32::from_le_bytes(bytes))
}

pub(super) fn read_u16(reader: &mut impl Read) -> Result<u16, WorldSaveError> {
    let mut bytes = [0_u8; 2];
    reader.read_exact(&mut bytes)?;
    Ok(u16::from_le_bytes(bytes))
}

pub(super) fn read_u8(reader: &mut impl Read) -> Result<u8, WorldSaveError> {
    let mut byte = [0_u8; 1];
    reader.read_exact(&mut byte)?;
    Ok(byte[0])
}

pub(super) fn read_block_properties(
    reader: &mut impl Read,
) -> Result<BlockProperties, WorldSaveError> {
    match read_u8(reader)? {
        0 => Ok(BlockProperties::None),
        1 => Ok(BlockProperties::HorizontalFacing {
            facing: read_horizontal_direction(reader)?,
        }),
        2 => Ok(BlockProperties::Furnace {
            facing: read_horizontal_direction(reader)?,
            lit: read_u8(reader)? != 0,
        }),
        3 => Ok(BlockProperties::Axis {
            axis: match read_u8(reader)? {
                0 => Axis::X,
                1 => Axis::Y,
                2 => Axis::Z,
                other => {
                    return Err(WorldSaveError::InvalidChunk(format!(
                        "invalid axis property {other}"
                    )));
                }
            },
        }),
        4 => Ok(BlockProperties::Slab {
            orientation: match read_u8(reader)? {
                0 => SlabOrientation::Bottom,
                1 => SlabOrientation::Top,
                2 => SlabOrientation::North,
                3 => SlabOrientation::South,
                4 => SlabOrientation::East,
                5 => SlabOrientation::West,
                other => {
                    return Err(WorldSaveError::InvalidChunk(format!(
                        "invalid slab orientation {other}"
                    )));
                }
            },
        }),
        5 => Ok(BlockProperties::Stairs {
            facing: read_horizontal_direction(reader)?,
            half: match read_u8(reader)? {
                0 => StairHalf::Bottom,
                1 => StairHalf::Top,
                other => {
                    return Err(WorldSaveError::InvalidChunk(format!(
                        "invalid stair half {other}"
                    )));
                }
            },
        }),
        6 => Ok(BlockProperties::Leaves {
            persistent: read_u8(reader)? != 0,
        }),
        7 => Ok(BlockProperties::Sapling {
            stage: read_u8(reader)?,
        }),
        other => Err(WorldSaveError::InvalidChunk(format!(
            "invalid block property kind {other}"
        ))),
    }
}

pub(super) fn write_block_properties(bytes: &mut Vec<u8>, properties: BlockProperties) {
    match properties {
        BlockProperties::None => bytes.push(0),
        BlockProperties::HorizontalFacing { facing } => {
            bytes.push(1);
            bytes.push(horizontal_direction_byte(facing));
        }
        BlockProperties::Furnace { facing, lit } => {
            bytes.push(2);
            bytes.push(horizontal_direction_byte(facing));
            bytes.push(u8::from(lit));
        }
        BlockProperties::Axis { axis } => {
            bytes.push(3);
            bytes.push(match axis {
                Axis::X => 0,
                Axis::Y => 1,
                Axis::Z => 2,
            });
        }
        BlockProperties::Slab { orientation } => {
            bytes.push(4);
            bytes.push(match orientation {
                SlabOrientation::Bottom => 0,
                SlabOrientation::Top => 1,
                SlabOrientation::North => 2,
                SlabOrientation::South => 3,
                SlabOrientation::East => 4,
                SlabOrientation::West => 5,
            });
        }
        BlockProperties::Stairs { facing, half } => {
            bytes.push(5);
            bytes.push(horizontal_direction_byte(facing));
            bytes.push(match half {
                StairHalf::Bottom => 0,
                StairHalf::Top => 1,
            });
        }
        BlockProperties::Leaves { persistent } => {
            bytes.push(6);
            bytes.push(u8::from(persistent));
        }
        BlockProperties::Sapling { stage } => {
            bytes.push(7);
            bytes.push(stage);
        }
    }
}

fn read_horizontal_direction(
    reader: &mut impl Read,
) -> Result<HorizontalDirection, WorldSaveError> {
    match read_u8(reader)? {
        0 => Ok(HorizontalDirection::North),
        1 => Ok(HorizontalDirection::South),
        2 => Ok(HorizontalDirection::East),
        3 => Ok(HorizontalDirection::West),
        other => Err(WorldSaveError::InvalidChunk(format!(
            "invalid horizontal direction {other}"
        ))),
    }
}

fn horizontal_direction_byte(direction: HorizontalDirection) -> u8 {
    match direction {
        HorizontalDirection::North => 0,
        HorizontalDirection::South => 1,
        HorizontalDirection::East => 2,
        HorizontalDirection::West => 3,
    }
}
