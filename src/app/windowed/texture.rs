use std::collections::HashMap;
use std::path::PathBuf;

use image::GenericImageView;

use crate::engine::mesh::chunk_mesher::FaceDirection;
use crate::engine::world::{
    Axis, BlockId, BlockProperties, BlockRegistry, BlockState, HorizontalDirection, ItemRegistry,
};

pub(super) struct TextureAtlas {
    _texture: wgpu::Texture,
    pub(super) view: wgpu::TextureView,
    pub(super) sampler: wgpu::Sampler,
    tiles: HashMap<String, AtlasTile>,
    fallback: AtlasTile,
}

impl TextureAtlas {
    pub(super) const TILE_SIZE: u32 = 16;

    pub(super) fn load(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        blocks: &BlockRegistry,
        items: &ItemRegistry,
    ) -> Self {
        let mut texture_keys = vec!["humancraft:missing".to_string()];
        for (_, definition) in blocks.iter() {
            for key in block_texture_keys(definition) {
                if key != "humancraft:missing" && !texture_keys.contains(&key.to_string()) {
                    texture_keys.push(key.to_string());
                }
            }
            for key in extra_block_texture_keys(definition) {
                if key != "humancraft:missing" && !texture_keys.contains(&key.to_string()) {
                    texture_keys.push(key.to_string());
                }
            }
        }
        for key in item_texture_keys(items) {
            if key != "humancraft:missing" && !texture_keys.contains(&key.to_string()) {
                texture_keys.push(key.to_string());
            }
        }
        for key in destroy_stage_texture_keys() {
            if !texture_keys.contains(&key.to_string()) {
                texture_keys.push(key.to_string());
            }
        }
        texture_keys.push(player_hand_texture_key());

        let width = Self::TILE_SIZE * texture_keys.len() as u32;
        let height = Self::TILE_SIZE;
        let mut atlas_pixels = vec![255; (width * height * 4) as usize];
        let mut tiles = HashMap::new();
        let mut atlas_loaded_counts = (0_usize, 0_usize);

        for (index, key) in texture_keys.iter().enumerate() {
            let tile_x = index as u32 * Self::TILE_SIZE;
            let (pixels, loaded) = match load_texture_pixels(key) {
                Some(pixels) => (pixels, true),
                None => (fallback_texture_pixels(key), false),
            };
            let loaded_count = usize::from(loaded);

            for y in 0..Self::TILE_SIZE {
                for x in 0..Self::TILE_SIZE {
                    let source = ((y * Self::TILE_SIZE + x) * 4) as usize;
                    let destination = ((y * width + tile_x + x) * 4) as usize;
                    atlas_pixels[destination..destination + 4]
                        .copy_from_slice(&pixels[source..source + 4]);
                }
            }

            let min_u = (tile_x as f32 + 0.5) / width as f32;
            let max_u = (tile_x as f32 + Self::TILE_SIZE as f32 - 0.5) / width as f32;
            tiles.insert(
                key.clone(),
                AtlasTile {
                    min_u,
                    max_u,
                    min_v: 0.5 / height as f32,
                    max_v: (Self::TILE_SIZE as f32 - 0.5) / height as f32,
                },
            );

            if loaded {
                println!("loaded block texture {key}");
            } else if key != "humancraft:missing" {
                eprintln!("missing texture {key}; using fallback texture");
            }

            atlas_loaded_counts.0 += loaded_count;
            atlas_loaded_counts.1 += usize::from(!loaded && key != "humancraft:missing");
        }

        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("Block Texture Atlas"),
            size: wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8UnormSrgb,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });
        queue.write_texture(
            wgpu::TexelCopyTextureInfo {
                texture: &texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            &atlas_pixels,
            wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(width * 4),
                rows_per_image: Some(height),
            },
            wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
        );

        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("Block Texture Sampler"),
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Nearest,
            min_filter: wgpu::FilterMode::Nearest,
            mipmap_filter: wgpu::FilterMode::Nearest,
            ..Default::default()
        });
        let fallback = *tiles
            .get("humancraft:missing")
            .expect("fallback texture should exist");
        println!(
            "texture atlas built: {} loaded, {} fallback",
            atlas_loaded_counts.0, atlas_loaded_counts.1
        );

        Self {
            _texture: texture,
            view,
            sampler,
            tiles,
            fallback,
        }
    }

    pub(super) fn tile(&self, key: &str) -> AtlasTile {
        self.tiles.get(key).copied().unwrap_or(self.fallback)
    }
}

#[derive(Debug, Copy, Clone)]
pub(super) struct AtlasTile {
    min_u: f32,
    max_u: f32,
    min_v: f32,
    max_v: f32,
}

impl AtlasTile {
    pub(super) fn uv_quad(self) -> [[f32; 2]; 4] {
        [
            [self.min_u, self.max_v],
            [self.max_u, self.max_v],
            [self.max_u, self.min_v],
            [self.min_u, self.min_v],
        ]
    }
}

