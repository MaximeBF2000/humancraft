//! World save metadata and chunk persistence.
//!
//! Purpose:
//! Store world identity, generation seed, player state, and edited chunks without
//! coupling persistence to rendering or input code.

use std::fmt::{Display, Formatter};
use std::fs;
use std::io::{self, Read, Write};
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use crate::engine::world::{
    BlockProperties, BlockRegistry, BlockState, CHUNK_VOLUME, Chunk, ChunkPosition,
    chunk::ChunkError,
};

mod block_properties;

use block_properties::{read_block_properties, read_i32, read_u16, write_block_properties};

const METADATA_FILE: &str = "world.txt";
const BLOCK_ENTITIES_FILE: &str = "block_entities.txt";
const LEGACY_RAW_ID_CHUNK_MAGIC: &[u8; 8] = b"HCCNK001";
const STATELESS_KEYED_CHUNK_MAGIC: &[u8; 8] = b"HCCNK002";
const CHUNK_MAGIC: &[u8; 8] = b"HCCNK003";

#[derive(Debug, Clone, PartialEq)]
pub struct WorldMetadata {
    pub id: String,
    pub name: String,
    pub seed: u64,
    pub player: PlayerSave,
    pub inventory: InventorySave,
    pub created_at_unix_seconds: u64,
    pub updated_at_unix_seconds: u64,
}

impl WorldMetadata {
    pub fn new(id: String, name: String, seed: u64, player: PlayerSave) -> Self {
        let now = unix_now_seconds();
        Self {
            id,
            name,
            seed,
            player,
            inventory: InventorySave::empty(36),
            created_at_unix_seconds: now,
            updated_at_unix_seconds: now,
        }
    }
}

#[derive(Debug, Copy, Clone, PartialEq)]
pub struct PlayerSave {
    pub eye_x: f32,
    pub eye_y: f32,
    pub eye_z: f32,
    pub yaw: f32,
    pub pitch: f32,
}