pub(super) struct Texture {
    pub(super) view: wgpu::TextureView,
}

impl Texture {
    pub(super) const DEPTH_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Depth32Float;

    pub(super) fn create_depth_texture(
        device: &wgpu::Device,
        config: &wgpu::SurfaceConfiguration,
    ) -> Self {
        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("Depth Texture"),
            size: wgpu::Extent3d {
                width: config.width,
                height: config.height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: Self::DEPTH_FORMAT,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            view_formats: &[],
        });
        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
        Self { view }
    }
}

pub(super) fn block_color(block: BlockId, blocks: &BlockRegistry) -> [f32; 3] {
    let Some(definition) = blocks.get(block) else {
        return [1.0, 0.0, 1.0];
    };
    match definition.key.as_str() {
        "humancraft:grass" => [0.22, 0.62, 0.18],
        "humancraft:dirt" => [0.42, 0.25, 0.12],
        "humancraft:stone" => [0.45, 0.45, 0.45],
        "humancraft:cobblestone" => [0.36, 0.36, 0.36],
        "humancraft:coal_ore" => [0.10, 0.10, 0.10],
        "humancraft:iron_ore" => [0.70, 0.42, 0.28],
        "humancraft:gold_ore" => [0.95, 0.72, 0.18],
        "humancraft:diamond_ore" => [0.20, 0.80, 0.90],
        "humancraft:oak_log" => [0.45, 0.28, 0.12],
        "humancraft:oak_leaves" => [0.18, 0.42, 0.14],
        "humancraft:sand" => [0.86, 0.78, 0.52],
        "humancraft:sandstone" => [0.72, 0.65, 0.43],
        "humancraft:bedrock" => [0.20, 0.20, 0.22],
        _ => [0.8, 0.8, 0.8],
    }
}

pub(super) fn shaded_block_color(
    block: BlockId,
    direction: FaceDirection,
    blocks: &BlockRegistry,
) -> [f32; 3] {
    let base = block_color(block, blocks);
    let shade: f32 = match direction {
        FaceDirection::Up => 1.15,
        FaceDirection::North => 0.85,
        FaceDirection::South => 0.95,
        FaceDirection::East => 0.75,
        FaceDirection::West => 0.80,
        FaceDirection::Down => 0.55,
    };

    [
        (base[0] * shade).min(1.0),
        (base[1] * shade).min(1.0),
        (base[2] * shade).min(1.0),
    ]
}

pub(super) fn render_material(
    block: BlockId,
    state: BlockState,
    direction: FaceDirection,
    blocks: &BlockRegistry,
) -> ([f32; 3], String) {
    let shade: f32 = match direction {
        FaceDirection::Up => 1.15,
        FaceDirection::North => 0.85,
        FaceDirection::South => 0.95,
        FaceDirection::East => 0.75,
        FaceDirection::West => 0.80,
        FaceDirection::Down => 0.55,
    };
    let Some(definition) = blocks.get(block) else {
        return ([1.0, 0.0, 1.0], "humancraft:missing".to_string());
    };

    let texture_key = texture_key_for_state_direction(definition, state, direction).to_string();
    if texture_key == "humancraft:missing" {
        return (shaded_block_color(block, direction, blocks), texture_key);
    }

    ([shade.min(1.0); 3], texture_key)
}

pub(super) fn texture_key_for_state_direction(
    definition: &crate::engine::world::BlockDefinition,
    state: BlockState,
    direction: FaceDirection,
) -> &str {
    if let BlockProperties::Axis { axis } = state.properties {
        return texture_key_for_axis_direction(definition, axis, direction);
    }
    let facing = match state.properties {
        BlockProperties::HorizontalFacing { facing } => Some(facing),
        BlockProperties::Furnace { facing, lit } => {
            if lit
                && rotate_face_to_block_front(direction, facing) == FaceDirection::North
                && definition.key == "humancraft:furnace"
            {
                return "humancraft:block/furnace/front_on";
            }
            Some(facing)
        }
        _ => None,
    };
    let Some(facing) = facing else {
        return texture_key_for_direction(definition, direction);
    };
    match rotate_face_to_block_front(direction, facing) {
        FaceDirection::North => &definition.textures.north,
        FaceDirection::South => &definition.textures.south,
        FaceDirection::East => &definition.textures.east,
        FaceDirection::West => &definition.textures.west,
        FaceDirection::Up => &definition.textures.top,
        FaceDirection::Down => &definition.textures.bottom,
    }
}

fn texture_key_for_axis_direction(
    definition: &crate::engine::world::BlockDefinition,
    axis: Axis,
    direction: FaceDirection,
) -> &str {
    let is_cap_face = match axis {
        Axis::X => matches!(direction, FaceDirection::East | FaceDirection::West),
        Axis::Y => matches!(direction, FaceDirection::Up | FaceDirection::Down),
        Axis::Z => matches!(direction, FaceDirection::North | FaceDirection::South),
    };
    if is_cap_face {
        &definition.textures.top
    } else {
        &definition.textures.north
    }
}