impl PlayerSave {
    pub fn new(eye_x: f32, eye_y: f32, eye_z: f32, yaw: f32, pitch: f32) -> Self {
        Self {
            eye_x,
            eye_y,
            eye_z,
            yaw,
            pitch,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InventorySave {
    pub slots: Vec<Option<ItemStackSave>>,
}

impl InventorySave {
    pub fn empty(slot_count: usize) -> Self {
        Self {
            slots: vec![None; slot_count],
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ItemStackSave {
    pub item_key: String,
    pub count: u16,
}

impl ItemStackSave {
    pub fn new(item_key: impl Into<String>, count: u16) -> Self {
        Self {
            item_key: item_key.into(),
            count,
        }
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub struct BlockEntityPositionSave {
    pub x: i32,
    pub y: i32,
    pub z: i32,
}

impl BlockEntityPositionSave {
    pub fn new(x: i32, y: i32, z: i32) -> Self {
        Self { x, y, z }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BlockEntitySave {
    pub position: BlockEntityPositionSave,
    pub kind: BlockEntityKindSave,
}

impl BlockEntitySave {
    pub fn chest(position: BlockEntityPositionSave, inventory: InventorySave) -> Self {
        Self {
            position,
            kind: BlockEntityKindSave::Chest { inventory },
        }
    }

    pub fn furnace(
        position: BlockEntityPositionSave,
        inventory: InventorySave,
        burn_ticks: u32,
        fuel_ticks: u32,
        cook_ticks: u32,
        cook_ticks_total: u32,
    ) -> Self {
        Self {
            position,
            kind: BlockEntityKindSave::Furnace {
                inventory,
                burn_ticks,
                fuel_ticks,
                cook_ticks,
                cook_ticks_total,
            },
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BlockEntityKindSave {
    Chest {
        inventory: InventorySave,
    },
    Furnace {
        inventory: InventorySave,
        burn_ticks: u32,
        fuel_ticks: u32,
        cook_ticks: u32,
        cook_ticks_total: u32,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WorldSaveError {
    Io(String),
    InvalidMetadata(String),
    InvalidChunk(String),
}

impl Display for WorldSaveError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Io(message) => write!(formatter, "save I/O error: {message}"),
            Self::InvalidMetadata(message) => {
                write!(formatter, "invalid world metadata: {message}")
            }
            Self::InvalidChunk(message) => write!(formatter, "invalid chunk save: {message}"),
        }
    }
}

impl std::error::Error for WorldSaveError {}

impl From<io::Error> for WorldSaveError {
    fn from(error: io::Error) -> Self {
        Self::Io(error.to_string())
    }
}

impl From<ChunkError> for WorldSaveError {
    fn from(error: ChunkError) -> Self {
        Self::InvalidChunk(format!("{error:?}"))
    }
}

pub struct WorldSaveStore {
    root: PathBuf,
}

impl WorldSaveStore {
    pub fn new(root: impl Into<PathBuf>) -> Self {
        Self { root: root.into() }
    }

    pub fn default() -> Self {
        Self::new(
            PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                .join("saves")
                .join("worlds"),
        )
    }

    pub fn list_worlds(&self) -> Result<Vec<WorldMetadata>, WorldSaveError> {
        if !self.root.exists() {
            return Ok(Vec::new());
        }

        let mut worlds = Vec::new();
        for entry in fs::read_dir(&self.root)? {
            let entry = entry?;
            if !entry.file_type()?.is_dir() {
                continue;
            }
            let metadata_path = entry.path().join(METADATA_FILE);
            if metadata_path.exists() {
                worlds.push(read_metadata_file(&metadata_path)?);
            }
        }
        worlds.sort_by(|left, right| {
            right
                .updated_at_unix_seconds
                .cmp(&left.updated_at_unix_seconds)
                .then_with(|| left.name.cmp(&right.name))
        });
        Ok(worlds)
    }

    pub fn create_world(
        &self,
        name: &str,
        seed: u64,
        player: PlayerSave,
    ) -> Result<WorldMetadata, WorldSaveError> {
        fs::create_dir_all(&self.root)?;
        let id = self.unique_world_id(name);
        let metadata = WorldMetadata::new(id, sanitize_display_name(name), seed, player);
        fs::create_dir_all(self.world_path(&metadata.id).join("chunks"))?;
        self.save_metadata(&metadata)?;
        Ok(metadata)
    }

    pub fn save_metadata(&self, metadata: &WorldMetadata) -> Result<(), WorldSaveError> {
        fs::create_dir_all(self.world_path(&metadata.id))?;
        write_metadata_file(&self.metadata_path(&metadata.id), metadata)
    }

    pub fn rename_world(&self, id: &str, name: &str) -> Result<WorldMetadata, WorldSaveError> {
        let mut metadata = self.load_metadata(id)?;
        metadata.name = sanitize_display_name(name);
        metadata.updated_at_unix_seconds = unix_now_seconds();
        self.save_metadata(&metadata)?;
        Ok(metadata)
    }

    pub fn delete_world(&self, id: &str) -> Result<(), WorldSaveError> {
        let path = self.world_path(id);
        if path.exists() {
            fs::remove_dir_all(path)?;
        }
        Ok(())
    }

    pub fn load_metadata(&self, id: &str) -> Result<WorldMetadata, WorldSaveError> {
        read_metadata_file(&self.metadata_path(id))
    }

    pub fn load_block_entities(
        &self,
        world_id: &str,
    ) -> Result<Vec<BlockEntitySave>, WorldSaveError> {
        let path = self.block_entities_path(world_id);
        if !path.exists() {
            return Ok(Vec::new());
        }
        read_block_entities_file(&path)
    }

    pub fn save_block_entities(
        &self,
        world_id: &str,
        entities: &[BlockEntitySave],
    ) -> Result<(), WorldSaveError> {
        let path = self.block_entities_path(world_id);
        write_block_entities_file(&path, entities)
    }

    pub fn load_chunk(
        &self,
        world_id: &str,
        position: ChunkPosition,
        blocks: &BlockRegistry,
    ) -> Result<Option<Chunk>, WorldSaveError> {
        let path = self.chunk_path(world_id, position);
        if !path.exists() {
            return Ok(None);
        }

        let mut file = fs::File::open(path)?;
        let mut magic = [0_u8; 8];
        file.read_exact(&mut magic)?;
        if &magic == LEGACY_RAW_ID_CHUNK_MAGIC {
            eprintln!(
                "ignoring legacy raw-id chunk {},{}; regenerating with keyed chunk format",
                position.x, position.z
            );
            return Ok(None);
        }
        let stateful_chunk = match &magic {
            value if value == STATELESS_KEYED_CHUNK_MAGIC => false,
            value if value == CHUNK_MAGIC => true,
            _ => return Err(WorldSaveError::InvalidChunk("bad magic".to_string())),
        };

        let saved_x = read_i32(&mut file)?;
        let saved_z = read_i32(&mut file)?;
        if saved_x != position.x || saved_z != position.z {
            return Err(WorldSaveError::InvalidChunk(format!(
                "chunk file position {saved_x},{saved_z} did not match requested {},{}",
                position.x, position.z
            )));
        }

        let palette_len = read_u16(&mut file)? as usize;
        let mut palette = Vec::with_capacity(palette_len);
        for _ in 0..palette_len {
            let key_len = read_u16(&mut file)? as usize;
            let mut key_bytes = vec![0_u8; key_len];
            file.read_exact(&mut key_bytes)?;
            let key = String::from_utf8(key_bytes)
                .map_err(|_| WorldSaveError::InvalidChunk("invalid block key".to_string()))?;
            let Some(block) = blocks.id_for_key(&key) else {
                return Err(WorldSaveError::InvalidChunk(format!(
                    "unknown saved block key {key}"
                )));
            };
            let properties = if stateful_chunk {
                read_block_properties(&mut file)?
            } else {
                BlockProperties::None
            };
            palette.push(BlockState::with_properties(block, properties));
        }
        if palette.is_empty() {
            return Err(WorldSaveError::InvalidChunk(
                "empty block palette".to_string(),
            ));
        }

        let mut chunk_blocks = Vec::with_capacity(CHUNK_VOLUME);
        for _ in 0..CHUNK_VOLUME {
            let palette_index = read_u16(&mut file)? as usize;
            let Some(state) = palette.get(palette_index).copied() else {
                return Err(WorldSaveError::InvalidChunk(format!(
                    "invalid block palette index {palette_index}"
                )));
            };
            chunk_blocks.push(state);
        }

        Ok(Some(Chunk::from_states(position, chunk_blocks)?))
    }

    pub fn save_chunk(
        &self,
        world_id: &str,
        chunk: &Chunk,
        blocks: &BlockRegistry,
    ) -> Result<(), WorldSaveError> {
        let path = self.chunk_path(world_id, chunk.position());
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }

        let (palette, palette_indices) = chunk_block_palette(chunk, blocks)?;
        let palette_bytes: usize = palette
            .iter()
            .filter_map(|state| blocks.get(state.block))
            .map(|definition| 2 + definition.key.len())
            .sum();
        let mut bytes = Vec::with_capacity(
            CHUNK_MAGIC.len() + 10 + palette_bytes + palette.len() * 3 + CHUNK_VOLUME * 2,
        );
        bytes.extend_from_slice(CHUNK_MAGIC);
        bytes.extend_from_slice(&chunk.position().x.to_le_bytes());
        bytes.extend_from_slice(&chunk.position().z.to_le_bytes());
        bytes.extend_from_slice(
            &u16::try_from(palette.len())
                .map_err(|_| WorldSaveError::InvalidChunk("block palette too large".to_string()))?
                .to_le_bytes(),
        );
        for state in &palette {
            let definition = blocks.get(state.block).ok_or_else(|| {
                WorldSaveError::InvalidChunk(format!("unknown block id {}", state.block.raw()))
            })?;
            let key_bytes = definition.key.as_bytes();
            bytes.extend_from_slice(
                &u16::try_from(key_bytes.len())
                    .map_err(|_| {
                        WorldSaveError::InvalidChunk(format!(
                            "block key too long {}",
                            definition.key
                        ))
                    })?
                    .to_le_bytes(),
            );
            bytes.extend_from_slice(key_bytes);
            write_block_properties(&mut bytes, state.properties);
        }
        for palette_index in palette_indices {
            bytes.extend_from_slice(&palette_index.to_le_bytes());
        }
        let mut file = fs::File::create(path)?;
        file.write_all(&bytes)?;
        Ok(())
    }

    fn unique_world_id(&self, name: &str) -> String {
        let base = slugify(name);
        let stamp = unix_now_seconds();
        let mut candidate = format!("{base}-{stamp}");
        let mut suffix = 2;
        while self.world_path(&candidate).exists() {
            candidate = format!("{base}-{stamp}-{suffix}");
            suffix += 1;
        }
        candidate
    }

    fn world_path(&self, id: &str) -> PathBuf {
        self.root.join(id)
    }

    fn metadata_path(&self, id: &str) -> PathBuf {
        self.world_path(id).join(METADATA_FILE)
    }

    fn chunk_path(&self, world_id: &str, position: ChunkPosition) -> PathBuf {
        self.world_path(world_id)
            .join("chunks")
            .join(format!("{}_{}.hcc", position.x, position.z))
    }

    fn block_entities_path(&self, world_id: &str) -> PathBuf {
        self.world_path(world_id).join(BLOCK_ENTITIES_FILE)
    }
}

fn write_metadata_file(path: &Path, metadata: &WorldMetadata) -> Result<(), WorldSaveError> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let escaped_name = escape_value(&metadata.name);
    let mut contents = format!(
        "version=1\nid={}\nname={}\nseed={}\nplayer_eye_x={}\nplayer_eye_y={}\nplayer_eye_z={}\nplayer_yaw={}\nplayer_pitch={}\ncreated_at={}\nupdated_at={}\n",
        metadata.id,
        escaped_name,
        metadata.seed,
        metadata.player.eye_x,
        metadata.player.eye_y,
        metadata.player.eye_z,
        metadata.player.yaw,
        metadata.player.pitch,
        metadata.created_at_unix_seconds,
        metadata.updated_at_unix_seconds
    );
    contents.push_str(&format!(
        "inventory_slot_count={}\n",
        metadata.inventory.slots.len()
    ));
    for (index, slot) in metadata.inventory.slots.iter().enumerate() {
        if let Some(stack) = slot {
            contents.push_str(&format!(
                "inventory_slot_{index}={},{}\n",
                escape_value(&stack.item_key),
                stack.count
            ));
        }
    }
    fs::write(path, contents)?;
    Ok(())
}

fn read_metadata_file(path: &Path) -> Result<WorldMetadata, WorldSaveError> {
    let contents = fs::read_to_string(path)?;
    let mut values = std::collections::HashMap::new();
    for line in contents.lines() {
        let Some((key, value)) = line.split_once('=') else {
            continue;
        };
        values.insert(key, value);
    }

    if values.get("version").copied() != Some("1") {
        return Err(WorldSaveError::InvalidMetadata(
            "unsupported metadata version".to_string(),
        ));
    }

    let inventory_slot_count = values
        .get("inventory_slot_count")
        .and_then(|value| value.parse::<usize>().ok())
        .unwrap_or(36);
    let inventory = read_inventory_save(&values, inventory_slot_count)?;

    Ok(WorldMetadata {
        id: required_string(&values, "id")?,
        name: unescape_value(&required_string(&values, "name")?),
        seed: required_parse(&values, "seed")?,
        player: PlayerSave {
            eye_x: required_parse(&values, "player_eye_x")?,
            eye_y: required_parse(&values, "player_eye_y")?,
            eye_z: required_parse(&values, "player_eye_z")?,
            yaw: required_parse(&values, "player_yaw")?,
            pitch: required_parse(&values, "player_pitch")?,
        },
        inventory,
        created_at_unix_seconds: required_parse(&values, "created_at")?,
        updated_at_unix_seconds: required_parse(&values, "updated_at")?,
    })
}

fn read_inventory_save(
    values: &std::collections::HashMap<&str, &str>,
    slot_count: usize,
) -> Result<InventorySave, WorldSaveError> {
    read_inventory_save_with_prefix(values, "inventory", slot_count)
}

fn read_inventory_save_with_prefix(
    values: &std::collections::HashMap<&str, &str>,
    prefix: &str,
    slot_count: usize,
) -> Result<InventorySave, WorldSaveError> {
    let mut inventory = InventorySave::empty(slot_count);
    for index in 0..slot_count {
        let key = format!("{prefix}_slot_{index}");
        let Some(value) = values.get(key.as_str()) else {
            continue;
        };
        let Some((item_key, count)) = value.rsplit_once(',') else {
            return Err(WorldSaveError::InvalidMetadata(format!("invalid {key}")));
        };
        inventory.slots[index] = Some(ItemStackSave::new(
            unescape_value(item_key),
            count
                .parse()
                .map_err(|_| WorldSaveError::InvalidMetadata(format!("invalid {key} count")))?,
        ));
    }
    Ok(inventory)
}

fn write_block_entities_file(
    path: &Path,
    entities: &[BlockEntitySave],
) -> Result<(), WorldSaveError> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }

    let mut contents = format!("version=1\nblock_entity_count={}\n", entities.len());
    for (index, entity) in entities.iter().enumerate() {
        contents.push_str(&format!(
            "block_entity_{index}_position={},{},{}\n",
            entity.position.x, entity.position.y, entity.position.z
        ));
        match &entity.kind {
            BlockEntityKindSave::Chest { inventory } => {
                contents.push_str(&format!("block_entity_{index}_kind=chest\n"));
                write_inventory_save(&mut contents, &format!("block_entity_{index}"), inventory);
            }
            BlockEntityKindSave::Furnace {
                inventory,
                burn_ticks,
                fuel_ticks,
                cook_ticks,
                cook_ticks_total,
            } => {
                contents.push_str(&format!("block_entity_{index}_kind=furnace\n"));
                contents.push_str(&format!("block_entity_{index}_burn_ticks={burn_ticks}\n"));
                contents.push_str(&format!("block_entity_{index}_fuel_ticks={fuel_ticks}\n"));
                contents.push_str(&format!("block_entity_{index}_cook_ticks={cook_ticks}\n"));
                contents.push_str(&format!(
                    "block_entity_{index}_cook_ticks_total={cook_ticks_total}\n"
                ));
                write_inventory_save(&mut contents, &format!("block_entity_{index}"), inventory);
            }
        }
    }

    fs::write(path, contents)?;
    Ok(())
}

fn write_inventory_save(contents: &mut String, prefix: &str, inventory: &InventorySave) {
    contents.push_str(&format!("{prefix}_slot_count={}\n", inventory.slots.len()));
    for (slot_index, slot) in inventory.slots.iter().enumerate() {
        if let Some(stack) = slot {
            contents.push_str(&format!(
                "{prefix}_slot_{slot_index}={},{}\n",
                escape_value(&stack.item_key),
                stack.count
            ));
        }
    }
}

fn read_block_entities_file(path: &Path) -> Result<Vec<BlockEntitySave>, WorldSaveError> {
    let contents = fs::read_to_string(path)?;
    let mut values = std::collections::HashMap::new();
    for line in contents.lines() {
        let Some((key, value)) = line.split_once('=') else {
            continue;
        };
        values.insert(key, value);
    }

    if values.get("version").copied() != Some("1") {
        return Err(WorldSaveError::InvalidMetadata(
            "unsupported block entity save version".to_string(),
        ));
    }

    let count = values
        .get("block_entity_count")
        .and_then(|value| value.parse::<usize>().ok())
        .unwrap_or(0);
    let mut entities = Vec::with_capacity(count);
    for index in 0..count {
        let prefix = format!("block_entity_{index}");
        let position = read_block_entity_position(&values, &prefix)?;
        let kind = match values.get(format!("{prefix}_kind").as_str()).copied() {
            Some("chest") => BlockEntityKindSave::Chest {
                inventory: read_inventory_save_with_prefix(
                    &values,
                    &prefix,
                    values
                        .get(format!("{prefix}_slot_count").as_str())
                        .and_then(|value| value.parse::<usize>().ok())
                        .unwrap_or(27),
                )?,
            },
            Some("furnace") => BlockEntityKindSave::Furnace {
                inventory: read_inventory_save_with_prefix(
                    &values,
                    &prefix,
                    values
                        .get(format!("{prefix}_slot_count").as_str())
                        .and_then(|value| value.parse::<usize>().ok())
                        .unwrap_or(3),
                )?,
                burn_ticks: required_parse(&values, &format!("{prefix}_burn_ticks"))?,
                fuel_ticks: required_parse(&values, &format!("{prefix}_fuel_ticks"))?,
                cook_ticks: required_parse(&values, &format!("{prefix}_cook_ticks"))?,
                cook_ticks_total: required_parse(&values, &format!("{prefix}_cook_ticks_total"))?,
            },
            Some(other) => {
                return Err(WorldSaveError::InvalidMetadata(format!(
                    "invalid {prefix}_kind {other}"
                )));
            }
            None => {
                return Err(WorldSaveError::InvalidMetadata(format!(
                    "missing {prefix}_kind"
                )));
            }
        };
        entities.push(BlockEntitySave { position, kind });
    }
    Ok(entities)
}

fn read_block_entity_position(
    values: &std::collections::HashMap<&str, &str>,
    prefix: &str,
) -> Result<BlockEntityPositionSave, WorldSaveError> {
    let value = required_string(values, &format!("{prefix}_position"))?;
    let mut parts = value.split(',');
    let x = parts
        .next()
        .ok_or_else(|| WorldSaveError::InvalidMetadata(format!("invalid {prefix}_position")))?
        .parse()
        .map_err(|_| WorldSaveError::InvalidMetadata(format!("invalid {prefix}_position x")))?;
    let y = parts
        .next()
        .ok_or_else(|| WorldSaveError::InvalidMetadata(format!("invalid {prefix}_position")))?
        .parse()
        .map_err(|_| WorldSaveError::InvalidMetadata(format!("invalid {prefix}_position y")))?;
    let z = parts
        .next()
        .ok_or_else(|| WorldSaveError::InvalidMetadata(format!("invalid {prefix}_position")))?
        .parse()
        .map_err(|_| WorldSaveError::InvalidMetadata(format!("invalid {prefix}_position z")))?;
    Ok(BlockEntityPositionSave { x, y, z })
}

fn required_string(
    values: &std::collections::HashMap<&str, &str>,
    key: &str,
) -> Result<String, WorldSaveError> {
    values
        .get(key)
        .map(|value| (*value).to_string())
        .ok_or_else(|| WorldSaveError::InvalidMetadata(format!("missing {key}")))
}

fn required_parse<T: std::str::FromStr>(
    values: &std::collections::HashMap<&str, &str>,
    key: &str,
) -> Result<T, WorldSaveError> {
    values
        .get(key)
        .ok_or_else(|| WorldSaveError::InvalidMetadata(format!("missing {key}")))?
        .parse()
        .map_err(|_| WorldSaveError::InvalidMetadata(format!("invalid {key}")))
}

fn chunk_block_palette(
    chunk: &Chunk,
    blocks: &BlockRegistry,
) -> Result<(Vec<BlockState>, Vec<u16>), WorldSaveError> {
    let mut palette = Vec::new();
    let mut indices = Vec::with_capacity(CHUNK_VOLUME);
    let mut state_to_palette = std::collections::HashMap::new();

    for state in chunk.block_states() {
        if blocks.get(state.block).is_none() {
            return Err(WorldSaveError::InvalidChunk(format!(
                "unknown block id {}",
                state.block.raw()
            )));
        }
        let palette_index = if let Some(index) = state_to_palette.get(state).copied() {
            index
        } else {
            let index = u16::try_from(palette.len())
                .map_err(|_| WorldSaveError::InvalidChunk("block palette too large".to_string()))?;
            palette.push(*state);
            state_to_palette.insert(*state, index);
            index
        };
        indices.push(palette_index);
    }

    Ok((palette, indices))
}

fn sanitize_display_name(name: &str) -> String {
    let trimmed = name.trim();
    if trimmed.is_empty() {
        "New World".to_string()
    } else {
        trimmed.chars().take(64).collect()
    }
}

pub fn default_world_name(existing_count: usize) -> String {
    format!("World {}", existing_count + 1)
}

pub fn new_world_seed(existing_count: usize) -> u64 {
    unix_now_seconds()
        .wrapping_mul(0x9E37_79B9_7F4A_7C15)
        .wrapping_add(existing_count as u64)
}

fn slugify(name: &str) -> String {
    let mut slug = String::new();
    for character in name.chars().flat_map(char::to_lowercase) {
        if character.is_ascii_alphanumeric() {
            slug.push(character);
        } else if !slug.ends_with('-') {
            slug.push('-');
        }
    }
    let slug = slug.trim_matches('-');
    if slug.is_empty() {
        "world".to_string()
    } else {
        slug.to_string()
    }
}

fn escape_value(value: &str) -> String {
    value.replace('\\', "\\\\").replace('\n', "\\n")
}

fn unescape_value(value: &str) -> String {
    let mut output = String::new();
    let mut chars = value.chars();
    while let Some(character) = chars.next() {
        if character == '\\' {
            match chars.next() {
                Some('n') => output.push('\n'),
                Some('\\') => output.push('\\'),
                Some(other) => {
                    output.push('\\');
                    output.push(other);
                }
                None => output.push('\\'),
            }
        } else {
            output.push(character);
        }
    }
    output
}

fn unix_now_seconds() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs())
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engine::world::{
        BlockDefinition, BlockPosition, Chunk, HorizontalDirection, PlacementRuleKind, StairHalf,
    };

    fn temp_save_root(name: &str) -> PathBuf {
        std::env::temp_dir().join(format!(
            "humancraft-save-test-{name}-{}",
            std::process::id()
        ))
    }

    fn test_blocks(keys: &[&str]) -> BlockRegistry {
        let mut blocks = BlockRegistry::new();
        for key in keys {
            blocks
                .register(BlockDefinition::new(*key, *key))
                .expect("test block keys should be unique");
        }
        blocks
    }

    #[test]
    fn world_metadata_round_trips_seed_and_player_position() {
        let root = temp_save_root("metadata");
        let _ = fs::remove_dir_all(&root);
        let store = WorldSaveStore::new(&root);

        let metadata = store
            .create_world(
                "Seeded Test",
                12345,
                PlayerSave::new(1.0, 72.5, -3.25, 0.4, -0.2),
            )
            .unwrap();
        let loaded = store.load_metadata(&metadata.id).unwrap();

        assert_eq!(loaded.name, "Seeded Test");
        assert_eq!(loaded.seed, 12345);
        assert_eq!(loaded.player, PlayerSave::new(1.0, 72.5, -3.25, 0.4, -0.2));

        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn world_metadata_round_trips_inventory_slots() {
        let root = temp_save_root("inventory");
        let _ = fs::remove_dir_all(&root);
        let store = WorldSaveStore::new(&root);

        let mut metadata = store
            .create_world(
                "Inventory Test",
                12345,
                PlayerSave::new(1.0, 72.5, -3.25, 0.4, -0.2),
            )
            .unwrap();
        metadata.inventory.slots[0] = Some(ItemStackSave::new("humancraft:dirt", 64));
        metadata.inventory.slots[9] = Some(ItemStackSave::new("humancraft:diamond", 3));
        store.save_metadata(&metadata).unwrap();

        let loaded = store.load_metadata(&metadata.id).unwrap();

        assert_eq!(
            loaded.inventory.slots[0],
            Some(ItemStackSave::new("humancraft:dirt", 64))
        );
        assert_eq!(
            loaded.inventory.slots[9],
            Some(ItemStackSave::new("humancraft:diamond", 3))
        );

        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn block_entities_round_trip_chest_and_furnace_state() {
        let root = temp_save_root("block-entities");
        let _ = fs::remove_dir_all(&root);
        let store = WorldSaveStore::new(&root);
        let metadata = store
            .create_world(
                "Block Entity Test",
                12345,
                PlayerSave::new(1.0, 72.5, -3.25, 0.4, -0.2),
            )
            .unwrap();
        let mut chest_inventory = InventorySave::empty(27);
        chest_inventory.slots[0] = Some(ItemStackSave::new("humancraft:dirt", 12));
        let mut furnace_inventory = InventorySave::empty(3);
        furnace_inventory.slots[0] = Some(ItemStackSave::new("humancraft:sand", 3));
        furnace_inventory.slots[1] = Some(ItemStackSave::new("humancraft:coal", 1));

        let entities = vec![
            BlockEntitySave::chest(BlockEntityPositionSave::new(1, 64, 2), chest_inventory),
            BlockEntitySave::furnace(
                BlockEntityPositionSave::new(-3, 70, 5),
                furnace_inventory,
                99,
                1600,
                42,
                200,
            ),
        ];

        store.save_block_entities(&metadata.id, &entities).unwrap();
        let loaded = store.load_block_entities(&metadata.id).unwrap();

        assert_eq!(loaded, entities);

        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn chunk_files_round_trip_block_changes() {
        let root = temp_save_root("chunk");
        let _ = fs::remove_dir_all(&root);
        let store = WorldSaveStore::new(&root);
        let metadata = store
            .create_world("Chunk Test", 7, PlayerSave::new(0.0, 0.0, 0.0, 0.0, 0.0))
            .unwrap();
        let blocks = test_blocks(&["test:air", "test:stone"]);
        let air = blocks.id_for_key("test:air").unwrap();
        let stone = blocks.id_for_key("test:stone").unwrap();
        let mut chunk = Chunk::filled(ChunkPosition { x: -1, z: 2 }, air);
        chunk
            .set_block(BlockPosition { x: 15, y: 64, z: 0 }, stone)
            .unwrap();

        store.save_chunk(&metadata.id, &chunk, &blocks).unwrap();
        let loaded = store
            .load_chunk(&metadata.id, ChunkPosition { x: -1, z: 2 }, &blocks)
            .unwrap()
            .unwrap();

        assert_eq!(
            loaded.block(BlockPosition { x: 15, y: 64, z: 0 }),
            Some(stone)
        );

        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn chunk_files_round_trip_block_state_properties() {
        let root = temp_save_root("chunk-state");
        let _ = fs::remove_dir_all(&root);
        let store = WorldSaveStore::new(&root);
        let metadata = store
            .create_world(
                "Chunk State Test",
                7,
                PlayerSave::new(0.0, 0.0, 0.0, 0.0, 0.0),
            )
            .unwrap();
        let mut blocks = BlockRegistry::new();
        let air = blocks
            .register(BlockDefinition::new("test:air", "Air"))
            .unwrap();
        let stairs = blocks
            .register(
                BlockDefinition::new("test:stairs", "Stairs").placement(PlacementRuleKind::Stairs),
            )
            .unwrap();
        let mut chunk = Chunk::filled(ChunkPosition { x: 0, z: 0 }, air);
        let state = BlockState::with_properties(
            stairs,
            BlockProperties::Stairs {
                facing: HorizontalDirection::East,
                half: StairHalf::Top,
            },
        );
        chunk
            .set_block_state(BlockPosition { x: 2, y: 3, z: 4 }, state)
            .unwrap();

        store.save_chunk(&metadata.id, &chunk, &blocks).unwrap();
        let loaded = store
            .load_chunk(&metadata.id, ChunkPosition { x: 0, z: 0 }, &blocks)
            .unwrap()
            .unwrap();

        assert_eq!(
            loaded.block_state(BlockPosition { x: 2, y: 3, z: 4 }),
            Some(state)
        );

        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn chunk_files_resolve_blocks_by_key_not_registry_order() {
        let root = temp_save_root("chunk-keyed");
        let _ = fs::remove_dir_all(&root);
        let store = WorldSaveStore::new(&root);
        let metadata = store
            .create_world(
                "Keyed Chunk Test",
                7,
                PlayerSave::new(0.0, 0.0, 0.0, 0.0, 0.0),
            )
            .unwrap();
        let old_blocks = test_blocks(&["test:air", "test:stone"]);
        let old_air = old_blocks.id_for_key("test:air").unwrap();
        let old_stone = old_blocks.id_for_key("test:stone").unwrap();
        let mut chunk = Chunk::filled(ChunkPosition { x: 0, z: 0 }, old_air);
        chunk
            .set_block(BlockPosition { x: 2, y: 3, z: 4 }, old_stone)
            .unwrap();

        store.save_chunk(&metadata.id, &chunk, &old_blocks).unwrap();

        let new_blocks = test_blocks(&["test:stone", "test:air"]);
        let loaded = store
            .load_chunk(&metadata.id, ChunkPosition { x: 0, z: 0 }, &new_blocks)
            .unwrap()
            .unwrap();

        assert_eq!(
            loaded.block(BlockPosition { x: 2, y: 3, z: 4 }),
            new_blocks.id_for_key("test:stone")
        );
        assert_eq!(
            loaded.block(BlockPosition { x: 0, y: 0, z: 0 }),
            new_blocks.id_for_key("test:air")
        );

        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn legacy_raw_id_chunks_are_ignored_for_regeneration() {
        let root = temp_save_root("chunk-legacy");
        let _ = fs::remove_dir_all(&root);
        let store = WorldSaveStore::new(&root);
        let metadata = store
            .create_world(
                "Legacy Chunk Test",
                7,
                PlayerSave::new(0.0, 0.0, 0.0, 0.0, 0.0),
            )
            .unwrap();
        let position = ChunkPosition { x: 3, z: -2 };
        let path = store.chunk_path(&metadata.id, position);
        let mut bytes = Vec::with_capacity(LEGACY_RAW_ID_CHUNK_MAGIC.len() + 8 + CHUNK_VOLUME * 4);
        bytes.extend_from_slice(LEGACY_RAW_ID_CHUNK_MAGIC);
        bytes.extend_from_slice(&position.x.to_le_bytes());
        bytes.extend_from_slice(&position.z.to_le_bytes());
        for _ in 0..CHUNK_VOLUME {
            bytes.extend_from_slice(&0_u32.to_le_bytes());
        }
        fs::write(path, bytes).unwrap();

        let blocks = test_blocks(&["test:air"]);

        assert!(
            store
                .load_chunk(&metadata.id, position, &blocks)
                .unwrap()
                .is_none()
        );

        let _ = fs::remove_dir_all(root);
    }
}