fn rotate_face_to_block_front(
    world_direction: FaceDirection,
    facing: HorizontalDirection,
) -> FaceDirection {
    match world_direction {
        FaceDirection::Up | FaceDirection::Down => world_direction,
        FaceDirection::North => match facing {
            HorizontalDirection::North => FaceDirection::North,
            HorizontalDirection::South => FaceDirection::South,
            HorizontalDirection::East => FaceDirection::West,
            HorizontalDirection::West => FaceDirection::East,
        },
        FaceDirection::South => match facing {
            HorizontalDirection::North => FaceDirection::South,
            HorizontalDirection::South => FaceDirection::North,
            HorizontalDirection::East => FaceDirection::East,
            HorizontalDirection::West => FaceDirection::West,
        },
        FaceDirection::East => match facing {
            HorizontalDirection::North => FaceDirection::East,
            HorizontalDirection::South => FaceDirection::West,
            HorizontalDirection::East => FaceDirection::North,
            HorizontalDirection::West => FaceDirection::South,
        },
        FaceDirection::West => match facing {
            HorizontalDirection::North => FaceDirection::West,
            HorizontalDirection::South => FaceDirection::East,
            HorizontalDirection::East => FaceDirection::South,
            HorizontalDirection::West => FaceDirection::North,
        },
    }
}

pub(super) fn texture_key_for_direction(
    definition: &crate::engine::world::BlockDefinition,
    direction: FaceDirection,
) -> &str {
    match direction {
        FaceDirection::North => &definition.textures.north,
        FaceDirection::South => &definition.textures.south,
        FaceDirection::East => &definition.textures.east,
        FaceDirection::West => &definition.textures.west,
        FaceDirection::Up => &definition.textures.top,
        FaceDirection::Down => &definition.textures.bottom,
    }
}

pub(super) fn block_texture_keys(definition: &crate::engine::world::BlockDefinition) -> [&str; 6] {
    [
        &definition.textures.top,
        &definition.textures.bottom,
        &definition.textures.north,
        &definition.textures.south,
        &definition.textures.east,
        &definition.textures.west,
    ]
}

fn extra_block_texture_keys(
    definition: &crate::engine::world::BlockDefinition,
) -> impl Iterator<Item = &'static str> {
    if definition.key == "humancraft:furnace" {
        Some("humancraft:block/furnace/front_on").into_iter()
    } else {
        None.into_iter()
    }
}

fn item_texture_keys(items: &ItemRegistry) -> Vec<&str> {
    items
        .iter()
        .map(|(_, definition)| definition.texture.as_str())
        .collect()
}

pub(super) fn load_texture_pixels(key: &str) -> Option<Vec<u8>> {
    let path = texture_path(key)?;
    let image = image::open(path).ok()?;
    if image.dimensions() != (TextureAtlas::TILE_SIZE, TextureAtlas::TILE_SIZE) {
        return None;
    }
    Some(image.to_rgba8().into_raw())
}

pub(super) fn texture_path(key: &str) -> Option<PathBuf> {
    if let Some(path) = key.strip_prefix("humancraft:block/") {
        let (block, face) = path.split_once('/')?;
        return Some(
            PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                .join("textures")
                .join("blocks")
                .join(block)
                .join(format!("{face}.png")),
        );
    }

    if let Some(stage) = key.strip_prefix("humancraft:destroy/") {
        return Some(
            PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                .join("textures")
                .join("overlays")
                .join(format!("destroy_stage_{stage}.png")),
        );
    }

    if key == player_hand_texture_key() {
        return Some(
            PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                .join("textures")
                .join("overlays")
                .join("player_hand.png"),
        );
    }

    let item = key.strip_prefix("humancraft:item/")?;
    Some(
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("textures")
            .join("items")
            .join(format!("{item}.png")),
    )
}

pub(super) fn destroy_stage_texture_key(stage: u8) -> String {
    format!("humancraft:destroy/{}", stage.min(9))
}

pub(super) fn player_hand_texture_key() -> String {
    "humancraft:overlay/player_hand".to_string()
}

fn destroy_stage_texture_keys() -> impl Iterator<Item = String> {
    (0_u8..=9).map(destroy_stage_texture_key)
}

fn fallback_texture_pixels(key: &str) -> Vec<u8> {
    let color = if key == "humancraft:missing" {
        [255, 255, 255, 255]
    } else {
        [255, 0, 255, 255]
    };
    let mut pixels =
        Vec::with_capacity((TextureAtlas::TILE_SIZE * TextureAtlas::TILE_SIZE * 4) as usize);
    for _ in 0..TextureAtlas::TILE_SIZE * TextureAtlas::TILE_SIZE {
        pixels.extend_from_slice(&color);
    }
    pixels
}
