//! Native windowed client using winit and wgpu.

use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Instant;

use glam::{Mat4, Vec3};
use image::GenericImageView;
use wgpu::util::DeviceExt;
use winit::application::ApplicationHandler;
use winit::dpi::{PhysicalPosition, PhysicalSize};
use winit::event::{DeviceEvent, ElementState, Ime, KeyEvent, MouseButton, WindowEvent};
use winit::event_loop::{ActiveEventLoop, ControlFlow, EventLoop};
use winit::keyboard::{Key, KeyCode, NamedKey, PhysicalKey};
use winit::window::{CursorGrabMode, Window, WindowAttributes, WindowId};

use crate::content::{bootstrap_content, default_generation_pipeline};
use crate::engine::mesh::chunk_mesher::ChunkMesh;
use crate::engine::world::generation::GenerationContext;
use crate::engine::world::save::{
    PlayerSave, WorldMetadata, WorldSaveError, WorldSaveStore, default_world_name, new_world_seed,
};
use crate::engine::world::{
    BlockId, BlockRegistry, ChunkPosition, Inventory, ItemRegistry, ItemStack, LootEntity,
};

mod client_world;
mod constants;
mod inventory_interaction;
mod loot;
mod player_collision;
mod spatial;

use client_world::ClientWorld;
use constants::*;
use inventory_interaction::{
    InventoryDrag, InventoryMouseButton, distribute_carried_stack_evenly, inventory_from_save,
    inventory_to_save, left_click_inventory_slot, place_one_carried_item,
    right_click_inventory_slot,
};
use spatial::{WorldBlockPosition, chunk_position_for_render_position};

const SHADER: &str = r#"
struct Camera {
    view_proj: mat4x4<f32>,
};

@group(0) @binding(0)
var<uniform> camera: Camera;

struct VertexInput {
    @location(0) position: vec3<f32>,
    @location(1) color: vec3<f32>,
    @location(2) tex_coords: vec2<f32>,
};

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) color: vec3<f32>,
    @location(1) tex_coords: vec2<f32>,
};

@group(1) @binding(0)
var block_texture: texture_2d<f32>;

@group(1) @binding(1)
var block_sampler: sampler;

@vertex
fn vs_main(input: VertexInput) -> VertexOutput {
    var output: VertexOutput;
    output.clip_position = camera.view_proj * vec4<f32>(input.position, 1.0);
    output.color = input.color;
    output.tex_coords = input.tex_coords;
    return output;
}

@fragment
fn fs_main(input: VertexOutput) -> @location(0) vec4<f32> {
    let texel = textureSample(block_texture, block_sampler, input.tex_coords);
    if texel.a < 0.1 {
        discard;
    }
    return vec4<f32>(texel.rgb * input.color, texel.a);
}
"#;

const LINE_SHADER: &str = r#"
struct Camera {
    view_proj: mat4x4<f32>,
};

@group(0) @binding(0)
var<uniform> camera: Camera;

struct VertexInput {
    @location(0) position: vec3<f32>,
    @location(1) color: vec3<f32>,
    @location(2) tex_coords: vec2<f32>,
};

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) color: vec3<f32>,
};

@vertex
fn vs_main(input: VertexInput) -> VertexOutput {
    var output: VertexOutput;
    output.clip_position = camera.view_proj * vec4<f32>(input.position, 1.0);
    output.color = input.color;
    return output;
}

@fragment
fn fs_main(input: VertexOutput) -> @location(0) vec4<f32> {
    return vec4<f32>(input.color, 1.0);
}
"#;

const UI_SHADER: &str = r#"
struct VertexInput {
    @location(0) position: vec3<f32>,
    @location(1) color: vec3<f32>,
};

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) color: vec3<f32>,
};

@vertex
fn vs_main(input: VertexInput) -> VertexOutput {
    var output: VertexOutput;
    output.clip_position = vec4<f32>(input.position, 1.0);
    output.color = input.color;
    return output;
}

@fragment
fn fs_main(input: VertexOutput) -> @location(0) vec4<f32> {
    return vec4<f32>(input.color, 1.0);
}
"#;

const TEXTURED_UI_SHADER: &str = r#"
struct VertexInput {
    @location(0) position: vec3<f32>,
    @location(1) color: vec3<f32>,
    @location(2) tex_coords: vec2<f32>,
};

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) color: vec3<f32>,
    @location(1) tex_coords: vec2<f32>,
};

@group(0) @binding(0)
var ui_texture: texture_2d<f32>;

@group(0) @binding(1)
var ui_sampler: sampler;

@vertex
fn vs_main(input: VertexInput) -> VertexOutput {
    var output: VertexOutput;
    output.clip_position = vec4<f32>(input.position, 1.0);
    output.color = input.color;
    output.tex_coords = input.tex_coords;
    return output;
}

@fragment
fn fs_main(input: VertexOutput) -> @location(0) vec4<f32> {
    let texel = textureSample(ui_texture, ui_sampler, input.tex_coords);
    if texel.a < 0.1 {
        discard;
    }
    return vec4<f32>(texel.rgb * input.color, texel.a);
}
"#;

pub fn run_windowed_game() {
    let event_loop = EventLoop::new().expect("event loop should be created");
    event_loop.set_control_flow(ControlFlow::Poll);

    let mut app = WindowedApp::default();
    event_loop
        .run_app(&mut app)
        .expect("windowed game should run");
}

#[derive(Default)]
struct WindowedApp {
    state: Option<RenderState>,
}

impl ApplicationHandler for WindowedApp {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.state.is_some() {
            return;
        }

        let window = Arc::new(
            event_loop
                .create_window(
                    WindowAttributes::default()
                        .with_title("HumanCraft")
                        .with_inner_size(PhysicalSize::new(1280, 720)),
                )
                .expect("window should be created"),
        );

        self.state = Some(pollster::block_on(RenderState::new(window)));
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, id: WindowId, event: WindowEvent) {
        let Some(state) = self.state.as_mut() else {
            return;
        };
        if id != state.window.id() {
            return;
        }

        match event {
            WindowEvent::CloseRequested => {
                state.flush_active_world_to_disk();
                event_loop.exit();
            }
            WindowEvent::Resized(size) => state.resize(size),
            WindowEvent::KeyboardInput { event, .. } => {
                if state.handle_key(&event) {
                    return;
                }
            }
            WindowEvent::Ime(Ime::Commit(text)) => state.handle_text_input(&text),
            WindowEvent::CursorMoved { position, .. } => state.handle_cursor_moved(position),
            WindowEvent::MouseInput {
                state: mouse_state,
                button,
                ..
            } => state.handle_mouse_button(button, mouse_state),
            WindowEvent::Focused(false) => state.handle_focus_lost(),
            WindowEvent::RedrawRequested => {
                state.update();
                match state.render() {
                    Ok(()) => {}
                    Err(wgpu::SurfaceError::Lost | wgpu::SurfaceError::Outdated) => {
                        state.resize(state.size)
                    }
                    Err(wgpu::SurfaceError::OutOfMemory) => event_loop.exit(),
                    Err(wgpu::SurfaceError::Timeout) => {}
                    Err(wgpu::SurfaceError::Other) => {}
                }
            }
            _ => {}
        }
    }

    fn device_event(
        &mut self,
        _event_loop: &ActiveEventLoop,
        _device_id: winit::event::DeviceId,
        event: DeviceEvent,
    ) {
        let Some(state) = self.state.as_mut() else {
            return;
        };

        if let DeviceEvent::MouseMotion { delta } = event {
            state.handle_mouse_motion(delta.0 as f32, delta.1 as f32);
        }
    }

    fn about_to_wait(&mut self, _event_loop: &ActiveEventLoop) {
        if let Some(state) = &self.state {
            state.window.request_redraw();
        }
    }
}

struct RenderState {
    window: Arc<Window>,
    surface: wgpu::Surface<'static>,
    device: wgpu::Device,
    queue: wgpu::Queue,
    config: wgpu::SurfaceConfiguration,
    size: PhysicalSize<u32>,
    render_pipeline: wgpu::RenderPipeline,
    ui_pipeline: wgpu::RenderPipeline,
    textured_ui_pipeline: wgpu::RenderPipeline,
    line_pipeline: wgpu::RenderPipeline,
    texture_atlas: TextureAtlas,
    texture_bind_group: wgpu::BindGroup,
    camera_buffer: wgpu::Buffer,
    camera_bind_group: wgpu::BindGroup,
    chunk_buffers: HashMap<ChunkPosition, ChunkRenderBuffer>,
    pending_chunk_remeshes: HashSet<ChunkPosition>,
    crosshair_vertex_buffer: wgpu::Buffer,
    crosshair_index_buffer: wgpu::Buffer,
    crosshair_index_count: u32,
    outline_vertex_buffer: wgpu::Buffer,
    outline_vertex_count: u32,
    depth_texture: Texture,
    camera: Camera,
    world: Option<ClientWorld>,
    save_store: WorldSaveStore,
    worlds: Vec<WorldMetadata>,
    active_world: Option<WorldMetadata>,
    mode: AppMode,
    selected_world: usize,
    text_entry: TextEntry,
    new_world_config: NewWorldConfig,
    dirty_save_chunks: HashSet<ChunkPosition>,
    player_state_dirty: bool,
    cursor_position: PhysicalPosition<f64>,
    targeted_block: Option<WorldBlockPosition>,
    input: InputState,
    paused: bool,
    inventory_open: bool,
    inventory_cursor: Option<ItemStack>,
    inventory_drag: Option<InventoryDrag>,
    held_block_interaction: HeldBlockInteraction,
    selected_hotbar_slot: usize,
    last_frame: Instant,
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
enum AppMode {
    MainMenu,
    ManageWorlds,
    ConfigNewWorld,
    RenamingWorld,
    InGame,
}

#[derive(Debug, Default, Copy, Clone)]
struct HeldBlockInteraction {
    button: Option<MouseButton>,
    repeat_seconds: f32,
}

impl HeldBlockInteraction {
    fn press(&mut self, button: MouseButton) {
        self.button = Some(button);
        self.repeat_seconds = 0.0;
    }

    fn release(&mut self, button: MouseButton) {
        if self.button == Some(button) {
            self.clear();
        }
    }

    fn clear(&mut self) {
        self.button = None;
        self.repeat_seconds = 0.0;
    }

    fn repeat_button(&mut self, delta_seconds: f32) -> Option<MouseButton> {
        let button = self.button?;
        self.repeat_seconds += delta_seconds;
        if self.repeat_seconds < BLOCK_INTERACTION_REPEAT_SECONDS {
            return None;
        }
        self.repeat_seconds = 0.0;
        Some(button)
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
enum ConfigField {
    Name,
    Seed,
}

#[derive(Debug, Clone)]
struct NewWorldConfig {
    name: String,
    seed: String,
    focused: ConfigField,
}

impl Default for NewWorldConfig {
    fn default() -> Self {
        Self {
            name: String::new(),
            seed: String::new(),
            focused: ConfigField::Name,
        }
    }
}

impl NewWorldConfig {
    fn start(&mut self, fallback_name: String) {
        self.name = fallback_name;
        self.seed.clear();
        self.focused = ConfigField::Name;
    }

    fn push(&mut self, text: &str) {
        let target = match self.focused {
            ConfigField::Name => &mut self.name,
            ConfigField::Seed => &mut self.seed,
        };
        for character in text.chars() {
            if !character.is_control() && target.chars().count() < 64 {
                target.push(character);
            }
        }
    }

    fn pop(&mut self) {
        match self.focused {
            ConfigField::Name => {
                self.name.pop();
            }
            ConfigField::Seed => {
                self.seed.pop();
            }
        }
    }

    fn toggle_focus(&mut self) {
        self.focused = match self.focused {
            ConfigField::Name => ConfigField::Seed,
            ConfigField::Seed => ConfigField::Name,
        };
    }

    fn final_name(&self) -> String {
        let trimmed = self.name.trim();
        if trimmed.is_empty() {
            "New World".to_string()
        } else {
            trimmed.to_string()
        }
    }
}

#[derive(Debug, Clone)]
struct TextEntry {
    value: String,
    fallback: String,
}

impl Default for TextEntry {
    fn default() -> Self {
        Self {
            value: String::new(),
            fallback: String::new(),
        }
    }
}

impl TextEntry {
    fn start(&mut self, fallback: impl Into<String>) {
        self.value.clear();
        self.fallback = fallback.into();
    }

    fn push(&mut self, text: &str) {
        for character in text.chars() {
            if !character.is_control() && self.value.chars().count() < 64 {
                self.value.push(character);
            }
        }
    }

    fn pop(&mut self) {
        self.value.pop();
    }

    fn finish(&self) -> String {
        let trimmed = self.value.trim();
        if trimmed.is_empty() {
            self.fallback.clone()
        } else {
            trimmed.to_string()
        }
    }

    fn display(&self) -> &str {
        if self.value.is_empty() {
            &self.fallback
        } else {
            &self.value
        }
    }
}

impl RenderState {
    async fn new(window: Arc<Window>) -> Self {
        let size = window.inner_size();
        let instance = wgpu::Instance::default();
        let surface = instance
            .create_surface(window.clone())
            .expect("surface should be created");
        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::HighPerformance,
                compatible_surface: Some(&surface),
                force_fallback_adapter: false,
            })
            .await
            .expect("adapter should be available");
        let (device, queue) = adapter
            .request_device(&wgpu::DeviceDescriptor {
                label: Some("HumanCraft Device"),
                required_features: wgpu::Features::empty(),
                required_limits: wgpu::Limits::default(),
                memory_hints: wgpu::MemoryHints::Performance,
                trace: wgpu::Trace::Off,
            })
            .await
            .expect("device should be available");

        let surface_caps = surface.get_capabilities(&adapter);
        let surface_format = surface_caps
            .formats
            .iter()
            .copied()
            .find(|format| format.is_srgb())
            .unwrap_or(surface_caps.formats[0]);
        let present_mode = surface_caps
            .present_modes
            .iter()
            .copied()
            .find(|mode| *mode == wgpu::PresentMode::Fifo)
            .unwrap_or(surface_caps.present_modes[0]);
        let alpha_mode = surface_caps.alpha_modes[0];
        let config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format: surface_format,
            width: size.width.max(1),
            height: size.height.max(1),
            present_mode,
            alpha_mode,
            view_formats: vec![],
            desired_maximum_frame_latency: 2,
        };
        surface.configure(&device, &config);

        let content = bootstrap_content().expect("content should bootstrap");
        let texture_atlas = TextureAtlas::load(&device, &queue, &content.blocks, &content.items);

        let camera = Camera::new(Vec3::new(0.0, PLAYER_STANDING_EYE_HEIGHT + 12.0, 0.0));
        let camera_uniform =
            CameraUniform::new(camera.view_projection(config.width, config.height));
        let camera_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Camera Buffer"),
            contents: bytemuck::bytes_of(&camera_uniform),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });
        let camera_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("Camera Bind Group Layout"),
                entries: &[wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::VERTEX,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                }],
            });
        let camera_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Camera Bind Group"),
            layout: &camera_bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: camera_buffer.as_entire_binding(),
            }],
        });

        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("HumanCraft Shader"),
            source: wgpu::ShaderSource::Wgsl(SHADER.into()),
        });
        let line_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("HumanCraft Line Shader"),
            source: wgpu::ShaderSource::Wgsl(LINE_SHADER.into()),
        });
        let ui_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("HumanCraft UI Shader"),
            source: wgpu::ShaderSource::Wgsl(UI_SHADER.into()),
        });
        let textured_ui_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("HumanCraft Textured UI Shader"),
            source: wgpu::ShaderSource::Wgsl(TEXTURED_UI_SHADER.into()),
        });
        let texture_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("Texture Atlas Bind Group Layout"),
                entries: &[
                    wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Texture {
                            multisampled: false,
                            view_dimension: wgpu::TextureViewDimension::D2,
                            sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 1,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                        count: None,
                    },
                ],
            });
        let texture_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Texture Atlas Bind Group"),
            layout: &texture_bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&texture_atlas.view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&texture_atlas.sampler),
                },
            ],
        });
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Render Pipeline Layout"),
            bind_group_layouts: &[&camera_bind_group_layout, &texture_bind_group_layout],
            push_constant_ranges: &[],
        });
        let ui_pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("UI Pipeline Layout"),
            bind_group_layouts: &[],
            push_constant_ranges: &[],
        });
        let textured_ui_pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("Textured UI Pipeline Layout"),
                bind_group_layouts: &[&texture_bind_group_layout],
                push_constant_ranges: &[],
            });
        let line_pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Line Pipeline Layout"),
            bind_group_layouts: &[&camera_bind_group_layout],
            push_constant_ranges: &[],
        });
        let depth_texture = Texture::create_depth_texture(&device, &config);
        let render_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("HumanCraft Render Pipeline"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_main"),
                buffers: &[Vertex::layout()],
                compilation_options: wgpu::PipelineCompilationOptions::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fs_main"),
                targets: &[Some(wgpu::ColorTargetState {
                    format: config.format,
                    blend: Some(wgpu::BlendState::REPLACE),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: wgpu::PipelineCompilationOptions::default(),
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                strip_index_format: None,
                front_face: wgpu::FrontFace::Ccw,
                cull_mode: Some(wgpu::Face::Back),
                polygon_mode: wgpu::PolygonMode::Fill,
                unclipped_depth: false,
                conservative: false,
            },
            depth_stencil: Some(wgpu::DepthStencilState {
                format: Texture::DEPTH_FORMAT,
                depth_write_enabled: true,
                depth_compare: wgpu::CompareFunction::Less,
                stencil: wgpu::StencilState::default(),
                bias: wgpu::DepthBiasState::default(),
            }),
            multisample: wgpu::MultisampleState::default(),
            multiview: None,
            cache: None,
        });
        let ui_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("HumanCraft UI Pipeline"),
            layout: Some(&ui_pipeline_layout),
            vertex: wgpu::VertexState {
                module: &ui_shader,
                entry_point: Some("vs_main"),
                buffers: &[Vertex::layout()],
                compilation_options: wgpu::PipelineCompilationOptions::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &ui_shader,
                entry_point: Some("fs_main"),
                targets: &[Some(wgpu::ColorTargetState {
                    format: config.format,
                    blend: Some(wgpu::BlendState::REPLACE),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: wgpu::PipelineCompilationOptions::default(),
            }),
            primitive: wgpu::PrimitiveState::default(),
            depth_stencil: Some(wgpu::DepthStencilState {
                format: Texture::DEPTH_FORMAT,
                depth_write_enabled: false,
                depth_compare: wgpu::CompareFunction::Always,
                stencil: wgpu::StencilState::default(),
                bias: wgpu::DepthBiasState::default(),
            }),
            multisample: wgpu::MultisampleState::default(),
            multiview: None,
            cache: None,
        });
        let textured_ui_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("HumanCraft Textured UI Pipeline"),
            layout: Some(&textured_ui_pipeline_layout),
            vertex: wgpu::VertexState {
                module: &textured_ui_shader,
                entry_point: Some("vs_main"),
                buffers: &[Vertex::layout()],
                compilation_options: wgpu::PipelineCompilationOptions::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &textured_ui_shader,
                entry_point: Some("fs_main"),
                targets: &[Some(wgpu::ColorTargetState {
                    format: config.format,
                    blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: wgpu::PipelineCompilationOptions::default(),
            }),
            primitive: wgpu::PrimitiveState::default(),
            depth_stencil: Some(wgpu::DepthStencilState {
                format: Texture::DEPTH_FORMAT,
                depth_write_enabled: false,
                depth_compare: wgpu::CompareFunction::Always,
                stencil: wgpu::StencilState::default(),
                bias: wgpu::DepthBiasState::default(),
            }),
            multisample: wgpu::MultisampleState::default(),
            multiview: None,
            cache: None,
        });
        let line_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("HumanCraft Line Pipeline"),
            layout: Some(&line_pipeline_layout),
            vertex: wgpu::VertexState {
                module: &line_shader,
                entry_point: Some("vs_main"),
                buffers: &[Vertex::layout()],
                compilation_options: wgpu::PipelineCompilationOptions::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &line_shader,
                entry_point: Some("fs_main"),
                targets: &[Some(wgpu::ColorTargetState {
                    format: config.format,
                    blend: Some(wgpu::BlendState::REPLACE),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: wgpu::PipelineCompilationOptions::default(),
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::LineList,
                strip_index_format: None,
                front_face: wgpu::FrontFace::Ccw,
                cull_mode: None,
                polygon_mode: wgpu::PolygonMode::Fill,
                unclipped_depth: false,
                conservative: false,
            },
            depth_stencil: Some(wgpu::DepthStencilState {
                format: Texture::DEPTH_FORMAT,
                depth_write_enabled: false,
                depth_compare: wgpu::CompareFunction::Always,
                stencil: wgpu::StencilState::default(),
                bias: wgpu::DepthBiasState::default(),
            }),
            multisample: wgpu::MultisampleState::default(),
            multiview: None,
            cache: None,
        });
        let chunk_buffers = HashMap::new();
        let (crosshair_vertices, crosshair_indices) =
            build_crosshair_mesh(config.width, config.height);
        let crosshair_vertex_buffer =
            device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("Crosshair Vertex Buffer"),
                contents: bytemuck::cast_slice(&crosshair_vertices),
                usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            });
        let crosshair_index_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Crosshair Index Buffer"),
            contents: bytemuck::cast_slice(&crosshair_indices),
            usage: wgpu::BufferUsages::INDEX,
        });
        let outline_vertex_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Target Outline Vertex Buffer"),
            size: (std::mem::size_of::<Vertex>() * 24) as u64,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        release_cursor(&window);

        let save_store = WorldSaveStore::default();
        let worlds = save_store.list_worlds().unwrap_or_else(|error| {
            eprintln!("{error}");
            Vec::new()
        });

        Self {
            window,
            surface,
            device,
            queue,
            config,
            size,
            render_pipeline,
            ui_pipeline,
            textured_ui_pipeline,
            line_pipeline,
            texture_atlas,
            texture_bind_group,
            camera_buffer,
            camera_bind_group,
            chunk_buffers,
            pending_chunk_remeshes: HashSet::new(),
            crosshair_vertex_buffer,
            crosshair_index_buffer,
            crosshair_index_count: crosshair_indices.len() as u32,
            outline_vertex_buffer,
            outline_vertex_count: 0,
            depth_texture,
            camera,
            world: None,
            save_store,
            worlds,
            active_world: None,
            mode: AppMode::MainMenu,
            selected_world: 0,
            text_entry: TextEntry::default(),
            new_world_config: NewWorldConfig::default(),
            dirty_save_chunks: HashSet::new(),
            player_state_dirty: false,
            cursor_position: PhysicalPosition::new(0.0, 0.0),
            targeted_block: None,
            input: InputState::default(),
            paused: true,
            inventory_open: false,
            inventory_cursor: None,
            inventory_drag: None,
            held_block_interaction: HeldBlockInteraction::default(),
            selected_hotbar_slot: 0,
            last_frame: Instant::now(),
        }
        .with_updated_title()
    }

    fn resize(&mut self, size: PhysicalSize<u32>) {
        if size.width == 0 || size.height == 0 {
            return;
        }

        self.size = size;
        self.config.width = size.width;
        self.config.height = size.height;
        self.surface.configure(&self.device, &self.config);
        self.depth_texture = Texture::create_depth_texture(&self.device, &self.config);
        self.update_crosshair_mesh();
    }

    fn handle_key(&mut self, event: &KeyEvent) -> bool {
        if event.state == ElementState::Pressed {
            if self.mode == AppMode::InGame && is_inventory_key(event) {
                self.set_inventory_open(!self.inventory_open);
                return true;
            }

            if self.mode == AppMode::InGame && !self.paused && self.handle_hotbar_key(event) {
                return true;
            }

            if self.handle_menu_key(event) {
                return true;
            }
        }

        if event.state == ElementState::Pressed
            && matches!(event.logical_key.as_ref(), Key::Named(NamedKey::Escape))
            && self.mode == AppMode::InGame
        {
            if self.inventory_open {
                self.set_inventory_open(false);
                return true;
            }
            self.set_paused(!self.paused);
            return true;
        }

        if self.mode == AppMode::InGame && !self.paused && !self.inventory_open {
            self.input.handle_key(event);
        }
        true
    }

    fn handle_menu_key(&mut self, event: &KeyEvent) -> bool {
        match self.mode {
            AppMode::MainMenu => {
                if is_confirm_key(event) {
                    self.mode = AppMode::ManageWorlds;
                    self.refresh_worlds();
                    self.update_window_title();
                    return true;
                }
            }
            AppMode::ManageWorlds => {
                if is_confirm_key(event) {
                    self.load_selected_world();
                    return true;
                }
                if matches!(event.logical_key.as_ref(), Key::Named(NamedKey::ArrowUp)) {
                    self.select_previous_world();
                    return true;
                }
                if matches!(event.logical_key.as_ref(), Key::Named(NamedKey::ArrowDown)) {
                    self.select_next_world();
                    return true;
                }
                if matches!(event.logical_key.as_ref(), Key::Named(NamedKey::Delete)) {
                    self.delete_selected_world();
                    return true;
                }
                if character_key(event, "n") {
                    self.start_world_creation();
                    return true;
                }
                if character_key(event, "r") {
                    self.start_world_rename();
                    return true;
                }
                if matches!(event.logical_key.as_ref(), Key::Named(NamedKey::Escape)) {
                    self.mode = AppMode::MainMenu;
                    self.update_window_title();
                    return true;
                }
            }
            AppMode::ConfigNewWorld => {
                if is_confirm_key(event) {
                    self.create_configured_world();
                    return true;
                }
                if matches!(event.logical_key.as_ref(), Key::Named(NamedKey::Tab)) {
                    self.new_world_config.toggle_focus();
                    self.update_window_title();
                    return true;
                }
                if matches!(event.logical_key.as_ref(), Key::Named(NamedKey::Backspace)) {
                    self.new_world_config.pop();
                    self.update_window_title();
                    return true;
                }
                if matches!(event.logical_key.as_ref(), Key::Named(NamedKey::Escape)) {
                    self.mode = AppMode::ManageWorlds;
                    self.update_window_title();
                    return true;
                }
                if let Key::Character(character) = event.logical_key.as_ref() {
                    self.new_world_config.push(character);
                    self.update_window_title();
                    return true;
                }
            }
            AppMode::RenamingWorld => {
                if is_confirm_key(event) {
                    self.finish_text_entry();
                    return true;
                }
                if matches!(event.logical_key.as_ref(), Key::Named(NamedKey::Backspace)) {
                    self.text_entry.pop();
                    self.update_window_title();
                    return true;
                }
                if matches!(event.logical_key.as_ref(), Key::Named(NamedKey::Escape)) {
                    self.mode = AppMode::ManageWorlds;
                    self.text_entry = TextEntry::default();
                    self.update_window_title();
                    return true;
                }
                if let Key::Character(character) = event.logical_key.as_ref() {
                    self.text_entry.push(character);
                    self.update_window_title();
                    return true;
                }
            }
            AppMode::InGame => {
                if self.paused {
                    if is_confirm_key(event) {
                        self.resume_game();
                        return true;
                    }
                    if matches!(event.logical_key.as_ref(), Key::Named(NamedKey::Escape)) {
                        self.resume_game();
                        return true;
                    }
                    if character_key(event, "q") {
                        self.save_and_quit_to_main_menu();
                        return true;
                    }
                }
            }
        }

        false
    }

    fn handle_text_input(&mut self, text: &str) {
        if text.is_ascii() {
            return;
        }
        match self.mode {
            AppMode::ConfigNewWorld => {
                self.new_world_config.push(text);
                self.update_window_title();
            }
            AppMode::RenamingWorld => {
                self.text_entry.push(text);
                self.update_window_title();
            }
            _ => {}
        }
    }

    fn handle_cursor_moved(&mut self, position: PhysicalPosition<f64>) {
        self.cursor_position = position;
        if self.mode == AppMode::InGame && self.inventory_open {
            self.update_inventory_drag();
        }
    }

    fn handle_focus_lost(&mut self) {
        if self.mode == AppMode::InGame {
            self.set_paused(true);
        }
    }

    fn handle_hotbar_key(&mut self, event: &KeyEvent) -> bool {
        if matches!(event.logical_key.as_ref(), Key::Named(NamedKey::ArrowLeft)) {
            self.selected_hotbar_slot =
                (self.selected_hotbar_slot + INVENTORY_HOTBAR_SLOTS - 1) % INVENTORY_HOTBAR_SLOTS;
            return true;
        }
        if matches!(event.logical_key.as_ref(), Key::Named(NamedKey::ArrowRight)) {
            self.selected_hotbar_slot = (self.selected_hotbar_slot + 1) % INVENTORY_HOTBAR_SLOTS;
            return true;
        }
        false
    }

    fn handle_mouse_motion(&mut self, delta_x: f32, delta_y: f32) {
        if self.mode == AppMode::InGame && !self.paused && !self.inventory_open {
            self.camera.apply_mouse_delta(delta_x, delta_y);
        }
    }

    fn handle_mouse_button(&mut self, button: MouseButton, mouse_state: ElementState) {
        if self.mode == AppMode::InGame
            && self.inventory_open
            && (button == MouseButton::Left || button == MouseButton::Right)
        {
            match mouse_state {
                ElementState::Pressed => self.start_inventory_mouse(button),
                ElementState::Released => self.finish_inventory_mouse(button),
            }
            return;
        }

        if mouse_state == ElementState::Released {
            self.held_block_interaction.release(button);
            return;
        }

        if self.mode != AppMode::InGame || self.paused {
            if button == MouseButton::Left {
                self.handle_menu_click();
            }
            return;
        }

        if self.paused {
            return;
        }

        if !matches!(button, MouseButton::Left | MouseButton::Right) {
            return;
        }
        self.held_block_interaction.press(button);
        let dirty_chunks = self.apply_block_interaction(button);

        if !dirty_chunks.is_empty() {
            self.mark_dirty_chunks_for_save(&dirty_chunks);
            self.rebuild_chunk_meshes(&dirty_chunks);
        }
    }

    fn set_paused(&mut self, paused: bool) {
        if self.mode != AppMode::InGame {
            return;
        }
        if paused {
            self.stow_inventory_cursor();
            self.inventory_drag = None;
            self.held_block_interaction.clear();
        }
        self.paused = paused;
        if paused {
            self.inventory_open = false;
        }
        self.input.clear_movement();
        if paused {
            self.mark_player_state_dirty();
            release_cursor(&self.window);
            self.window
                .set_title("HumanCraft - Paused (Esc to resume, close window to quit)");
        } else {
            capture_cursor(&self.window);
            self.update_window_title();
        }
    }

    fn set_inventory_open(&mut self, inventory_open: bool) {
        if self.mode != AppMode::InGame || self.paused {
            return;
        }
        self.held_block_interaction.clear();
        if !inventory_open {
            self.stow_inventory_cursor();
            self.inventory_drag = None;
        }
        self.inventory_open = inventory_open;
        self.input.clear_movement();
        if inventory_open {
            release_cursor(&self.window);
            self.window
                .set_title("HumanCraft - Inventory (E or Esc to close)");
        } else {
            capture_cursor(&self.window);
            self.update_window_title();
        }
    }

    fn start_inventory_mouse(&mut self, button: MouseButton) {
        let Some(button) = inventory_mouse_button(button) else {
            return;
        };
        let slot = self.inventory_slot_at_cursor();
        self.inventory_drag = Some(InventoryDrag::new(button, slot));
    }

    fn finish_inventory_mouse(&mut self, button: MouseButton) {
        let Some(button) = inventory_mouse_button(button) else {
            return;
        };
        let Some(drag) = self.inventory_drag.take() else {
            return;
        };
        if drag.button != button {
            return;
        }

        let slot = self.inventory_slot_at_cursor().or(drag.start_slot);
        match drag.button {
            InventoryMouseButton::Left if drag.changed_slots && !drag.slots.is_empty() => {
                if let Some(world) = self.world.as_mut() {
                    distribute_carried_stack_evenly(
                        &mut world.player_inventory,
                        &mut self.inventory_cursor,
                        &drag.slots,
                        &world.items,
                    );
                }
            }
            InventoryMouseButton::Left => {
                if let (Some(world), Some(slot)) = (self.world.as_mut(), slot) {
                    left_click_inventory_slot(
                        &mut world.player_inventory,
                        &mut self.inventory_cursor,
                        slot,
                        &world.items,
                    );
                }
            }
            InventoryMouseButton::Right if drag.applied_drag => {}
            InventoryMouseButton::Right => {
                if let (Some(world), Some(slot)) = (self.world.as_mut(), slot) {
                    right_click_inventory_slot(
                        &mut world.player_inventory,
                        &mut self.inventory_cursor,
                        slot,
                        &world.items,
                    );
                }
            }
        }
    }

    fn update_inventory_drag(&mut self) {
        let Some(slot) = self.inventory_slot_at_cursor() else {
            return;
        };
        let Some(drag) = self.inventory_drag.as_mut() else {
            return;
        };
        if !drag.push_slot(slot) {
            return;
        }
        if drag.button == InventoryMouseButton::Right {
            if let Some(world) = self.world.as_mut() {
                if place_one_carried_item(
                    &mut world.player_inventory,
                    &mut self.inventory_cursor,
                    slot,
                    &world.items,
                ) {
                    drag.applied_drag = true;
                }
            }
        }
    }

    fn inventory_slot_at_cursor(&self) -> Option<usize> {
        if self.mode != AppMode::InGame || !self.inventory_open {
            return None;
        }
        let point = cursor_to_ui_point(self.cursor_position, self.size);
        let aspect = self.config.width.max(1) as f32 / self.config.height.max(1) as f32;
        inventory_slot_at_point(point, aspect)
    }

    fn stow_inventory_cursor(&mut self) {
        let Some(stack) = self.inventory_cursor.take() else {
            return;
        };
        let Some(world) = self.world.as_mut() else {
            self.inventory_cursor = Some(stack);
            return;
        };
        self.inventory_cursor = world.player_inventory.add_stack(stack, &world.items);
    }

    fn update(&mut self) {
        let now = Instant::now();
        let delta_seconds = now.duration_since(self.last_frame).as_secs_f32().min(0.1);
        self.last_frame = now;
        let mut dirty_chunks = Vec::new();
        if self.mode == AppMode::InGame && !self.paused && !self.inventory_open {
            if let Some(world) = self.world.as_mut() {
                self.camera.update(&self.input, world, delta_seconds);
                dirty_chunks.extend(world.ensure_chunks_around_render_position_with_store(
                    self.camera.position,
                    MAX_CHUNK_LOADS_PER_FRAME,
                    &self.save_store,
                ));
            }
            if let Some(button) = self.held_block_interaction.repeat_button(delta_seconds) {
                dirty_chunks.extend(self.apply_block_interaction(button));
            }
        }
        if self.mode == AppMode::InGame && !self.paused {
            if let Some(world) = self.world.as_mut() {
                world.update_loot(self.camera.position, delta_seconds);
            }
        }
        if !dirty_chunks.is_empty() {
            self.mark_dirty_chunks_for_save(&dirty_chunks);
            self.queue_chunk_remeshes(&dirty_chunks);
        }
        let remesh_chunks = self.take_pending_chunk_remeshes(MAX_CHUNK_REMESHES_PER_FRAME);
        if !remesh_chunks.is_empty() {
            self.rebuild_chunk_meshes(&remesh_chunks);
        }
        self.update_target_outline();
        let uniform = CameraUniform::new(
            self.camera
                .view_projection(self.config.width, self.config.height),
        );
        self.queue
            .write_buffer(&self.camera_buffer, 0, bytemuck::bytes_of(&uniform));
    }

    fn apply_block_interaction(&mut self, button: MouseButton) -> Vec<ChunkPosition> {
        let Some(world) = self.world.as_mut() else {
            return Vec::new();
        };
        let Some(hit) = world.raycast(self.camera.position, self.camera.forward()) else {
            return Vec::new();
        };

        match button {
            MouseButton::Left => world.break_block(hit.block),
            MouseButton::Right => world.place_selected_hotbar_block_for_player(
                hit.previous,
                self.selected_hotbar_slot,
                self.camera.position,
            ),
            _ => Vec::new(),
        }
    }

    fn render(&mut self) -> Result<(), wgpu::SurfaceError> {
        let aspect = self.config.width.max(1) as f32 / self.config.height.max(1) as f32;
        let ui_mesh = if self.mode == AppMode::InGame && !self.paused {
            self.world.as_ref().map(|world| {
                build_gameplay_ui_mesh(
                    world,
                    self.inventory_open,
                    aspect,
                    self.selected_hotbar_slot,
                    self.inventory_cursor,
                    cursor_to_ui_point(self.cursor_position, self.size),
                )
            })
        } else if self.mode != AppMode::InGame || self.paused {
            Some(build_menu_mesh(self))
        } else {
            None
        };
        let textured_ui_mesh = if self.mode == AppMode::InGame && !self.paused {
            self.world.as_ref().map(|world| {
                build_inventory_icon_mesh(
                    world,
                    &self.texture_atlas,
                    self.inventory_open,
                    aspect,
                    self.selected_hotbar_slot,
                    self.inventory_cursor,
                    cursor_to_ui_point(self.cursor_position, self.size),
                )
            })
        } else {
            None
        };
        let loot_mesh = if self.mode == AppMode::InGame {
            self.world
                .as_ref()
                .map(|world| build_loot_mesh(world, &self.texture_atlas, &self.camera))
        } else {
            None
        };
        let ui_buffers = ui_mesh.as_ref().and_then(|(vertices, indices)| {
            if vertices.is_empty() || indices.is_empty() {
                return None;
            }
            let vertex_buffer = self
                .device
                .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: Some("Dynamic Menu Vertex Buffer"),
                    contents: bytemuck::cast_slice(vertices),
                    usage: wgpu::BufferUsages::VERTEX,
                });
            let index_buffer = self
                .device
                .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: Some("Dynamic Menu Index Buffer"),
                    contents: bytemuck::cast_slice(indices),
                    usage: wgpu::BufferUsages::INDEX,
                });
            Some((vertex_buffer, index_buffer, indices.len() as u32))
        });
        let textured_ui_buffers = textured_ui_mesh.as_ref().and_then(|(vertices, indices)| {
            if vertices.is_empty() || indices.is_empty() {
                return None;
            }
            let vertex_buffer = self
                .device
                .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: Some("Dynamic Textured UI Vertex Buffer"),
                    contents: bytemuck::cast_slice(vertices),
                    usage: wgpu::BufferUsages::VERTEX,
                });
            let index_buffer = self
                .device
                .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: Some("Dynamic Textured UI Index Buffer"),
                    contents: bytemuck::cast_slice(indices),
                    usage: wgpu::BufferUsages::INDEX,
                });
            Some((vertex_buffer, index_buffer, indices.len() as u32))
        });
        let loot_buffers = loot_mesh.as_ref().and_then(|(vertices, indices)| {
            if vertices.is_empty() || indices.is_empty() {
                return None;
            }
            let vertex_buffer = self
                .device
                .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: Some("Dynamic Loot Vertex Buffer"),
                    contents: bytemuck::cast_slice(vertices),
                    usage: wgpu::BufferUsages::VERTEX,
                });
            let index_buffer = self
                .device
                .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: Some("Dynamic Loot Index Buffer"),
                    contents: bytemuck::cast_slice(indices),
                    usage: wgpu::BufferUsages::INDEX,
                });
            Some((vertex_buffer, index_buffer, indices.len() as u32))
        });

        let frame = self.surface.get_current_texture()?;
        let view = frame
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());
        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("Render Encoder"),
            });

        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Main Render Pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color {
                            r: 0.53,
                            g: 0.75,
                            b: 0.95,
                            a: 1.0,
                        }),
                        store: wgpu::StoreOp::Store,
                    },
                    depth_slice: None,
                })],
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: &self.depth_texture.view,
                    depth_ops: Some(wgpu::Operations {
                        load: wgpu::LoadOp::Clear(1.0),
                        store: wgpu::StoreOp::Store,
                    }),
                    stencil_ops: None,
                }),
                timestamp_writes: None,
                occlusion_query_set: None,
            });
            if self.mode == AppMode::InGame {
                pass.set_pipeline(&self.render_pipeline);
                pass.set_bind_group(0, &self.camera_bind_group, &[]);
                pass.set_bind_group(1, &self.texture_bind_group, &[]);
                for chunk_buffer in self.chunk_buffers.values() {
                    if chunk_buffer.index_count == 0 {
                        continue;
                    }
                    pass.set_vertex_buffer(0, chunk_buffer.vertex_buffer.slice(..));
                    pass.set_index_buffer(
                        chunk_buffer.index_buffer.slice(..),
                        wgpu::IndexFormat::Uint32,
                    );
                    pass.draw_indexed(0..chunk_buffer.index_count, 0, 0..1);
                }

                if let Some((vertex_buffer, index_buffer, index_count)) = &loot_buffers {
                    pass.set_vertex_buffer(0, vertex_buffer.slice(..));
                    pass.set_index_buffer(index_buffer.slice(..), wgpu::IndexFormat::Uint32);
                    pass.draw_indexed(0..*index_count, 0, 0..1);
                }

                if self.outline_vertex_count > 0 {
                    pass.set_pipeline(&self.line_pipeline);
                    pass.set_bind_group(0, &self.camera_bind_group, &[]);
                    pass.set_vertex_buffer(0, self.outline_vertex_buffer.slice(..));
                    pass.draw(0..self.outline_vertex_count, 0..1);
                }
            }

            pass.set_pipeline(&self.ui_pipeline);
            if self.mode == AppMode::InGame && !self.paused {
                pass.set_vertex_buffer(0, self.crosshair_vertex_buffer.slice(..));
                pass.set_index_buffer(
                    self.crosshair_index_buffer.slice(..),
                    wgpu::IndexFormat::Uint32,
                );
                pass.draw_indexed(0..self.crosshair_index_count, 0, 0..1);
            }

            if let Some((vertex_buffer, index_buffer, index_count)) = &ui_buffers {
                pass.set_vertex_buffer(0, vertex_buffer.slice(..));
                pass.set_index_buffer(index_buffer.slice(..), wgpu::IndexFormat::Uint32);
                pass.draw_indexed(0..*index_count, 0, 0..1);
            }

            if let Some((vertex_buffer, index_buffer, index_count)) = &textured_ui_buffers {
                pass.set_pipeline(&self.textured_ui_pipeline);
                pass.set_bind_group(0, &self.texture_bind_group, &[]);
                pass.set_vertex_buffer(0, vertex_buffer.slice(..));
                pass.set_index_buffer(index_buffer.slice(..), wgpu::IndexFormat::Uint32);
                pass.draw_indexed(0..*index_count, 0, 0..1);
            }
        }

        self.queue.submit(Some(encoder.finish()));
        frame.present();
        Ok(())
    }

    fn rebuild_chunk_meshes(&mut self, dirty_chunks: &[ChunkPosition]) {
        let Some(world) = &self.world else {
            return;
        };
        for chunk_position in unique_loaded_chunk_positions(dirty_chunks, world) {
            let Some((vertices, indices)) =
                world.build_chunk_render_mesh(chunk_position, &self.texture_atlas)
            else {
                self.chunk_buffers.remove(&chunk_position);
                continue;
            };
            self.chunk_buffers.insert(
                chunk_position,
                ChunkRenderBuffer::new(&self.device, chunk_position, &vertices, &indices),
            );
        }
    }

    fn queue_chunk_remeshes(&mut self, dirty_chunks: &[ChunkPosition]) {
        let Some(world) = &self.world else {
            return;
        };
        for chunk_position in unique_loaded_chunk_positions(dirty_chunks, world) {
            self.pending_chunk_remeshes.insert(chunk_position);
        }
    }

    fn take_pending_chunk_remeshes(&mut self, limit: usize) -> Vec<ChunkPosition> {
        let mut chunks: Vec<_> = self.pending_chunk_remeshes.iter().copied().collect();
        chunks.sort_by_key(|chunk| {
            let camera_chunk = chunk_position_for_render_position(self.camera.position);
            let dx = (chunk.x - camera_chunk.x).abs();
            let dz = (chunk.z - camera_chunk.z).abs();
            (dx.max(dz), dx + dz, chunk.z, chunk.x)
        });
        chunks.truncate(limit);

        for chunk in &chunks {
            self.pending_chunk_remeshes.remove(chunk);
        }

        chunks
    }

    fn update_target_outline(&mut self) {
        self.targeted_block = if self.mode != AppMode::InGame || self.paused {
            None
        } else {
            self.world
                .as_ref()
                .and_then(|world| world.raycast(self.camera.position, self.camera.forward()))
                .map(|hit| hit.block)
        };

        if let Some(block) = self.targeted_block {
            let vertices = build_outline_vertices(block);
            self.queue.write_buffer(
                &self.outline_vertex_buffer,
                0,
                bytemuck::cast_slice(&vertices),
            );
            self.outline_vertex_count = vertices.len() as u32;
        } else {
            self.outline_vertex_count = 0;
        }
    }

    fn update_crosshair_mesh(&self) {
        let (vertices, _) = build_crosshair_mesh(self.config.width, self.config.height);
        self.queue.write_buffer(
            &self.crosshair_vertex_buffer,
            0,
            bytemuck::cast_slice(&vertices),
        );
    }

    fn handle_menu_click(&mut self) {
        let point = cursor_to_ui_point(self.cursor_position, self.size);

        match self.mode {
            AppMode::MainMenu => {
                if UI_MAIN_PLAY.contains(point) {
                    self.mode = AppMode::ManageWorlds;
                    self.refresh_worlds();
                    self.update_window_title();
                }
            }
            AppMode::ManageWorlds => {
                if UI_WORLDS_PLAY.contains(point) {
                    self.load_selected_world();
                } else if UI_WORLDS_NEW.contains(point) {
                    self.start_world_creation();
                } else if UI_WORLDS_RENAME.contains(point) {
                    self.start_world_rename();
                } else if UI_WORLDS_DELETE.contains(point) {
                    self.delete_selected_world();
                } else if UI_WORLDS_BACK.contains(point) {
                    self.mode = AppMode::MainMenu;
                    self.update_window_title();
                } else if let Some(index) = world_list_hit_index(point, self.worlds.len()) {
                    self.selected_world = index;
                    self.update_window_title();
                }
            }
            AppMode::ConfigNewWorld => {
                if UI_CONFIG_NAME_FIELD.contains(point) {
                    self.new_world_config.focused = ConfigField::Name;
                    self.update_window_title();
                } else if UI_CONFIG_SEED_FIELD.contains(point) {
                    self.new_world_config.focused = ConfigField::Seed;
                    self.update_window_title();
                } else if UI_CONFIG_CREATE.contains(point) {
                    self.create_configured_world();
                } else if UI_CONFIG_BACK.contains(point) {
                    self.mode = AppMode::ManageWorlds;
                    self.update_window_title();
                }
            }
            AppMode::RenamingWorld => {
                if UI_RENAME_SAVE.contains(point) {
                    self.finish_text_entry();
                } else if UI_RENAME_BACK.contains(point) {
                    self.mode = AppMode::ManageWorlds;
                    self.text_entry = TextEntry::default();
                    self.update_window_title();
                }
            }
            AppMode::InGame if self.paused => {
                if UI_PAUSE_KEEP_PLAYING.contains(point) {
                    self.resume_game();
                } else if UI_PAUSE_SAVE_QUIT.contains(point) {
                    self.save_and_quit_to_main_menu();
                }
            }
            _ => {}
        }
    }

    fn refresh_worlds(&mut self) {
        self.worlds = self.save_store.list_worlds().unwrap_or_else(|error| {
            eprintln!("{error}");
            Vec::new()
        });
        if self.worlds.is_empty() {
            self.selected_world = 0;
        } else {
            self.selected_world = self.selected_world.min(self.worlds.len() - 1);
        }
    }

    fn select_previous_world(&mut self) {
        if self.worlds.is_empty() {
            return;
        }
        self.selected_world = self.selected_world.saturating_sub(1);
        self.update_window_title();
    }

    fn select_next_world(&mut self) {
        if self.worlds.is_empty() {
            return;
        }
        self.selected_world = (self.selected_world + 1).min(self.worlds.len() - 1);
        self.update_window_title();
    }

    fn start_world_creation(&mut self) {
        self.new_world_config
            .start(default_world_name(self.worlds.len()));
        self.mode = AppMode::ConfigNewWorld;
        self.update_window_title();
    }

    fn start_world_rename(&mut self) {
        let Some(world) = self.worlds.get(self.selected_world) else {
            return;
        };
        self.text_entry.start(world.name.clone());
        self.mode = AppMode::RenamingWorld;
        self.update_window_title();
    }

    fn finish_text_entry(&mut self) {
        match self.mode {
            AppMode::RenamingWorld => {
                let Some(world) = self.worlds.get(self.selected_world) else {
                    self.mode = AppMode::ManageWorlds;
                    self.update_window_title();
                    return;
                };
                let name = self.text_entry.finish();
                match self.save_store.rename_world(&world.id, &name) {
                    Ok(_) => {
                        self.mode = AppMode::ManageWorlds;
                        self.refresh_worlds();
                        self.update_window_title();
                    }
                    Err(error) => self.report_save_error(error),
                }
            }
            _ => {}
        }
    }

    fn create_configured_world(&mut self) {
        let name = self.new_world_config.final_name();
        let seed = self
            .new_world_config
            .seed
            .trim()
            .parse::<u64>()
            .unwrap_or_else(|_| new_world_seed(self.worlds.len()));
        let placeholder_player = PlayerSave::new(
            0.0,
            0.0,
            20.0,
            -90.0_f32.to_radians(),
            -18.0_f32.to_radians(),
        );
        match self
            .save_store
            .create_world(&name, seed, placeholder_player)
        {
            Ok(metadata) => {
                self.refresh_worlds();
                if let Some(index) = self.worlds.iter().position(|world| world.id == metadata.id) {
                    self.selected_world = index;
                }
                self.load_world(metadata);
            }
            Err(error) => self.report_save_error(error),
        }
    }

    fn delete_selected_world(&mut self) {
        let Some(world) = self.worlds.get(self.selected_world) else {
            return;
        };
        let id = world.id.clone();
        if let Err(error) = self.save_store.delete_world(&id) {
            self.report_save_error(error);
            return;
        }
        self.refresh_worlds();
        self.update_window_title();
    }

    fn load_selected_world(&mut self) {
        let Some(metadata) = self.worlds.get(self.selected_world).cloned() else {
            self.start_world_creation();
            return;
        };
        self.load_world(metadata);
    }

    fn load_world(&mut self, mut metadata: WorldMetadata) {
        let content = bootstrap_content().expect("content should bootstrap");
        let pipeline = default_generation_pipeline(content.block_ids);
        let generation_context = GenerationContext {
            seed: metadata.seed,
            air: content.block_ids.air,
        };
        let mut world = ClientWorld::new(
            content.blocks,
            content.items,
            content.block_ids,
            pipeline,
            generation_context,
            CLIENT_RENDER_DISTANCE_CHUNKS,
            metadata.id.clone(),
        );
        world.player_inventory = inventory_from_save(&metadata.inventory, &world.items);

        let saved_eye = Vec3::new(
            metadata.player.eye_x,
            metadata.player.eye_y,
            metadata.player.eye_z,
        );
        let generated_chunks = world.ensure_chunks_around_render_position_with_store(
            saved_eye,
            usize::MAX,
            &self.save_store,
        );
        let spawn_eye = if metadata.player.eye_y == 0.0 {
            world.safe_spawn_eye_position(Vec3::new(0.0, 0.0, 20.0))
        } else {
            saved_eye
        };
        self.camera = Camera::from_save(PlayerSave::new(
            spawn_eye.x,
            spawn_eye.y,
            spawn_eye.z,
            metadata.player.yaw,
            metadata.player.pitch,
        ));
        metadata.player = self.camera.to_save();

        self.world = Some(world);
        self.active_world = Some(metadata);
        self.chunk_buffers.clear();
        self.pending_chunk_remeshes.clear();
        self.dirty_save_chunks.clear();
        self.player_state_dirty = false;
        self.inventory_cursor = None;
        self.inventory_drag = None;
        self.selected_hotbar_slot = 0;
        self.chunk_buffers = if let Some(world) = &self.world {
            build_chunk_render_buffers(&self.device, world, &self.texture_atlas, &generated_chunks)
        } else {
            HashMap::new()
        };
        self.mode = AppMode::InGame;
        self.paused = false;
        self.inventory_open = false;
        self.input.clear_movement();
        capture_cursor(&self.window);
        self.update_window_title();
    }

    fn mark_dirty_chunks_for_save(&mut self, dirty_chunks: &[ChunkPosition]) {
        let Some(world) = &self.world else {
            return;
        };
        for chunk_position in unique_loaded_chunk_positions(dirty_chunks, world) {
            self.dirty_save_chunks.insert(chunk_position);
        }
    }

    fn mark_player_state_dirty(&mut self) {
        if self.active_world.is_some() {
            self.player_state_dirty = true;
        }
    }

    fn flush_active_world_to_disk(&mut self) {
        self.stow_inventory_cursor();
        let Some(metadata) = self.active_world.as_mut() else {
            return;
        };
        metadata.player = self.camera.to_save();
        if let Some(world) = &self.world {
            metadata.inventory = inventory_to_save(&world.player_inventory, &world.items);
        }
        metadata.updated_at_unix_seconds = current_save_time();
        let world_id = metadata.id.clone();
        if let Err(error) = self.save_store.save_metadata(metadata) {
            self.report_save_error(error);
        }

        if let Some(world) = &self.world {
            let dirty_chunks: Vec<_> = self.dirty_save_chunks.iter().copied().collect();
            for chunk_position in dirty_chunks {
                if let Some(chunk) = world.chunks.get(&chunk_position) {
                    if let Err(error) = self.save_store.save_chunk(&world_id, chunk) {
                        self.report_save_error(error);
                    }
                }
            }
        }

        self.dirty_save_chunks.clear();
        self.player_state_dirty = false;
    }

    fn resume_game(&mut self) {
        self.set_paused(false);
    }

    fn save_and_quit_to_main_menu(&mut self) {
        self.flush_active_world_to_disk();
        self.world = None;
        self.active_world = None;
        self.chunk_buffers.clear();
        self.pending_chunk_remeshes.clear();
        self.dirty_save_chunks.clear();
        self.player_state_dirty = false;
        self.inventory_cursor = None;
        self.inventory_drag = None;
        self.input.clear_movement();
        self.paused = true;
        self.inventory_open = false;
        self.mode = AppMode::MainMenu;
        self.refresh_worlds();
        release_cursor(&self.window);
        self.update_window_title();
    }

    fn report_save_error(&self, error: WorldSaveError) {
        eprintln!("{error}");
        self.window
            .set_title(&format!("HumanCraft - Save error: {error}"));
    }

    fn with_updated_title(self) -> Self {
        self.update_window_title();
        self
    }

    fn update_window_title(&self) {
        let title = match self.mode {
            AppMode::MainMenu => "HumanCraft - Main Menu: click Play or press Enter".to_string(),
            AppMode::ManageWorlds => {
                if let Some(world) = self.worlds.get(self.selected_world) {
                    format!(
                        "HumanCraft - Worlds: {} seed {} ({}/{}) | Enter load, N new, R rename, Delete delete",
                        world.name,
                        world.seed,
                        self.selected_world + 1,
                        self.worlds.len()
                    )
                } else {
                    "HumanCraft - Worlds: no saves | N create or Enter".to_string()
                }
            }
            AppMode::ConfigNewWorld => format!(
                "HumanCraft - Configure New World: name '{}', seed {} | Tab field, Enter create",
                self.new_world_config.final_name(),
                if self.new_world_config.seed.is_empty() {
                    "auto"
                } else {
                    self.new_world_config.seed.as_str()
                }
            ),
            AppMode::RenamingWorld => format!(
                "HumanCraft - Rename world: {} | type, Enter save, Esc cancel",
                self.text_entry.display()
            ),
            AppMode::InGame => self
                .active_world
                .as_ref()
                .map(|world| format!("HumanCraft - {} (seed {})", world.name, world.seed))
                .unwrap_or_else(|| "HumanCraft".to_string()),
        };
        self.window.set_title(&title);
    }
}

struct ChunkRenderBuffer {
    vertex_buffer: wgpu::Buffer,
    index_buffer: wgpu::Buffer,
    index_count: u32,
}

impl ChunkRenderBuffer {
    fn new(
        device: &wgpu::Device,
        chunk_position: ChunkPosition,
        vertices: &[Vertex],
        indices: &[u32],
    ) -> Self {
        let vertex_label = format!("Chunk Vertex Buffer {chunk_position:?}");
        let index_label = format!("Chunk Index Buffer {chunk_position:?}");
        let vertex_buffer = if vertices.is_empty() {
            device.create_buffer(&wgpu::BufferDescriptor {
                label: Some(&vertex_label),
                size: 4,
                usage: wgpu::BufferUsages::VERTEX,
                mapped_at_creation: false,
            })
        } else {
            device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some(&vertex_label),
                contents: bytemuck::cast_slice(vertices),
                usage: wgpu::BufferUsages::VERTEX,
            })
        };
        let index_buffer = if indices.is_empty() {
            device.create_buffer(&wgpu::BufferDescriptor {
                label: Some(&index_label),
                size: 4,
                usage: wgpu::BufferUsages::INDEX,
                mapped_at_creation: false,
            })
        } else {
            device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some(&index_label),
                contents: bytemuck::cast_slice(indices),
                usage: wgpu::BufferUsages::INDEX,
            })
        };

        Self {
            vertex_buffer,
            index_buffer,
            index_count: indices.len() as u32,
        }
    }
}

fn build_chunk_render_buffers(
    device: &wgpu::Device,
    world: &ClientWorld,
    texture_atlas: &TextureAtlas,
    chunk_positions: &[ChunkPosition],
) -> HashMap<ChunkPosition, ChunkRenderBuffer> {
    let mut buffers = HashMap::new();
    for chunk_position in unique_loaded_chunk_positions(chunk_positions, world) {
        if let Some((vertices, indices)) =
            world.build_chunk_render_mesh(chunk_position, texture_atlas)
        {
            buffers.insert(
                chunk_position,
                ChunkRenderBuffer::new(device, chunk_position, &vertices, &indices),
            );
        }
    }
    buffers
}

fn unique_loaded_chunk_positions(
    chunk_positions: &[ChunkPosition],
    world: &ClientWorld,
) -> Vec<ChunkPosition> {
    let mut seen = HashSet::new();
    let mut unique = Vec::new();
    for chunk_position in chunk_positions {
        if world.chunks.contains_key(chunk_position) && seen.insert(*chunk_position) {
            unique.push(*chunk_position);
        }
    }
    unique
}

#[derive(Debug, Copy, Clone)]
struct Camera {
    position: Vec3,
    yaw: f32,
    pitch: f32,
    horizontal_velocity: Vec3,
    vertical_velocity: f32,
    grounded: bool,
    sneaking: bool,
    sprinting: bool,
    physics_accumulator: f32,
}

impl Camera {
    fn new(position: Vec3) -> Self {
        Self {
            position,
            yaw: -90.0_f32.to_radians(),
            pitch: -18.0_f32.to_radians(),
            horizontal_velocity: Vec3::ZERO,
            vertical_velocity: 0.0,
            grounded: false,
            sneaking: false,
            sprinting: false,
            physics_accumulator: 0.0,
        }
    }

    fn from_save(save: PlayerSave) -> Self {
        let mut camera = Self::new(Vec3::new(save.eye_x, save.eye_y, save.eye_z));
        camera.yaw = save.yaw;
        camera.pitch = save.pitch;
        camera
    }

    fn to_save(self) -> PlayerSave {
        PlayerSave::new(
            self.position.x,
            self.position.y,
            self.position.z,
            self.yaw,
            self.pitch,
        )
    }

    fn update(&mut self, input: &InputState, world: &ClientWorld, delta_seconds: f32) {
        self.physics_accumulator += delta_seconds;
        while self.physics_accumulator >= PHYSICS_TICK_SECONDS {
            self.tick(input, world);
            self.physics_accumulator -= PHYSICS_TICK_SECONDS;
        }

        if self.position.y < -80.0 {
            self.position = world.safe_spawn_eye_position(Vec3::new(0.0, 0.0, 20.0));
            self.horizontal_velocity = Vec3::ZERO;
            self.vertical_velocity = 0.0;
            self.grounded = false;
            self.physics_accumulator = 0.0;
        }
    }

    fn tick(&mut self, input: &InputState, world: &ClientWorld) {
        self.update_sneaking(input.sneak, world);
        self.sprinting = input.sprint && input.forward && !input.sneak;

        let forward = self.forward();
        let flat_forward = Vec3::new(forward.x, 0.0, forward.z).normalize_or_zero();
        let right = flat_forward.cross(Vec3::Y).normalize_or_zero();
        let mut movement = Vec3::ZERO;

        if input.forward {
            movement += flat_forward;
        }
        if input.backward {
            movement -= flat_forward;
        }
        if input.right {
            movement += right;
        }
        if input.left {
            movement -= right;
        }

        if movement.length_squared() > 0.0 {
            let mut acceleration = if self.grounded {
                WALK_ACCELERATION
            } else {
                AIR_ACCELERATION
            };
            if self.sprinting {
                acceleration *= SPRINT_MULTIPLIER;
            }
            if input.sneak {
                acceleration *= SNEAK_MULTIPLIER;
            }
            self.horizontal_velocity += movement.normalize() * acceleration;
        }

        let jumped = input.jump && self.grounded;
        if jumped {
            self.vertical_velocity = JUMP_VELOCITY;
            if self.sprinting {
                self.horizontal_velocity += flat_forward * SPRINT_JUMP_BOOST;
            }
            self.grounded = false;
        }

        if !jumped {
            self.vertical_velocity -= GRAVITY_PER_TICK;
            self.vertical_velocity *= AIR_DRAG;
        }
        self.grounded = false;
        let vertical_delta = Vec3::new(0.0, self.vertical_velocity, 0.0);
        if !self.try_move(vertical_delta, world) {
            if vertical_delta.y < 0.0 {
                self.grounded = true;
            }
            self.vertical_velocity = 0.0;
        }
        if jumped && self.vertical_velocity != 0.0 {
            self.vertical_velocity -= GRAVITY_PER_TICK;
            self.vertical_velocity *= AIR_DRAG;
        }

        self.move_horizontally(world, input.sneak);
        self.horizontal_velocity *= if self.grounded {
            GROUND_FRICTION
        } else {
            AIR_HORIZONTAL_DRAG
        };
    }

    fn update_sneaking(&mut self, sneak: bool, world: &ClientWorld) {
        if self.sneaking == sneak {
            return;
        }

        let old_eye_height = self.eye_height();
        self.sneaking = sneak;
        let new_eye_height = self.eye_height();
        let adjusted = self.position + Vec3::Y * (new_eye_height - old_eye_height);
        if world.collides_player_at_eye_height(adjusted, new_eye_height) {
            self.sneaking = !sneak;
        } else {
            self.position = adjusted;
        }
    }

    fn move_horizontally(&mut self, world: &ClientWorld, sneak: bool) {
        let delta_x = Vec3::new(self.horizontal_velocity.x, 0.0, 0.0);
        if !self.try_horizontal_move(delta_x, world, sneak) && self.grounded {
            self.horizontal_velocity.x = 0.0;
        }

        let delta_z = Vec3::new(0.0, 0.0, self.horizontal_velocity.z);
        if !self.try_horizontal_move(delta_z, world, sneak) && self.grounded {
            self.horizontal_velocity.z = 0.0;
        }
    }

    fn try_horizontal_move(&mut self, delta: Vec3, world: &ClientWorld, sneak: bool) -> bool {
        if delta.length_squared() == 0.0 {
            return true;
        }

        let original = self.position;
        if self.try_move(delta, world)
            && (!sneak
                || !self.grounded
                || world.has_player_ground_support(self.position, self.eye_height()))
        {
            return true;
        }
        self.position = original;

        if !self.grounded || sneak {
            return false;
        }

        if self.try_move(Vec3::Y * STEP_HEIGHT, world) && self.try_move(delta, world) {
            self.try_move(Vec3::Y * -STEP_HEIGHT, world);
            return true;
        }

        self.position = original;
        false
    }

    fn try_move(&mut self, delta: Vec3, world: &ClientWorld) -> bool {
        if delta.length_squared() == 0.0 {
            return true;
        }

        let next = self.position + delta;
        if world.collides_player_at_eye_height(next, self.eye_height()) {
            return false;
        }

        self.position = next;
        true
    }

    fn apply_mouse_delta(&mut self, delta_x: f32, delta_y: f32) {
        let sensitivity = 0.0025;
        self.yaw += delta_x * sensitivity;
        self.pitch = (self.pitch - delta_y * sensitivity).clamp(-1.553, 1.553);
    }

    fn view_projection(&self, width: u32, height: u32) -> Mat4 {
        let aspect = width.max(1) as f32 / height.max(1) as f32;
        let view = Mat4::look_to_rh(self.position, self.forward(), Vec3::Y);
        let fov = if self.sprinting {
            SPRINT_FOV_DEGREES
        } else {
            NORMAL_FOV_DEGREES
        };
        let projection = Mat4::perspective_rh(fov.to_radians(), aspect, 0.1, 500.0);
        projection * view
    }

    fn eye_height(&self) -> f32 {
        if self.sneaking {
            PLAYER_SNEAKING_EYE_HEIGHT
        } else {
            PLAYER_STANDING_EYE_HEIGHT
        }
    }

    fn forward(&self) -> Vec3 {
        Vec3::new(
            self.yaw.cos() * self.pitch.cos(),
            self.pitch.sin(),
            self.yaw.sin() * self.pitch.cos(),
        )
        .normalize()
    }
}

#[derive(Debug, Default)]
struct InputState {
    forward: bool,
    backward: bool,
    left: bool,
    right: bool,
    jump: bool,
    sneak: bool,
    sprint: bool,
    last_forward_press: Option<Instant>,
}

impl InputState {
    fn handle_key(&mut self, event: &KeyEvent) {
        let pressed = event.state == ElementState::Pressed;
        self.handle_logical_key_at(event.logical_key.as_ref(), pressed, Instant::now());

        if let PhysicalKey::Code(code) = event.physical_key {
            match code {
                KeyCode::Space => self.jump = pressed,
                KeyCode::ShiftLeft | KeyCode::ShiftRight => self.sneak = pressed,
                _ => {}
            }
        }

        if matches!(event.logical_key.as_ref(), Key::Named(NamedKey::Shift)) {
            self.sneak = pressed;
        }
    }

    #[cfg(test)]
    fn handle_logical_key(&mut self, key: Key<&str>, pressed: bool) {
        self.handle_logical_key_at(key, pressed, Instant::now());
    }

    fn handle_logical_key_at(&mut self, key: Key<&str>, pressed: bool, now: Instant) {
        match key {
            Key::Character(character) => match character.to_lowercase().as_str() {
                "z" => {
                    if pressed && !self.forward {
                        if self.last_forward_press.is_some_and(|last| {
                            now.duration_since(last).as_secs_f32() <= SPRINT_DOUBLE_TAP_SECONDS
                        }) {
                            self.sprint = true;
                        }
                        self.last_forward_press = Some(now);
                    } else if !pressed {
                        self.sprint = false;
                    }
                    self.forward = pressed;
                }
                "s" => self.backward = pressed,
                "q" => self.left = pressed,
                "d" => self.right = pressed,
                _ => {}
            },
            _ => {}
        }
    }

    fn clear_movement(&mut self) {
        self.forward = false;
        self.backward = false;
        self.left = false;
        self.right = false;
        self.jump = false;
        self.sneak = false;
        self.sprint = false;
    }
}

#[repr(C)]
#[derive(Debug, Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
struct CameraUniform {
    view_proj: [[f32; 4]; 4],
}

impl CameraUniform {
    fn new(view_projection: Mat4) -> Self {
        Self {
            view_proj: view_projection.to_cols_array_2d(),
        }
    }
}

#[repr(C)]
#[derive(Debug, Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
struct Vertex {
    position: [f32; 3],
    color: [f32; 3],
    tex_coords: [f32; 2],
}

impl Vertex {
    fn layout() -> wgpu::VertexBufferLayout<'static> {
        wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<Vertex>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &[
                wgpu::VertexAttribute {
                    offset: 0,
                    shader_location: 0,
                    format: wgpu::VertexFormat::Float32x3,
                },
                wgpu::VertexAttribute {
                    offset: std::mem::size_of::<[f32; 3]>() as wgpu::BufferAddress,
                    shader_location: 1,
                    format: wgpu::VertexFormat::Float32x3,
                },
                wgpu::VertexAttribute {
                    offset: (std::mem::size_of::<[f32; 3]>() * 2) as wgpu::BufferAddress,
                    shader_location: 2,
                    format: wgpu::VertexFormat::Float32x2,
                },
            ],
        }
    }
}

struct TextureAtlas {
    _texture: wgpu::Texture,
    view: wgpu::TextureView,
    sampler: wgpu::Sampler,
    tiles: HashMap<String, AtlasTile>,
    fallback: AtlasTile,
}

impl TextureAtlas {
    const TILE_SIZE: u32 = 16;

    fn load(
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
        }
        for key in item_texture_keys(items) {
            if key != "humancraft:missing" && !texture_keys.contains(&key.to_string()) {
                texture_keys.push(key.to_string());
            }
        }

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

    fn tile(&self, key: &str) -> AtlasTile {
        self.tiles.get(key).copied().unwrap_or(self.fallback)
    }
}

#[derive(Debug, Copy, Clone)]
struct AtlasTile {
    min_u: f32,
    max_u: f32,
    min_v: f32,
    max_v: f32,
}

impl AtlasTile {
    fn uv_quad(self) -> [[f32; 2]; 4] {
        [
            [self.min_u, self.max_v],
            [self.max_u, self.max_v],
            [self.max_u, self.min_v],
            [self.min_u, self.min_v],
        ]
    }
}

struct Texture {
    view: wgpu::TextureView,
}

impl Texture {
    const DEPTH_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Depth32Float;

    fn create_depth_texture(device: &wgpu::Device, config: &wgpu::SurfaceConfiguration) -> Self {
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

fn build_render_mesh(
    chunk_meshes: &[(ChunkPosition, ChunkMesh)],
    blocks: &BlockRegistry,
    texture_atlas: &TextureAtlas,
    render_bounds: Option<RenderChunkBounds>,
) -> (Vec<Vertex>, Vec<u32>) {
    let quad_count = chunk_meshes
        .iter()
        .map(|(_, mesh)| mesh.quads.len())
        .sum::<usize>();
    let mut vertices = Vec::with_capacity(quad_count * 4);
    let mut indices = Vec::with_capacity(quad_count * 6);

    for (chunk_position, mesh) in chunk_meshes {
        let chunk_offset_x = chunk_position.x as f32 * 16.0;
        let chunk_offset_z = chunk_position.z as f32 * 16.0;
        for quad in &mesh.quads {
            if !should_render_preview_quad(quad, *chunk_position, render_bounds) {
                continue;
            }

            let base = vertices.len() as u32;
            let (color, texture_key) = render_material(quad.block, quad.direction, blocks);
            let tex_coords = texture_atlas.tile(&texture_key).uv_quad();
            for (index, vertex) in quad.vertices.into_iter().enumerate() {
                vertices.push(Vertex {
                    position: [
                        vertex[0] + chunk_offset_x - 8.0,
                        vertex[1] - 64.0,
                        vertex[2] + chunk_offset_z - 8.0,
                    ],
                    color,
                    tex_coords: tex_coords[index],
                });
            }
            indices.extend_from_slice(&[base, base + 1, base + 2, base, base + 2, base + 3]);
        }
    }

    (vertices, indices)
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
struct RenderChunkBounds {
    min_x: i32,
    max_x: i32,
    min_z: i32,
    max_z: i32,
}

impl RenderChunkBounds {
    fn from_chunk_positions(positions: impl IntoIterator<Item = ChunkPosition>) -> Option<Self> {
        let mut positions = positions.into_iter();
        let first = positions.next()?;
        let mut bounds = Self {
            min_x: first.x,
            max_x: first.x,
            min_z: first.z,
            max_z: first.z,
        };

        for position in positions {
            bounds.min_x = bounds.min_x.min(position.x);
            bounds.max_x = bounds.max_x.max(position.x);
            bounds.min_z = bounds.min_z.min(position.z);
            bounds.max_z = bounds.max_z.max(position.z);
        }

        Some(bounds)
    }
}

fn should_render_preview_quad(
    quad: &crate::engine::mesh::chunk_mesher::MeshQuad,
    chunk_position: ChunkPosition,
    render_bounds: Option<RenderChunkBounds>,
) -> bool {
    if is_outer_render_boundary(quad, chunk_position, render_bounds) {
        return false;
    }

    true
}

fn is_outer_render_boundary(
    quad: &crate::engine::mesh::chunk_mesher::MeshQuad,
    chunk_position: ChunkPosition,
    render_bounds: Option<RenderChunkBounds>,
) -> bool {
    use crate::engine::mesh::chunk_mesher::FaceDirection;

    let Some(render_bounds) = render_bounds else {
        return false;
    };

    let offset_x = chunk_position.x as f32 * CHUNK_WORLD_SIZE;
    let offset_z = chunk_position.z as f32 * CHUNK_WORLD_SIZE;
    let render_min_world_x = render_bounds.min_x as f32 * CHUNK_WORLD_SIZE;
    let render_max_world_x = (render_bounds.max_x + 1) as f32 * CHUNK_WORLD_SIZE;
    let render_min_world_z = render_bounds.min_z as f32 * CHUNK_WORLD_SIZE;
    let render_max_world_z = (render_bounds.max_z + 1) as f32 * CHUNK_WORLD_SIZE;
    match quad.direction {
        FaceDirection::West => quad
            .vertices
            .iter()
            .all(|vertex| nearly_equal(vertex[0] + offset_x, render_min_world_x)),
        FaceDirection::East => quad
            .vertices
            .iter()
            .all(|vertex| nearly_equal(vertex[0] + offset_x, render_max_world_x)),
        FaceDirection::North => quad
            .vertices
            .iter()
            .all(|vertex| nearly_equal(vertex[2] + offset_z, render_min_world_z)),
        FaceDirection::South => quad
            .vertices
            .iter()
            .all(|vertex| nearly_equal(vertex[2] + offset_z, render_max_world_z)),
        FaceDirection::Up => false,
        FaceDirection::Down => quad
            .vertices
            .iter()
            .all(|vertex| nearly_equal(vertex[1], 0.0)),
    }
}

fn nearly_equal(left: f32, right: f32) -> bool {
    (left - right).abs() < 0.001
}

fn capture_cursor(window: &Window) {
    if window.set_cursor_grab(CursorGrabMode::Locked).is_err() {
        let _ = window.set_cursor_grab(CursorGrabMode::Confined);
    }
    window.set_cursor_visible(false);
}

fn release_cursor(window: &Window) {
    let _ = window.set_cursor_grab(CursorGrabMode::None);
    window.set_cursor_visible(true);
}

fn is_confirm_key(event: &KeyEvent) -> bool {
    matches!(event.logical_key.as_ref(), Key::Named(NamedKey::Enter))
}

fn character_key(event: &KeyEvent, expected: &str) -> bool {
    matches!(event.logical_key.as_ref(), Key::Character(character) if character.eq_ignore_ascii_case(expected))
}

fn is_inventory_key(event: &KeyEvent) -> bool {
    event.state == ElementState::Pressed && character_key(event, DEFAULT_INVENTORY_KEY)
}

fn inventory_mouse_button(button: MouseButton) -> Option<InventoryMouseButton> {
    match button {
        MouseButton::Left => Some(InventoryMouseButton::Left),
        MouseButton::Right => Some(InventoryMouseButton::Right),
        _ => None,
    }
}

fn current_save_time() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|duration| duration.as_secs())
        .unwrap_or(0)
}

#[derive(Debug, Copy, Clone)]
struct UiPoint {
    x: f32,
    y: f32,
}

fn cursor_to_ui_point(position: PhysicalPosition<f64>, size: PhysicalSize<u32>) -> UiPoint {
    UiPoint {
        x: (position.x / size.width.max(1) as f64 * 2.0 - 1.0) as f32,
        y: (1.0 - position.y / size.height.max(1) as f64 * 2.0) as f32,
    }
}

#[derive(Debug, Copy, Clone)]
struct UiRect {
    left: f32,
    right: f32,
    bottom: f32,
    top: f32,
}

impl UiRect {
    const fn new(left: f32, bottom: f32, right: f32, top: f32) -> Self {
        Self {
            left,
            right,
            bottom,
            top,
        }
    }

    fn contains(self, point: UiPoint) -> bool {
        point.x >= self.left
            && point.x <= self.right
            && point.y >= self.bottom
            && point.y <= self.top
    }

    fn center_x(self) -> f32 {
        (self.left + self.right) * 0.5
    }

    fn center_y(self) -> f32 {
        (self.bottom + self.top) * 0.5
    }
}

const UI_MAIN_PLAY: UiRect = UiRect::new(-0.28, -0.05, 0.28, 0.08);
const UI_WORLDS_PLAY: UiRect = UiRect::new(0.38, 0.42, 0.78, 0.54);
const UI_WORLDS_NEW: UiRect = UiRect::new(0.38, 0.25, 0.78, 0.37);
const UI_WORLDS_RENAME: UiRect = UiRect::new(0.38, 0.08, 0.78, 0.20);
const UI_WORLDS_DELETE: UiRect = UiRect::new(0.38, -0.09, 0.78, 0.03);
const UI_WORLDS_BACK: UiRect = UiRect::new(0.38, -0.46, 0.78, -0.34);
const UI_CONFIG_NAME_FIELD: UiRect = UiRect::new(-0.30, 0.22, 0.54, 0.34);
const UI_CONFIG_SEED_FIELD: UiRect = UiRect::new(-0.30, -0.02, 0.54, 0.10);
const UI_CONFIG_CREATE: UiRect = UiRect::new(-0.30, -0.32, 0.04, -0.20);
const UI_CONFIG_BACK: UiRect = UiRect::new(0.20, -0.32, 0.54, -0.20);
const UI_RENAME_SAVE: UiRect = UiRect::new(-0.30, -0.20, 0.04, -0.08);
const UI_RENAME_BACK: UiRect = UiRect::new(0.20, -0.20, 0.54, -0.08);
const UI_PAUSE_KEEP_PLAYING: UiRect = UiRect::new(-0.46, -0.08, -0.02, 0.05);
const UI_PAUSE_SAVE_QUIT: UiRect = UiRect::new(0.02, -0.08, 0.46, 0.05);

fn world_list_hit_index(point: UiPoint, world_count: usize) -> Option<usize> {
    let count = world_count.min(7);
    for index in 0..count {
        let top = 0.45 - index as f32 * 0.13;
        let rect = UiRect::new(-0.78, top - 0.10, 0.26, top);
        if rect.contains(point) {
            return Some(index);
        }
    }
    None
}

fn build_menu_mesh(state: &RenderState) -> (Vec<Vertex>, Vec<u32>) {
    let mut ui = UiMeshBuilder::default();
    match state.mode {
        AppMode::MainMenu => {
            ui.rect(UiRect::new(-1.0, -1.0, 1.0, 1.0), [0.11, 0.13, 0.14]);
            ui.center_text(0.0, 0.52, 0.018, [0.92, 0.92, 0.88], "HUMANCRAFT");
            ui.button(UI_MAIN_PLAY, "PLAY", false);
        }
        AppMode::ManageWorlds => {
            ui.rect(UiRect::new(-1.0, -1.0, 1.0, 1.0), [0.10, 0.12, 0.13]);
            ui.center_text(0.0, 0.72, 0.012, [0.92, 0.92, 0.88], "MANAGE WORLDS");
            if state.worlds.is_empty() {
                ui.text(-0.76, 0.40, 0.007, [0.82, 0.82, 0.78], "NO WORLDS YET");
                ui.text(
                    -0.76,
                    0.28,
                    0.006,
                    [0.64, 0.66, 0.66],
                    "CREATE A WORLD TO START",
                );
            } else {
                for (index, world) in state.worlds.iter().take(7).enumerate() {
                    let top = 0.45 - index as f32 * 0.13;
                    let rect = UiRect::new(-0.78, top - 0.10, 0.26, top);
                    ui.rect(
                        rect,
                        if index == state.selected_world {
                            [0.32, 0.36, 0.34]
                        } else {
                            [0.18, 0.20, 0.20]
                        },
                    );
                    ui.text(
                        rect.left + 0.03,
                        rect.top - 0.028,
                        0.0048,
                        [0.95, 0.95, 0.90],
                        &world.name,
                    );
                    ui.text(
                        rect.left + 0.03,
                        rect.bottom + 0.030,
                        0.0036,
                        [0.66, 0.68, 0.68],
                        &format!("SEED {}", world.seed),
                    );
                }
            }
            ui.button(UI_WORLDS_PLAY, "PLAY", false);
            ui.button(UI_WORLDS_NEW, "NEW WORLD", false);
            ui.button(UI_WORLDS_RENAME, "RENAME", state.worlds.is_empty());
            ui.button(UI_WORLDS_DELETE, "DELETE", state.worlds.is_empty());
            ui.button(UI_WORLDS_BACK, "BACK", false);
        }
        AppMode::ConfigNewWorld => {
            ui.rect(UiRect::new(-1.0, -1.0, 1.0, 1.0), [0.10, 0.12, 0.13]);
            ui.center_text(0.0, 0.68, 0.012, [0.92, 0.92, 0.88], "CONFIG NEW WORLD");
            ui.text(-0.54, 0.38, 0.006, [0.85, 0.85, 0.80], "WORLD NAME");
            ui.field(
                UI_CONFIG_NAME_FIELD,
                &state.new_world_config.name,
                state.new_world_config.focused == ConfigField::Name,
            );
            ui.text(-0.54, 0.14, 0.006, [0.85, 0.85, 0.80], "SEED");
            ui.field(
                UI_CONFIG_SEED_FIELD,
                if state.new_world_config.seed.is_empty() {
                    "AUTO"
                } else {
                    &state.new_world_config.seed
                },
                state.new_world_config.focused == ConfigField::Seed,
            );
            ui.text(
                -0.54,
                -0.08,
                0.005,
                [0.64, 0.66, 0.66],
                "SAME NUMERIC SEED RECREATES TERRAIN",
            );
            ui.button(UI_CONFIG_CREATE, "CREATE", false);
            ui.button(UI_CONFIG_BACK, "BACK", false);
        }
        AppMode::RenamingWorld => {
            ui.rect(UiRect::new(-1.0, -1.0, 1.0, 1.0), [0.10, 0.12, 0.13]);
            ui.center_text(0.0, 0.58, 0.012, [0.92, 0.92, 0.88], "RENAME WORLD");
            ui.field(
                UI_CONFIG_NAME_FIELD,
                self_clamped_text(state.text_entry.display()),
                true,
            );
            ui.button(UI_RENAME_SAVE, "SAVE", false);
            ui.button(UI_RENAME_BACK, "BACK", false);
        }
        AppMode::InGame => {
            ui.rect(UiRect::new(-0.52, -0.22, 0.52, 0.30), [0.08, 0.09, 0.10]);
            ui.center_text(0.0, 0.16, 0.012, [0.92, 0.92, 0.88], "PAUSED");
            ui.button(UI_PAUSE_KEEP_PLAYING, "KEEP PLAYING", false);
            ui.button(UI_PAUSE_SAVE_QUIT, "SAVE & QUIT", false);
        }
    }
    ui.finish()
}

fn build_gameplay_ui_mesh(
    world: &ClientWorld,
    inventory_open: bool,
    aspect: f32,
    selected_hotbar_slot: usize,
    cursor_stack: Option<ItemStack>,
    cursor_point: UiPoint,
) -> (Vec<Vertex>, Vec<u32>) {
    let mut ui = UiMeshBuilder::default();
    if inventory_open {
        ui.rect(UiRect::new(-0.64, -0.52, 0.64, 0.56), [0.03, 0.03, 0.03]);
        ui.rect(UiRect::new(-0.62, -0.50, 0.62, 0.54), [0.58, 0.58, 0.56]);
        ui.rect(UiRect::new(-0.59, -0.43, 0.59, 0.45), [0.43, 0.43, 0.41]);
        ui.rect(UiRect::new(-0.56, -0.40, 0.56, 0.36), [0.50, 0.50, 0.48]);
        ui.center_text(0.0, 0.47, 0.007, [0.18, 0.18, 0.17], "INVENTORY");
        ui.center_text(0.0, -0.455, 0.0048, [0.20, 0.20, 0.19], "E OR ESC TO CLOSE");
        for index in 0..world.player_inventory.slots().len() {
            let rect = inventory_slot_rect(index, true, aspect);
            draw_inventory_slot(&mut ui, rect, index == selected_hotbar_slot);
        }
    } else {
        for index in 0..INVENTORY_HOTBAR_SLOTS {
            draw_inventory_slot(
                &mut ui,
                inventory_slot_rect(index, false, aspect),
                index == selected_hotbar_slot,
            );
        }
        if world.player_inventory.slot(selected_hotbar_slot).is_none() {
            draw_player_arm(&mut ui, aspect);
        }
    }

    for (index, stack) in world.player_inventory.slots().iter().enumerate() {
        if !inventory_open && index >= INVENTORY_HOTBAR_SLOTS {
            continue;
        }
        let Some(stack) = stack else {
            continue;
        };
        let rect = inventory_slot_rect(index, inventory_open, aspect);
        if stack.count > 1 {
            ui.text(
                rect.right - slot_width(rect) * 0.38,
                rect.bottom + slot_height(rect) * 0.30,
                0.0038,
                [0.96, 0.96, 0.90],
                &stack.count.to_string(),
            );
        }
    }

    if inventory_open {
        if let Some(stack) = cursor_stack {
            if stack.count > 1 {
                let rect = carried_item_rect(cursor_point, aspect);
                ui.text(
                    rect.right - slot_width(rect) * 0.38,
                    rect.bottom + slot_height(rect) * 0.30,
                    0.0038,
                    [0.96, 0.96, 0.90],
                    &stack.count.to_string(),
                );
            }
        }
    }

    ui.finish()
}

fn build_inventory_icon_mesh(
    world: &ClientWorld,
    texture_atlas: &TextureAtlas,
    inventory_open: bool,
    aspect: f32,
    selected_hotbar_slot: usize,
    cursor_stack: Option<ItemStack>,
    cursor_point: UiPoint,
) -> (Vec<Vertex>, Vec<u32>) {
    let mut vertices = Vec::new();
    let mut indices = Vec::new();
    for (index, stack) in world.player_inventory.slots().iter().enumerate() {
        if !inventory_open && index >= INVENTORY_HOTBAR_SLOTS {
            continue;
        }
        let Some(stack) = stack else {
            continue;
        };
        let Some(definition) = world.items.get(stack.item) else {
            continue;
        };
        let rect = inventory_icon_rect(inventory_slot_rect(index, inventory_open, aspect));
        push_textured_ui_rect(
            &mut vertices,
            &mut indices,
            rect,
            texture_atlas.tile(&definition.texture),
        );
    }
    if !inventory_open {
        if let Some(stack) = world.player_inventory.slot(selected_hotbar_slot) {
            if let Some(definition) = world.items.get(stack.item) {
                push_held_item_mesh(
                    world,
                    texture_atlas,
                    &mut vertices,
                    &mut indices,
                    definition,
                    aspect,
                );
            }
        }
    }
    if inventory_open {
        if let Some(stack) = cursor_stack {
            if let Some(definition) = world.items.get(stack.item) {
                push_textured_ui_rect(
                    &mut vertices,
                    &mut indices,
                    carried_item_rect(cursor_point, aspect),
                    texture_atlas.tile(&definition.texture),
                );
            }
        }
    }
    (vertices, indices)
}

fn build_loot_mesh(
    world: &ClientWorld,
    texture_atlas: &TextureAtlas,
    _camera: &Camera,
) -> (Vec<Vertex>, Vec<u32>) {
    let mut vertices = Vec::new();
    let mut indices = Vec::new();
    for loot in &world.loot_entities {
        let Some(definition) = world.items.get(loot.stack.item) else {
            continue;
        };
        let tile = texture_atlas.tile(&definition.texture);
        let corners = loot_billboard_corners(loot);
        let tex_coords = tile.uv_quad();
        let base = vertices.len() as u32;
        for (index, corner) in corners.into_iter().enumerate() {
            vertices.push(Vertex {
                position: corner.to_array(),
                color: [1.0, 1.0, 1.0],
                tex_coords: tex_coords[index],
            });
        }
        indices.extend_from_slice(&[
            base,
            base + 1,
            base + 2,
            base,
            base + 2,
            base + 3,
            base + 2,
            base + 1,
            base,
            base + 3,
            base + 2,
            base,
        ]);
    }
    (vertices, indices)
}

fn loot_billboard_corners(loot: &LootEntity) -> [Vec3; 4] {
    let axis_x = Vec3::new(
        loot.rotation_radians.cos(),
        0.0,
        loot.rotation_radians.sin(),
    ) * LOOT_RENDER_HALF_SIZE;
    let axis_y = Vec3::Y * LOOT_RENDER_HALF_SIZE;
    let center = loot.position + Vec3::Y * LOOT_RENDER_HALF_SIZE;
    [
        center - axis_x - axis_y,
        center + axis_x - axis_y,
        center + axis_x + axis_y,
        center - axis_x + axis_y,
    ]
}

fn draw_inventory_slot(ui: &mut UiMeshBuilder, rect: UiRect, selected: bool) {
    ui.rect(rect, [0.04, 0.04, 0.04]);
    ui.rect(
        inset_rect(rect, 0.004),
        if selected {
            [0.92, 0.94, 0.82]
        } else {
            [0.62, 0.62, 0.60]
        },
    );
    ui.rect(inset_rect(rect, 0.010), [0.28, 0.29, 0.28]);
    ui.rect(
        UiRect::new(
            rect.left + slot_width(rect) * 0.14,
            rect.top - slot_height(rect) * 0.16,
            rect.right - slot_width(rect) * 0.10,
            rect.top - slot_height(rect) * 0.08,
        ),
        [0.40, 0.41, 0.39],
    );
}

fn draw_player_arm(ui: &mut UiMeshBuilder, aspect: f32) {
    for face in player_arm_overlay_faces(aspect) {
        ui.quad(face.positions, face.color);
    }
}

fn inventory_slot_rect(index: usize, inventory_open: bool, aspect: f32) -> UiRect {
    let slot_height = 0.112;
    let slot_width = slot_height / aspect.max(0.1);
    let gap_y = 0.012;
    let gap_x = gap_y / aspect.max(0.1);
    if inventory_open {
        let columns = 9;
        let (row, column, top_start) = if index < INVENTORY_HOTBAR_SLOTS {
            (0, index, -0.20)
        } else {
            let inventory_index = index - INVENTORY_HOTBAR_SLOTS;
            (inventory_index / columns, inventory_index % columns, 0.24)
        };
        let total_width = columns as f32 * slot_width + (columns - 1) as f32 * gap_x;
        let left = -total_width * 0.5 + column as f32 * (slot_width + gap_x);
        let top = top_start - row as f32 * (slot_height + gap_y);
        UiRect::new(left, top - slot_height, left + slot_width, top)
    } else {
        let total_width = INVENTORY_HOTBAR_SLOTS as f32 * slot_width
            + (INVENTORY_HOTBAR_SLOTS - 1) as f32 * gap_x;
        let left = -total_width * 0.5 + index as f32 * (slot_width + gap_x);
        UiRect::new(left, -0.95, left + slot_width, -0.95 + slot_height)
    }
}

fn inventory_slot_at_point(point: UiPoint, aspect: f32) -> Option<usize> {
    for index in 0..Inventory::player().slot_count() {
        if inventory_slot_rect(index, true, aspect).contains(point) {
            return Some(index);
        }
    }
    None
}

fn inset_rect(rect: UiRect, inset: f32) -> UiRect {
    UiRect::new(
        rect.left + inset,
        rect.bottom + inset,
        rect.right - inset,
        rect.top - inset,
    )
}

fn inventory_icon_rect(rect: UiRect) -> UiRect {
    let width = slot_width(rect);
    let height = slot_height(rect);
    UiRect::new(
        rect.left + width * 0.18,
        rect.bottom + height * 0.34,
        rect.right - width * 0.22,
        rect.top - height * 0.12,
    )
}

fn carried_item_rect(point: UiPoint, aspect: f32) -> UiRect {
    let height = 0.10;
    let width = height / aspect.max(0.1);
    UiRect::new(
        point.x - width * 0.5,
        point.y - height * 0.5,
        point.x + width * 0.5,
        point.y + height * 0.5,
    )
}

fn push_held_item_mesh(
    world: &ClientWorld,
    texture_atlas: &TextureAtlas,
    vertices: &mut Vec<Vertex>,
    indices: &mut Vec<u32>,
    item: &crate::engine::world::ItemDefinition,
    aspect: f32,
) {
    if let Some(block) = item
        .place_block
        .as_ref()
        .and_then(|key| world.blocks.get_by_key(key))
        .map(|(_, block)| block)
    {
        push_held_block_mesh(vertices, indices, texture_atlas, block, aspect);
    } else {
        push_held_sprite_mesh(vertices, indices, texture_atlas.tile(&item.texture), aspect);
    }
}

fn push_held_block_mesh(
    vertices: &mut Vec<Vertex>,
    indices: &mut Vec<u32>,
    texture_atlas: &TextureAtlas,
    block: &crate::engine::world::BlockDefinition,
    aspect: f32,
) {
    let faces = held_block_overlay_faces(aspect);

    push_textured_ui_quad(
        vertices,
        indices,
        faces.front,
        texture_atlas.tile(&block.textures.south),
        [0.95, 0.95, 0.95],
    );
    push_textured_ui_quad(
        vertices,
        indices,
        faces.right,
        texture_atlas.tile(&block.textures.east),
        [0.72, 0.72, 0.72],
    );
    push_textured_ui_quad(
        vertices,
        indices,
        faces.top,
        texture_atlas.tile(&block.textures.top),
        [1.0, 1.0, 1.0],
    );
}

struct HeldBlockOverlayFaces {
    front: [[f32; 3]; 4],
    right: [[f32; 3]; 4],
    top: [[f32; 3]; 4],
}

fn held_block_overlay_faces(aspect: f32) -> HeldBlockOverlayFaces {
    let scale_x = 1.0 / aspect.max(0.1);
    let adjust = |x: f32, y: f32| [0.72 + (x - 0.72) * scale_x, y, 0.0];
    let front_bottom_left = adjust(0.45, -0.96);
    let front_bottom_right = adjust(0.82, -0.88);
    let front_top_right = adjust(0.82, -0.50);
    let front_top_left = adjust(0.45, -0.58);
    let depth = |point: [f32; 3]| [point[0] + 0.15 * scale_x, point[1] + 0.14, 0.0];
    let back_top_left = depth(front_top_left);
    let back_top_right = depth(front_top_right);
    let back_bottom_right = depth(front_bottom_right);

    HeldBlockOverlayFaces {
        front: [
            front_bottom_left,
            front_bottom_right,
            front_top_right,
            front_top_left,
        ],
        right: [
            front_bottom_right,
            back_bottom_right,
            back_top_right,
            front_top_right,
        ],
        top: [
            front_top_left,
            front_top_right,
            back_top_right,
            back_top_left,
        ],
    }
}

#[derive(Copy, Clone)]
struct UiFace {
    positions: [[f32; 3]; 4],
    color: [f32; 3],
}

fn player_arm_overlay_faces(aspect: f32) -> [UiFace; 3] {
    let scale_x = 1.0 / aspect.max(0.1);
    let adjust = |x: f32, y: f32| [0.78 + (x - 0.78) * scale_x, y, 0.0];
    let wrist_left = adjust(0.64, -0.99);
    let wrist_right = adjust(0.91, -0.99);
    let elbow_left = adjust(0.56, -0.61);
    let elbow_right = adjust(0.78, -0.49);
    let depth = |point: [f32; 3]| [point[0] + 0.18 * scale_x, point[1] + 0.07, 0.0];
    let wrist_right_back = depth(wrist_right);
    let elbow_right_back = depth(elbow_right);
    let elbow_left_back = depth(elbow_left);

    [
        UiFace {
            positions: [wrist_left, wrist_right, elbow_right, elbow_left],
            color: [0.70, 0.46, 0.30],
        },
        UiFace {
            positions: [wrist_right, wrist_right_back, elbow_right_back, elbow_right],
            color: [0.50, 0.31, 0.20],
        },
        UiFace {
            positions: [elbow_left, elbow_right, elbow_right_back, elbow_left_back],
            color: [0.82, 0.57, 0.38],
        },
    ]
}

fn push_held_sprite_mesh(
    vertices: &mut Vec<Vertex>,
    indices: &mut Vec<u32>,
    tile: AtlasTile,
    aspect: f32,
) {
    let scale_x = 1.0 / aspect.max(0.1);
    let adjust = |x: f32, y: f32| [0.76 + (x - 0.76) * scale_x, y, 0.0];
    push_textured_ui_quad(
        vertices,
        indices,
        [
            adjust(0.58, -0.94),
            adjust(0.96, -0.80),
            adjust(0.84, -0.39),
            adjust(0.46, -0.53),
        ],
        tile,
        [1.0, 1.0, 1.0],
    );
}

fn slot_width(rect: UiRect) -> f32 {
    rect.right - rect.left
}

fn slot_height(rect: UiRect) -> f32 {
    rect.top - rect.bottom
}

fn push_textured_ui_rect(
    vertices: &mut Vec<Vertex>,
    indices: &mut Vec<u32>,
    rect: UiRect,
    tile: AtlasTile,
) {
    let tex_coords = tile.uv_quad();
    let base = vertices.len() as u32;
    let positions = [
        [rect.left, rect.bottom, 0.0],
        [rect.right, rect.bottom, 0.0],
        [rect.right, rect.top, 0.0],
        [rect.left, rect.top, 0.0],
    ];
    for index in 0..4 {
        vertices.push(Vertex {
            position: positions[index],
            color: [1.0, 1.0, 1.0],
            tex_coords: tex_coords[index],
        });
    }
    indices.extend_from_slice(&[base, base + 1, base + 2, base, base + 2, base + 3]);
}

fn push_textured_ui_quad(
    vertices: &mut Vec<Vertex>,
    indices: &mut Vec<u32>,
    positions: [[f32; 3]; 4],
    tile: AtlasTile,
    color: [f32; 3],
) {
    let tex_coords = tile.uv_quad();
    let base = vertices.len() as u32;
    for index in 0..4 {
        vertices.push(Vertex {
            position: positions[index],
            color,
            tex_coords: tex_coords[index],
        });
    }
    indices.extend_from_slice(&[base, base + 1, base + 2, base, base + 2, base + 3]);
}

fn self_clamped_text(text: &str) -> &str {
    text
}

#[derive(Default)]
struct UiMeshBuilder {
    vertices: Vec<Vertex>,
    indices: Vec<u32>,
}

impl UiMeshBuilder {
    fn finish(self) -> (Vec<Vertex>, Vec<u32>) {
        (self.vertices, self.indices)
    }

    fn button(&mut self, rect: UiRect, label: &str, disabled: bool) {
        self.rect(
            rect,
            if disabled {
                [0.16, 0.16, 0.16]
            } else {
                [0.24, 0.24, 0.24]
            },
        );
        self.rect(
            UiRect::new(
                rect.left + 0.008,
                rect.bottom + 0.008,
                rect.right - 0.008,
                rect.top - 0.008,
            ),
            if disabled {
                [0.24, 0.24, 0.24]
            } else {
                [0.42, 0.42, 0.40]
            },
        );
        if !disabled {
            self.rect(
                UiRect::new(
                    rect.left + 0.014,
                    rect.top - 0.022,
                    rect.right - 0.014,
                    rect.top - 0.012,
                ),
                [0.64, 0.64, 0.60],
            );
            self.rect(
                UiRect::new(
                    rect.left + 0.014,
                    rect.bottom + 0.012,
                    rect.right - 0.014,
                    rect.bottom + 0.022,
                ),
                [0.29, 0.29, 0.28],
            );
        }
        let scale = button_label_scale(label, rect);
        self.center_text(
            rect.center_x(),
            rect.center_y() + 3.5 * scale,
            scale,
            if disabled {
                [0.50, 0.50, 0.50]
            } else {
                [0.95, 0.95, 0.92]
            },
            label,
        );
    }

    fn field(&mut self, rect: UiRect, value: &str, focused: bool) {
        self.rect(
            rect,
            if focused {
                [0.48, 0.48, 0.42]
            } else {
                [0.24, 0.25, 0.25]
            },
        );
        self.rect(
            UiRect::new(
                rect.left + 0.006,
                rect.bottom + 0.006,
                rect.right - 0.006,
                rect.top - 0.006,
            ),
            [0.12, 0.13, 0.13],
        );
        self.text(
            rect.left + 0.025,
            rect.bottom + 0.040,
            0.0058,
            [0.94, 0.94, 0.90],
            value,
        );
    }

    fn rect(&mut self, rect: UiRect, color: [f32; 3]) {
        self.quad(
            [
                [rect.left, rect.bottom, 0.0],
                [rect.right, rect.bottom, 0.0],
                [rect.right, rect.top, 0.0],
                [rect.left, rect.top, 0.0],
            ],
            color,
        );
    }

    fn quad(&mut self, positions: [[f32; 3]; 4], color: [f32; 3]) {
        let base = self.vertices.len() as u32;
        for position in positions {
            self.vertices.push(Vertex {
                position,
                color,
                tex_coords: [0.0, 0.0],
            });
        }
        self.indices
            .extend_from_slice(&[base, base + 1, base + 2, base, base + 2, base + 3]);
    }

    fn center_text(&mut self, center_x: f32, y: f32, scale: f32, color: [f32; 3], text: &str) {
        let width = text.chars().count() as f32 * 6.0 * scale;
        self.text(center_x - width * 0.5, y, scale, color, text);
    }

    fn text(&mut self, x: f32, y: f32, scale: f32, color: [f32; 3], text: &str) {
        let mut cursor_x = x;
        for character in text.chars() {
            self.glyph(cursor_x, y, scale, color, character);
            cursor_x += 6.0 * scale;
        }
    }

    fn glyph(&mut self, x: f32, y: f32, scale: f32, color: [f32; 3], character: char) {
        let glyph = glyph_rows(character);
        for (row, bits) in glyph.iter().enumerate() {
            for (column, bit) in bits.chars().enumerate() {
                if bit == '1' {
                    let left = x + column as f32 * scale;
                    let top = y - row as f32 * scale;
                    self.rect(UiRect::new(left, top - scale, left + scale, top), color);
                }
            }
        }
    }
}

fn button_label_scale(label: &str, rect: UiRect) -> f32 {
    let max_width = (rect.right - rect.left - 0.06).max(0.08);
    let base = 0.0054;
    let text_width = label.chars().count() as f32 * 6.0 * base;
    if text_width <= max_width {
        base
    } else {
        (max_width / (label.chars().count() as f32 * 6.0)).max(0.0038)
    }
}

fn glyph_rows(character: char) -> [&'static str; 7] {
    match character.to_ascii_uppercase() {
        'A' => [
            "01110", "10001", "10001", "11111", "10001", "10001", "10001",
        ],
        'B' => [
            "11110", "10001", "10001", "11110", "10001", "10001", "11110",
        ],
        'C' => [
            "01111", "10000", "10000", "10000", "10000", "10000", "01111",
        ],
        'D' => [
            "11110", "10001", "10001", "10001", "10001", "10001", "11110",
        ],
        'E' => [
            "11111", "10000", "10000", "11110", "10000", "10000", "11111",
        ],
        'F' => [
            "11111", "10000", "10000", "11110", "10000", "10000", "10000",
        ],
        'G' => [
            "01111", "10000", "10000", "10111", "10001", "10001", "01111",
        ],
        'H' => [
            "10001", "10001", "10001", "11111", "10001", "10001", "10001",
        ],
        'I' => [
            "11111", "00100", "00100", "00100", "00100", "00100", "11111",
        ],
        'J' => [
            "00111", "00010", "00010", "00010", "10010", "10010", "01100",
        ],
        'K' => [
            "10001", "10010", "10100", "11000", "10100", "10010", "10001",
        ],
        'L' => [
            "10000", "10000", "10000", "10000", "10000", "10000", "11111",
        ],
        'M' => [
            "10001", "11011", "10101", "10101", "10001", "10001", "10001",
        ],
        'N' => [
            "10001", "11001", "10101", "10011", "10001", "10001", "10001",
        ],
        'O' => [
            "01110", "10001", "10001", "10001", "10001", "10001", "01110",
        ],
        'P' => [
            "11110", "10001", "10001", "11110", "10000", "10000", "10000",
        ],
        'Q' => [
            "01110", "10001", "10001", "10001", "10101", "10010", "01101",
        ],
        'R' => [
            "11110", "10001", "10001", "11110", "10100", "10010", "10001",
        ],
        'S' => [
            "01111", "10000", "10000", "01110", "00001", "00001", "11110",
        ],
        'T' => [
            "11111", "00100", "00100", "00100", "00100", "00100", "00100",
        ],
        'U' => [
            "10001", "10001", "10001", "10001", "10001", "10001", "01110",
        ],
        'V' => [
            "10001", "10001", "10001", "10001", "10001", "01010", "00100",
        ],
        'W' => [
            "10001", "10001", "10001", "10101", "10101", "10101", "01010",
        ],
        'X' => [
            "10001", "10001", "01010", "00100", "01010", "10001", "10001",
        ],
        'Y' => [
            "10001", "10001", "01010", "00100", "00100", "00100", "00100",
        ],
        'Z' => [
            "11111", "00001", "00010", "00100", "01000", "10000", "11111",
        ],
        '0' => [
            "01110", "10001", "10011", "10101", "11001", "10001", "01110",
        ],
        '1' => [
            "00100", "01100", "00100", "00100", "00100", "00100", "01110",
        ],
        '2' => [
            "01110", "10001", "00001", "00010", "00100", "01000", "11111",
        ],
        '3' => [
            "11110", "00001", "00001", "01110", "00001", "00001", "11110",
        ],
        '4' => [
            "00010", "00110", "01010", "10010", "11111", "00010", "00010",
        ],
        '5' => [
            "11111", "10000", "10000", "11110", "00001", "00001", "11110",
        ],
        '6' => [
            "01110", "10000", "10000", "11110", "10001", "10001", "01110",
        ],
        '7' => [
            "11111", "00001", "00010", "00100", "01000", "01000", "01000",
        ],
        '8' => [
            "01110", "10001", "10001", "01110", "10001", "10001", "01110",
        ],
        '9' => [
            "01110", "10001", "10001", "01111", "00001", "00001", "01110",
        ],
        '&' => [
            "01100", "10010", "10100", "01000", "10101", "10010", "01101",
        ],
        ':' => [
            "00000", "00100", "00100", "00000", "00100", "00100", "00000",
        ],
        '-' => [
            "00000", "00000", "00000", "11111", "00000", "00000", "00000",
        ],
        '_' => [
            "00000", "00000", "00000", "00000", "00000", "00000", "11111",
        ],
        '/' => [
            "00001", "00010", "00010", "00100", "01000", "01000", "10000",
        ],
        '.' => [
            "00000", "00000", "00000", "00000", "00000", "01100", "01100",
        ],
        ',' => [
            "00000", "00000", "00000", "00000", "00100", "00100", "01000",
        ],
        '\'' => [
            "00100", "00100", "01000", "00000", "00000", "00000", "00000",
        ],
        '(' => [
            "00010", "00100", "01000", "01000", "01000", "00100", "00010",
        ],
        ')' => [
            "01000", "00100", "00010", "00010", "00010", "00100", "01000",
        ],
        ' ' => [
            "00000", "00000", "00000", "00000", "00000", "00000", "00000",
        ],
        _ => [
            "11111", "00001", "00010", "00100", "00100", "00000", "00100",
        ],
    }
}

fn build_crosshair_mesh(width: u32, height: u32) -> (Vec<Vertex>, Vec<u32>) {
    let color = [0.96, 0.96, 0.96];
    let aspect = width.max(1) as f32 / height.max(1) as f32;
    let thickness = 0.0035;
    let vertical_length = 0.032;
    let horizontal_length = vertical_length / aspect;
    let vertices = vec![
        Vertex {
            position: [-horizontal_length, -thickness, 0.0],
            color,
            tex_coords: [0.0, 0.0],
        },
        Vertex {
            position: [horizontal_length, -thickness, 0.0],
            color,
            tex_coords: [0.0, 0.0],
        },
        Vertex {
            position: [horizontal_length, thickness, 0.0],
            color,
            tex_coords: [0.0, 0.0],
        },
        Vertex {
            position: [-horizontal_length, thickness, 0.0],
            color,
            tex_coords: [0.0, 0.0],
        },
        Vertex {
            position: [-thickness, -vertical_length, 0.0],
            color,
            tex_coords: [0.0, 0.0],
        },
        Vertex {
            position: [thickness, -vertical_length, 0.0],
            color,
            tex_coords: [0.0, 0.0],
        },
        Vertex {
            position: [thickness, vertical_length, 0.0],
            color,
            tex_coords: [0.0, 0.0],
        },
        Vertex {
            position: [-thickness, vertical_length, 0.0],
            color,
            tex_coords: [0.0, 0.0],
        },
    ];
    let indices = vec![0, 1, 2, 0, 2, 3, 4, 5, 6, 4, 6, 7];
    (vertices, indices)
}

fn build_outline_vertices(block: WorldBlockPosition) -> Vec<Vertex> {
    let color = [1.0, 0.92, 0.18];
    let min = Vec3::new(
        block.x as f32 - 8.0 - 0.025,
        block.y as f32 - 64.0 - 0.025,
        block.z as f32 - 8.0 - 0.025,
    );
    let max = min + Vec3::splat(1.05);
    let corners = [
        [min.x, min.y, min.z],
        [max.x, min.y, min.z],
        [max.x, max.y, min.z],
        [min.x, max.y, min.z],
        [min.x, min.y, max.z],
        [max.x, min.y, max.z],
        [max.x, max.y, max.z],
        [min.x, max.y, max.z],
    ];
    let edges = [
        (0, 1),
        (1, 2),
        (2, 3),
        (3, 0),
        (4, 5),
        (5, 6),
        (6, 7),
        (7, 4),
        (0, 4),
        (1, 5),
        (2, 6),
        (3, 7),
    ];

    edges
        .into_iter()
        .flat_map(|(a, b)| {
            [
                Vertex {
                    position: corners[a],
                    color,
                    tex_coords: [0.0, 0.0],
                },
                Vertex {
                    position: corners[b],
                    color,
                    tex_coords: [0.0, 0.0],
                },
            ]
        })
        .collect()
}

fn block_color(block: BlockId, blocks: &BlockRegistry) -> [f32; 3] {
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

fn shaded_block_color(
    block: BlockId,
    direction: crate::engine::mesh::chunk_mesher::FaceDirection,
    blocks: &BlockRegistry,
) -> [f32; 3] {
    use crate::engine::mesh::chunk_mesher::FaceDirection;

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

fn render_material(
    block: BlockId,
    direction: crate::engine::mesh::chunk_mesher::FaceDirection,
    blocks: &BlockRegistry,
) -> ([f32; 3], String) {
    use crate::engine::mesh::chunk_mesher::FaceDirection;

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

    let texture_key = texture_key_for_direction(definition, direction).to_string();
    if texture_key == "humancraft:missing" {
        return (shaded_block_color(block, direction, blocks), texture_key);
    }

    ([shade.min(1.0); 3], texture_key)
}

fn texture_key_for_direction(
    definition: &crate::engine::world::BlockDefinition,
    direction: crate::engine::mesh::chunk_mesher::FaceDirection,
) -> &str {
    use crate::engine::mesh::chunk_mesher::FaceDirection;

    match direction {
        FaceDirection::North => &definition.textures.north,
        FaceDirection::South => &definition.textures.south,
        FaceDirection::East => &definition.textures.east,
        FaceDirection::West => &definition.textures.west,
        FaceDirection::Up => &definition.textures.top,
        FaceDirection::Down => &definition.textures.bottom,
    }
}

fn block_texture_keys(definition: &crate::engine::world::BlockDefinition) -> [&str; 6] {
    [
        &definition.textures.top,
        &definition.textures.bottom,
        &definition.textures.north,
        &definition.textures.south,
        &definition.textures.east,
        &definition.textures.west,
    ]
}

fn item_texture_keys(items: &ItemRegistry) -> Vec<&str> {
    items
        .iter()
        .map(|(_, definition)| definition.texture.as_str())
        .collect()
}

fn load_texture_pixels(key: &str) -> Option<Vec<u8>> {
    let path = texture_path(key)?;
    let image = image::open(path).ok()?;
    if image.dimensions() != (TextureAtlas::TILE_SIZE, TextureAtlas::TILE_SIZE) {
        return None;
    }
    Some(image.to_rgba8().into_raw())
}

fn texture_path(key: &str) -> Option<PathBuf> {
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

    let item = key.strip_prefix("humancraft:item/")?;
    Some(
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("textures")
            .join("items")
            .join(format!("{item}.png")),
    )
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

#[cfg(test)]
mod tests {
    use super::spatial::{split_world_block_position, world_block_from_render};
    use super::*;
    use crate::content::{GameContent, bootstrap_content, default_generation_pipeline};
    use crate::engine::mesh::chunk_mesher::{FaceDirection, MeshQuad};
    use crate::engine::world::{
        BlockPosition, CHUNK_SIZE, Chunk, ChunkPosition, ItemId, LootEntity,
    };

    fn test_client_world(content: &GameContent) -> ClientWorld {
        ClientWorld::new(
            content.blocks.clone(),
            content.items.clone(),
            content.block_ids,
            default_generation_pipeline(content.block_ids),
            GenerationContext {
                seed: 1,
                air: content.block_ids.air,
            },
            CLIENT_RENDER_DISTANCE_CHUNKS,
            "test-world".to_string(),
        )
    }

    #[test]
    fn logical_zqsd_controls_drive_movement() {
        let mut input = InputState::default();

        input.handle_logical_key(Key::Character("z"), true);
        input.handle_logical_key(Key::Character("q"), true);

        assert!(input.forward);
        assert!(input.left);
        assert!(!input.backward);
        assert!(!input.right);

        input.handle_logical_key(Key::Character("z"), false);

        assert!(!input.forward);
        assert!(input.left);
    }

    #[test]
    fn wasd_keys_are_not_treated_as_forward_left_on_azerty_bindings() {
        let mut input = InputState::default();

        input.handle_logical_key(Key::Character("w"), true);
        input.handle_logical_key(Key::Character("a"), true);

        assert!(!input.forward);
        assert!(!input.left);
    }

    #[test]
    fn double_tapping_forward_enables_sprint() {
        let mut input = InputState::default();
        let start = Instant::now();

        input.handle_logical_key_at(Key::Character("z"), true, start);
        input.handle_logical_key_at(
            Key::Character("z"),
            false,
            start + std::time::Duration::from_millis(40),
        );
        input.handle_logical_key_at(
            Key::Character("z"),
            true,
            start + std::time::Duration::from_millis(160),
        );

        assert!(input.forward);
        assert!(input.sprint);
    }

    #[test]
    fn slow_forward_tap_does_not_enable_sprint() {
        let mut input = InputState::default();
        let start = Instant::now();

        input.handle_logical_key_at(Key::Character("z"), true, start);
        input.handle_logical_key_at(
            Key::Character("z"),
            false,
            start + std::time::Duration::from_millis(40),
        );
        input.handle_logical_key_at(
            Key::Character("z"),
            true,
            start + std::time::Duration::from_millis(600),
        );

        assert!(input.forward);
        assert!(!input.sprint);
    }

    #[test]
    fn preview_filter_keeps_real_underground_faces_visible() {
        let content = bootstrap_content().unwrap();
        let wall = MeshQuad {
            block: content.block_ids.stone,
            direction: FaceDirection::North,
            vertices: [
                [0.0, 0.0, 0.0],
                [1.0, 0.0, 0.0],
                [1.0, 40.0, 0.0],
                [0.0, 40.0, 0.0],
            ],
        };
        let surface_side = MeshQuad {
            vertices: [
                [0.0, 60.0, 0.0],
                [1.0, 60.0, 0.0],
                [1.0, 70.0, 0.0],
                [0.0, 70.0, 0.0],
            ],
            ..wall.clone()
        };
        let top = MeshQuad {
            direction: FaceDirection::Up,
            ..wall.clone()
        };
        let bottom = MeshQuad {
            direction: FaceDirection::Down,
            ..wall.clone()
        };
        let world_bottom = MeshQuad {
            vertices: [
                [0.0, 0.0, 0.0],
                [1.0, 0.0, 0.0],
                [1.0, 0.0, 1.0],
                [0.0, 0.0, 1.0],
            ],
            ..bottom.clone()
        };

        assert!(should_render_preview_quad(
            &wall,
            ChunkPosition { x: 0, z: 0 },
            None
        ));
        assert!(should_render_preview_quad(
            &surface_side,
            ChunkPosition { x: 0, z: 0 },
            None
        ));
        assert!(should_render_preview_quad(
            &top,
            ChunkPosition { x: 0, z: 0 },
            None
        ));
        assert!(should_render_preview_quad(
            &bottom,
            ChunkPosition { x: 0, z: 0 },
            None
        ));
        assert!(should_render_preview_quad(
            &world_bottom,
            ChunkPosition { x: 0, z: 0 },
            None
        ));
    }

    #[test]
    fn preview_filter_hides_outer_render_boundary_faces() {
        let content = bootstrap_content().unwrap();
        let outer_west_wall = MeshQuad {
            block: content.block_ids.stone,
            direction: FaceDirection::West,
            vertices: [
                [0.0, 60.0, 0.0],
                [0.0, 60.0, 1.0],
                [0.0, 70.0, 1.0],
                [0.0, 70.0, 0.0],
            ],
        };
        let inner_west_wall = MeshQuad {
            vertices: [
                [1.0, 60.0, 0.0],
                [1.0, 60.0, 1.0],
                [1.0, 70.0, 1.0],
                [1.0, 70.0, 0.0],
            ],
            ..outer_west_wall.clone()
        };
        let bounds = Some(RenderChunkBounds {
            min_x: -4,
            max_x: 0,
            min_z: -2,
            max_z: 2,
        });

        assert!(!should_render_preview_quad(
            &outer_west_wall,
            ChunkPosition { x: -4, z: 0 },
            bounds
        ));
        assert!(should_render_preview_quad(
            &inner_west_wall,
            ChunkPosition { x: -4, z: 0 },
            bounds
        ));
    }

    #[test]
    fn preview_filter_hides_loaded_world_bottom_boundary_faces() {
        let content = bootstrap_content().unwrap();
        let bottom = MeshQuad {
            block: content.block_ids.bedrock,
            direction: FaceDirection::Down,
            vertices: [
                [0.0, 0.0, 0.0],
                [1.0, 0.0, 0.0],
                [1.0, 0.0, 1.0],
                [0.0, 0.0, 1.0],
            ],
        };
        let bounds = Some(RenderChunkBounds {
            min_x: 0,
            max_x: 0,
            min_z: 0,
            max_z: 0,
        });

        assert!(!should_render_preview_quad(
            &bottom,
            ChunkPosition { x: 0, z: 0 },
            bounds
        ));
    }

    #[test]
    fn splits_negative_world_positions_into_chunk_and_local_positions() {
        let (chunk, block) = split_world_block_position(WorldBlockPosition {
            x: -1,
            y: 64,
            z: -17,
        })
        .unwrap();

        assert_eq!(chunk, ChunkPosition { x: -1, z: -2 });
        assert_eq!(
            block,
            BlockPosition {
                x: 15,
                y: 64,
                z: 15
            }
        );
    }

    #[test]
    fn render_positions_map_to_chunk_positions() {
        assert_eq!(
            chunk_position_for_render_position(Vec3::new(0.0, 0.0, 0.0)),
            ChunkPosition { x: 0, z: 0 }
        );
        assert_eq!(
            chunk_position_for_render_position(Vec3::new(-9.0, 0.0, -9.0)),
            ChunkPosition { x: -1, z: -1 }
        );
    }

    #[test]
    fn client_world_generates_chunks_around_player_position() {
        let content = bootstrap_content().unwrap();
        let mut world = test_client_world(&content);

        let initial_dirty =
            world.ensure_chunks_around_render_position(Vec3::new(0.0, 0.0, 0.0), usize::MAX);
        assert_eq!(world.chunks.len(), 25);
        assert_eq!(initial_dirty.len(), 25);
        assert!(initial_dirty.contains(&ChunkPosition { x: 0, z: 0 }));
        assert!(
            world
                .ensure_chunks_around_render_position(Vec3::new(0.0, 0.0, 0.0), usize::MAX)
                .is_empty()
        );
        assert_eq!(world.chunks.len(), 25);

        let distant_dirty =
            world.ensure_chunks_around_render_position(Vec3::new(80.0, 0.0, -72.0), usize::MAX);
        assert!(!distant_dirty.is_empty());
        assert!(world.chunks.contains_key(&ChunkPosition { x: 5, z: -4 }));
        assert!(world.chunks.len() > 25);
    }

    #[test]
    fn client_world_respects_chunk_load_budget() {
        let content = bootstrap_content().unwrap();
        let mut world = test_client_world(&content);

        let dirty = world.ensure_chunks_around_render_position(Vec3::new(0.0, 0.0, 0.0), 3);

        assert_eq!(world.chunks.len(), 3);
        assert!(!dirty.is_empty());
        assert!(world.chunks.contains_key(&ChunkPosition { x: 0, z: 0 }));
        assert!(!world.chunks.contains_key(&ChunkPosition { x: -2, z: -2 }));
    }

    #[test]
    fn generated_distant_chunks_are_deterministic() {
        let content = bootstrap_content().unwrap();
        let mut world = test_client_world(&content);
        let position = Vec3::new(112.0, 0.0, -96.0);

        world.ensure_chunks_around_render_position(position, usize::MAX);
        let first_block = world.block(WorldBlockPosition {
            x: 120,
            y: 64,
            z: -88,
        });

        let mut other_world = test_client_world(&content);
        other_world.ensure_chunks_around_render_position(position, usize::MAX);
        let second_block = other_world.block(WorldBlockPosition {
            x: 120,
            y: 64,
            z: -88,
        });

        assert_eq!(first_block, second_block);
    }

    #[test]
    fn saved_chunks_override_generation_when_streaming() {
        let content = bootstrap_content().unwrap();
        let root = std::env::temp_dir().join(format!(
            "humancraft-window-save-test-{}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&root);
        let store = WorldSaveStore::new(&root);
        let metadata = store
            .create_world("Saved Chunk", 1, PlayerSave::new(0.0, 0.0, 0.0, 0.0, 0.0))
            .unwrap();
        let chunk_position = ChunkPosition { x: 0, z: 0 };
        let mut saved_chunk = Chunk::filled(chunk_position, content.block_ids.air);
        saved_chunk
            .set_block(
                BlockPosition { x: 8, y: 64, z: 8 },
                content.block_ids.diamond_ore,
            )
            .unwrap();
        store.save_chunk(&metadata.id, &saved_chunk).unwrap();

        let mut world = ClientWorld::new(
            content.blocks.clone(),
            content.items.clone(),
            content.block_ids,
            default_generation_pipeline(content.block_ids),
            GenerationContext {
                seed: metadata.seed,
                air: content.block_ids.air,
            },
            CLIENT_RENDER_DISTANCE_CHUNKS,
            metadata.id.clone(),
        );
        world.ensure_chunks_around_render_position_with_store(Vec3::ZERO, usize::MAX, &store);

        assert_eq!(
            world.block(WorldBlockPosition { x: 8, y: 64, z: 8 }),
            Some(content.block_ids.diamond_ore)
        );

        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn client_world_can_break_and_place_blocks() {
        let content = bootstrap_content().unwrap();
        let mut world = test_client_world(&content);
        let mut chunk = Chunk::filled(ChunkPosition { x: 0, z: 0 }, content.block_ids.air);
        chunk
            .set_block(BlockPosition { x: 1, y: 1, z: 1 }, content.block_ids.stone)
            .unwrap();
        world.insert_chunk(chunk);
        let position = WorldBlockPosition { x: 1, y: 1, z: 1 };

        assert!(world.is_solid(position));
        assert_eq!(
            world.break_block(position),
            vec![ChunkPosition { x: 0, z: 0 }]
        );
        assert!(!world.is_solid(position));
        assert_eq!(
            world.place_block(position, content.block_ids.dirt),
            vec![ChunkPosition { x: 0, z: 0 }]
        );
        assert!(world.is_solid(position));
    }

    #[test]
    fn breaking_block_spawns_configured_loot() {
        let content = bootstrap_content().unwrap();
        let mut world = test_client_world(&content);
        let mut chunk = Chunk::filled(ChunkPosition { x: 0, z: 0 }, content.block_ids.air);
        chunk
            .set_block(BlockPosition { x: 1, y: 1, z: 1 }, content.block_ids.stone)
            .unwrap();
        world.insert_chunk(chunk);

        let dirty = world.break_block(WorldBlockPosition { x: 1, y: 1, z: 1 });
        let cobblestone = world.items.id_for_key("humancraft:cobblestone").unwrap();

        assert_eq!(dirty, vec![ChunkPosition { x: 0, z: 0 }]);
        assert_eq!(world.loot_entities.len(), 1);
        assert_eq!(world.loot_entities[0].stack, ItemStack::new(cobblestone, 1));
    }

    #[test]
    fn player_pickup_adds_loot_to_inventory() {
        let content = bootstrap_content().unwrap();
        let mut world = test_client_world(&content);
        world.insert_chunk(Chunk::filled(
            ChunkPosition { x: 0, z: 0 },
            content.block_ids.air,
        ));
        let dirt = world.items.id_for_key("humancraft:dirt").unwrap();
        world.loot_entities.push(LootEntity::new(
            ItemStack::new(dirt, 3),
            Vec3::new(0.0, 0.05, 0.0),
        ));

        world.update_loot(Vec3::new(0.0, PLAYER_STANDING_EYE_HEIGHT + 0.05, 0.0), 0.05);

        assert!(world.loot_entities.is_empty());
        assert_eq!(
            world.player_inventory.hotbar_slots()[0],
            Some(ItemStack::new(dirt, 3))
        );
    }

    #[test]
    fn inventory_save_round_trips_registered_item_stacks() {
        let content = bootstrap_content().unwrap();
        let dirt = content.items.id_for_key("humancraft:dirt").unwrap();
        let diamond = content.items.id_for_key("humancraft:diamond").unwrap();
        let mut inventory = Inventory::player();
        inventory.add_stack(ItemStack::new(dirt, 64), &content.items);
        inventory.add_stack(ItemStack::new(diamond, 3), &content.items);

        let saved = inventory_to_save(&inventory, &content.items);
        let restored = inventory_from_save(&saved, &content.items);

        assert_eq!(restored.slots()[0], Some(ItemStack::new(dirt, 64)));
        assert_eq!(restored.slots()[1], Some(ItemStack::new(diamond, 3)));
    }

    #[test]
    fn inventory_left_click_picks_up_merges_and_swaps_stacks() {
        let content = bootstrap_content().unwrap();
        let dirt = content.items.id_for_key("humancraft:dirt").unwrap();
        let stone = content.items.id_for_key("humancraft:stone").unwrap();
        let mut inventory = Inventory::new(3, 1);
        let mut cursor = None;
        inventory.set_slot(0, Some(ItemStack::new(dirt, 10)));
        inventory.set_slot(1, Some(ItemStack::new(dirt, 60)));
        inventory.set_slot(2, Some(ItemStack::new(stone, 4)));

        left_click_inventory_slot(&mut inventory, &mut cursor, 0, &content.items);
        assert_eq!(cursor, Some(ItemStack::new(dirt, 10)));
        assert_eq!(inventory.slot(0), None);

        left_click_inventory_slot(&mut inventory, &mut cursor, 1, &content.items);
        assert_eq!(inventory.slot(1), Some(ItemStack::new(dirt, 64)));
        assert_eq!(cursor, Some(ItemStack::new(dirt, 6)));

        left_click_inventory_slot(&mut inventory, &mut cursor, 2, &content.items);
        assert_eq!(inventory.slot(2), Some(ItemStack::new(dirt, 6)));
        assert_eq!(cursor, Some(ItemStack::new(stone, 4)));
    }

    #[test]
    fn inventory_right_click_splits_and_places_one_item() {
        let content = bootstrap_content().unwrap();
        let dirt = content.items.id_for_key("humancraft:dirt").unwrap();
        let mut inventory = Inventory::new(2, 1);
        let mut cursor = None;
        inventory.set_slot(0, Some(ItemStack::new(dirt, 7)));

        right_click_inventory_slot(&mut inventory, &mut cursor, 0, &content.items);
        assert_eq!(inventory.slot(0), Some(ItemStack::new(dirt, 3)));
        assert_eq!(cursor, Some(ItemStack::new(dirt, 4)));

        right_click_inventory_slot(&mut inventory, &mut cursor, 1, &content.items);
        assert_eq!(inventory.slot(1), Some(ItemStack::new(dirt, 1)));
        assert_eq!(cursor, Some(ItemStack::new(dirt, 3)));
    }

    #[test]
    fn inventory_drag_distributes_and_right_drag_places_one_per_slot() {
        let content = bootstrap_content().unwrap();
        let dirt = content.items.id_for_key("humancraft:dirt").unwrap();
        let mut inventory = Inventory::new(4, 1);
        let mut cursor = Some(ItemStack::new(dirt, 8));

        distribute_carried_stack_evenly(&mut inventory, &mut cursor, &[0, 1, 2], &content.items);
        assert_eq!(cursor, None);
        assert_eq!(inventory.slot(0), Some(ItemStack::new(dirt, 3)));
        assert_eq!(inventory.slot(1), Some(ItemStack::new(dirt, 3)));
        assert_eq!(inventory.slot(2), Some(ItemStack::new(dirt, 2)));

        cursor = Some(ItemStack::new(dirt, 3));
        assert!(place_one_carried_item(
            &mut inventory,
            &mut cursor,
            0,
            &content.items
        ));
        assert!(place_one_carried_item(
            &mut inventory,
            &mut cursor,
            3,
            &content.items
        ));
        assert_eq!(inventory.slot(0), Some(ItemStack::new(dirt, 4)));
        assert_eq!(inventory.slot(3), Some(ItemStack::new(dirt, 1)));
        assert_eq!(cursor, Some(ItemStack::new(dirt, 1)));
    }

    #[test]
    fn selected_hotbar_item_controls_block_placement() {
        let content = bootstrap_content().unwrap();
        let mut world = test_client_world(&content);
        world.insert_chunk(Chunk::filled(
            ChunkPosition { x: 0, z: 0 },
            content.block_ids.air,
        ));
        let dirt = world.items.id_for_key("humancraft:dirt").unwrap();
        let coal = world.items.id_for_key("humancraft:coal").unwrap();
        let player_eye = Vec3::new(0.0, 2.0, 0.0);
        let position = WorldBlockPosition { x: 1, y: 1, z: 1 };

        world
            .player_inventory
            .set_slot(0, Some(ItemStack::new(coal, 1)));
        assert!(
            world
                .place_selected_hotbar_block_for_player(position, 0, player_eye)
                .is_empty()
        );
        assert_eq!(world.block(position), Some(content.block_ids.air));

        world
            .player_inventory
            .set_slot(0, Some(ItemStack::new(dirt, 2)));
        assert_eq!(
            world.place_selected_hotbar_block_for_player(position, 0, player_eye),
            vec![ChunkPosition { x: 0, z: 0 }]
        );
        assert_eq!(world.block(position), Some(content.block_ids.dirt));
        assert_eq!(
            world.player_inventory.slot(0),
            Some(ItemStack::new(dirt, 1))
        );

        let cobblestone = world.items.id_for_key("humancraft:cobblestone").unwrap();
        let cobblestone_position = WorldBlockPosition { x: 2, y: 1, z: 1 };
        world
            .player_inventory
            .set_slot(0, Some(ItemStack::new(cobblestone, 1)));
        assert_eq!(
            world.place_selected_hotbar_block_for_player(cobblestone_position, 0, player_eye),
            vec![ChunkPosition { x: 0, z: 0 }]
        );
        assert_eq!(
            world.block(cobblestone_position),
            Some(content.block_ids.cobblestone)
        );
        assert_eq!(world.player_inventory.slot(0), None);
    }

    #[test]
    fn inventory_slots_are_square_in_screen_pixels() {
        let aspect = 16.0 / 9.0;
        let hotbar = inventory_slot_rect(0, false, aspect);
        let inventory = inventory_slot_rect(0, true, aspect);

        assert!((slot_width(hotbar) * aspect - slot_height(hotbar)).abs() < 0.0001);
        assert!((slot_width(inventory) * aspect - slot_height(inventory)).abs() < 0.0001);
        assert!(slot_height(hotbar) > 0.10);
    }

    #[test]
    fn held_block_overlay_uses_three_visible_faces() {
        let faces = held_block_overlay_faces(16.0 / 9.0);
        let all_faces = [faces.front, faces.right, faces.top];

        for face in all_faces {
            assert!(quad_area(face) > 0.002);
            for [x, y, _] in face {
                assert!((-1.0..=1.0).contains(&x));
                assert!((-1.0..=1.0).contains(&y));
            }
        }
        assert_eq!(faces.front[1], faces.right[0]);
        assert_eq!(faces.front[2], faces.right[3]);
        assert_eq!(faces.front[2], faces.top[1]);
        assert_eq!(faces.front[3], faces.top[0]);
    }

    #[test]
    fn held_block_overlay_is_framed_like_a_first_person_held_block() {
        let faces = held_block_overlay_faces(16.0 / 9.0);
        let all_points = faces
            .front
            .into_iter()
            .chain(faces.right)
            .chain(faces.top)
            .collect::<Vec<_>>();
        let min_x = all_points
            .iter()
            .map(|point| point[0])
            .fold(f32::MAX, f32::min);
        let max_x = all_points
            .iter()
            .map(|point| point[0])
            .fold(f32::MIN, f32::max);
        let min_y = all_points
            .iter()
            .map(|point| point[1])
            .fold(f32::MAX, f32::min);
        let max_y = all_points
            .iter()
            .map(|point| point[1])
            .fold(f32::MIN, f32::max);

        assert!(min_x > 0.55);
        assert!(max_x < 0.90);
        assert!(min_y < -0.90);
        assert!(max_y > -0.45);
    }

    #[test]
    fn player_arm_overlay_uses_three_visible_faces() {
        let faces = player_arm_overlay_faces(16.0 / 9.0);

        for face in faces {
            assert!(quad_area(face.positions) > 0.002);
            for [x, y, _] in face.positions {
                assert!((-1.0..=1.0).contains(&x));
                assert!((-1.0..=1.0).contains(&y));
            }
        }
    }

    #[test]
    fn loot_mesh_stays_above_entity_contact_point_and_rotates_around_y() {
        let loot = LootEntity {
            stack: ItemStack::new(ItemId::from(1), 1),
            position: Vec3::new(0.0, 2.0, 0.0),
            velocity: Vec3::ZERO,
            rotation_radians: std::f32::consts::FRAC_PI_2,
        };
        let corners = loot_billboard_corners(&loot);

        assert!(corners.iter().all(|corner| corner.y >= 2.0));
        assert!(corners.iter().any(|corner| corner.z.abs() > 0.20));
        assert!(corners.iter().all(|corner| corner.x.abs() < 0.001));
    }

    #[test]
    fn held_block_interaction_repeats_after_cadence_until_released() {
        let mut interaction = HeldBlockInteraction::default();

        interaction.press(MouseButton::Right);
        assert_eq!(interaction.repeat_button(0.05), None);
        assert_eq!(interaction.repeat_button(0.09), None);
        assert_eq!(interaction.repeat_button(0.01), Some(MouseButton::Right));
        assert_eq!(interaction.repeat_button(0.01), None);
        interaction.release(MouseButton::Right);
        assert_eq!(
            interaction.repeat_button(BLOCK_INTERACTION_REPEAT_SECONDS),
            None
        );
    }

    #[test]
    fn loot_from_block_below_another_block_spawns_in_open_space_and_falls() {
        let content = bootstrap_content().unwrap();
        let mut world = test_client_world(&content);
        let mut chunk = Chunk::filled(ChunkPosition { x: 0, z: 0 }, content.block_ids.air);
        chunk
            .set_block(BlockPosition { x: 1, y: 1, z: 1 }, content.block_ids.stone)
            .unwrap();
        chunk
            .set_block(BlockPosition { x: 1, y: 2, z: 1 }, content.block_ids.dirt)
            .unwrap();
        world.insert_chunk(chunk);

        world.break_block(WorldBlockPosition { x: 1, y: 1, z: 1 });
        assert_eq!(world.loot_entities.len(), 1);
        assert!(world.loot_entities[0].position.y + LOOT_RENDER_HALF_SIZE * 2.0 < 2.0);

        let player_far_away = Vec3::new(8.0, PLAYER_STANDING_EYE_HEIGHT + 1.0, 8.0);
        for _ in 0..20 {
            world.update_loot(player_far_away, PHYSICS_TICK_SECONDS);
        }

        assert!(world.loot_entities[0].position.y < 1.5);
    }

    fn quad_area(points: [[f32; 3]; 4]) -> f32 {
        let triangles = [[0, 1, 2], [0, 2, 3]];
        triangles
            .iter()
            .map(|[a, b, c]| {
                let a = glam::Vec2::new(points[*a][0], points[*a][1]);
                let b = glam::Vec2::new(points[*b][0], points[*b][1]);
                let c = glam::Vec2::new(points[*c][0], points[*c][1]);
                ((b.x - a.x) * (c.y - a.y) - (b.y - a.y) * (c.x - a.x)).abs() * 0.5
            })
            .sum()
    }

    #[test]
    fn client_world_does_not_break_unbreakable_blocks() {
        let content = bootstrap_content().unwrap();
        let mut world = test_client_world(&content);
        let mut chunk = Chunk::filled(ChunkPosition { x: 0, z: 0 }, content.block_ids.air);
        chunk
            .set_block(
                BlockPosition { x: 1, y: 0, z: 1 },
                content.block_ids.bedrock,
            )
            .unwrap();
        world.insert_chunk(chunk);
        let position = WorldBlockPosition { x: 1, y: 0, z: 1 };

        assert!(world.is_solid(position));
        assert!(world.break_block(position).is_empty());
        assert_eq!(world.block(position), Some(content.block_ids.bedrock));
    }

    #[test]
    fn player_facing_placement_rejects_player_occupied_blocks() {
        let content = bootstrap_content().unwrap();
        let mut world = test_client_world(&content);
        world.insert_chunk(Chunk::filled(
            ChunkPosition { x: 0, z: 0 },
            content.block_ids.air,
        ));
        let player_eye = Vec3::new(0.0, 1.62, 0.0);
        let feet_block = WorldBlockPosition { x: 8, y: 64, z: 8 };
        let head_block = WorldBlockPosition { x: 8, y: 65, z: 8 };
        let nearby_block = WorldBlockPosition { x: 9, y: 64, z: 8 };

        assert!(
            world
                .place_block_for_player(feet_block, content.block_ids.dirt, player_eye)
                .is_empty()
        );
        assert!(
            world
                .place_block_for_player(head_block, content.block_ids.dirt, player_eye)
                .is_empty()
        );
        assert_eq!(world.block(feet_block), Some(content.block_ids.air));
        assert_eq!(world.block(head_block), Some(content.block_ids.air));

        assert_eq!(
            world.place_block_for_player(nearby_block, content.block_ids.dirt, player_eye),
            vec![ChunkPosition { x: 0, z: 0 }]
        );
        assert_eq!(world.block(nearby_block), Some(content.block_ids.dirt));
    }

    #[test]
    fn block_edits_mark_only_affected_chunks_dirty() {
        let content = bootstrap_content().unwrap();
        let mut world = test_client_world(&content);
        let mut center_chunk = Chunk::filled(ChunkPosition { x: 0, z: 0 }, content.block_ids.air);
        center_chunk
            .set_block(BlockPosition { x: 8, y: 1, z: 8 }, content.block_ids.stone)
            .unwrap();
        center_chunk
            .set_block(
                BlockPosition {
                    x: CHUNK_SIZE - 1,
                    y: 1,
                    z: 1,
                },
                content.block_ids.stone,
            )
            .unwrap();
        world.insert_chunk(center_chunk);
        world.insert_chunk(Chunk::filled(
            ChunkPosition { x: 1, z: 0 },
            content.block_ids.air,
        ));

        assert_eq!(
            world.break_block(WorldBlockPosition { x: 8, y: 1, z: 8 }),
            vec![ChunkPosition { x: 0, z: 0 }]
        );

        let dirty = world.break_block(WorldBlockPosition { x: 15, y: 1, z: 1 });
        assert_eq!(dirty.len(), 2);
        assert!(dirty.contains(&ChunkPosition { x: 0, z: 0 }));
        assert!(dirty.contains(&ChunkPosition { x: 1, z: 0 }));
    }

    #[test]
    fn client_world_mesh_culls_faces_across_chunk_boundaries() {
        let content = bootstrap_content().unwrap();
        let mut world = test_client_world(&content);
        let mut west_chunk = Chunk::filled(ChunkPosition { x: 0, z: 0 }, content.block_ids.air);
        let mut east_chunk = Chunk::filled(ChunkPosition { x: 1, z: 0 }, content.block_ids.air);
        west_chunk
            .set_block(
                BlockPosition {
                    x: CHUNK_SIZE - 1,
                    y: 1,
                    z: 1,
                },
                content.block_ids.stone,
            )
            .unwrap();
        east_chunk
            .set_block(BlockPosition { x: 0, y: 1, z: 1 }, content.block_ids.stone)
            .unwrap();
        world.insert_chunk(west_chunk);
        world.insert_chunk(east_chunk);

        let west_mesh = world.mesh_chunk_for_render(
            ChunkPosition { x: 0, z: 0 },
            world.chunks.get(&ChunkPosition { x: 0, z: 0 }).unwrap(),
        );
        let east_mesh = world.mesh_chunk_for_render(
            ChunkPosition { x: 1, z: 0 },
            world.chunks.get(&ChunkPosition { x: 1, z: 0 }).unwrap(),
        );

        assert_eq!(west_mesh.quads.len(), 5);
        assert_eq!(east_mesh.quads.len(), 5);
        assert!(
            !west_mesh
                .quads
                .iter()
                .any(|quad| quad.direction == FaceDirection::East)
        );
        assert!(
            !east_mesh
                .quads
                .iter()
                .any(|quad| quad.direction == FaceDirection::West)
        );
    }

    #[test]
    fn client_world_mesh_exposes_border_face_after_neighbor_breaks() {
        let content = bootstrap_content().unwrap();
        let mut world = test_client_world(&content);
        let mut west_chunk = Chunk::filled(ChunkPosition { x: 0, z: 0 }, content.block_ids.air);
        let mut east_chunk = Chunk::filled(ChunkPosition { x: 1, z: 0 }, content.block_ids.air);
        west_chunk
            .set_block(
                BlockPosition {
                    x: CHUNK_SIZE - 1,
                    y: 1,
                    z: 1,
                },
                content.block_ids.stone,
            )
            .unwrap();
        east_chunk
            .set_block(BlockPosition { x: 0, y: 1, z: 1 }, content.block_ids.stone)
            .unwrap();
        world.insert_chunk(west_chunk);
        world.insert_chunk(east_chunk);

        assert!(
            !world
                .break_block(WorldBlockPosition { x: 16, y: 1, z: 1 })
                .is_empty()
        );
        let west_mesh = world.mesh_chunk_for_render(
            ChunkPosition { x: 0, z: 0 },
            world.chunks.get(&ChunkPosition { x: 0, z: 0 }).unwrap(),
        );

        assert_eq!(west_mesh.quads.len(), 6);
        assert!(
            west_mesh
                .quads
                .iter()
                .any(|quad| quad.direction == FaceDirection::East)
        );
    }

    #[test]
    fn player_aabb_detects_wall_collision_without_surface_snap() {
        let content = bootstrap_content().unwrap();
        let mut world = test_client_world(&content);
        let mut chunk = Chunk::filled(ChunkPosition { x: 0, z: 0 }, content.block_ids.air);
        chunk
            .set_block(BlockPosition { x: 8, y: 64, z: 8 }, content.block_ids.stone)
            .unwrap();
        world.insert_chunk(chunk);

        let colliding_eye = Vec3::new(0.0, 2.0, 0.0);
        let nearby_eye = Vec3::new(1.4, 2.0, 0.0);

        assert!(world.collides_player_at(colliding_eye));
        assert!(!world.collides_player_at(nearby_eye));
    }

    #[test]
    fn safe_spawn_position_does_not_collide_with_blocks() {
        let content = bootstrap_content().unwrap();
        let mut world = test_client_world(&content);
        let mut chunk = Chunk::filled(ChunkPosition { x: 0, z: 0 }, content.block_ids.air);
        for y in 0..=64 {
            chunk
                .set_block(BlockPosition { x: 8, y, z: 8 }, content.block_ids.stone)
                .unwrap();
        }
        chunk
            .set_block(BlockPosition { x: 8, y: 65, z: 8 }, content.block_ids.stone)
            .unwrap();
        world.insert_chunk(chunk);

        let spawn = world.safe_spawn_eye_position(Vec3::new(0.0, 0.0, 0.0));

        assert!(!world.collides_player_at(spawn));
        assert_ne!(
            world_block_from_render(spawn),
            WorldBlockPosition { x: 8, y: 65, z: 8 }
        );
    }

    #[test]
    fn sneaking_prevents_player_from_walking_off_block_edge() {
        let content = bootstrap_content().unwrap();
        let mut world = test_client_world(&content);
        let mut chunk = Chunk::filled(ChunkPosition { x: 0, z: 0 }, content.block_ids.air);
        chunk
            .set_block(BlockPosition { x: 8, y: 63, z: 8 }, content.block_ids.stone)
            .unwrap();
        world.insert_chunk(chunk);

        let mut camera = Camera::new(Vec3::new(0.5, PLAYER_STANDING_EYE_HEIGHT + 0.05, 0.5));
        camera.grounded = true;
        let input = InputState {
            forward: true,
            sneak: true,
            ..InputState::default()
        };

        for _ in 0..80 {
            camera.update(&input, &world, PHYSICS_TICK_SECONDS);
        }

        assert!(world.has_player_ground_support(camera.position, camera.eye_height()));
    }

    #[test]
    fn player_can_jump_onto_one_block_ledge() {
        let content = bootstrap_content().unwrap();
        let mut world = test_client_world(&content);
        let mut chunk = Chunk::filled(ChunkPosition { x: 0, z: 0 }, content.block_ids.air);
        for z in 8..=12 {
            chunk
                .set_block(BlockPosition { x: 8, y: 63, z }, content.block_ids.stone)
                .unwrap();
        }
        chunk
            .set_block(BlockPosition { x: 8, y: 64, z: 8 }, content.block_ids.stone)
            .unwrap();
        world.insert_chunk(chunk);

        let mut camera = Camera::new(Vec3::new(0.5, PLAYER_STANDING_EYE_HEIGHT + 0.05, 4.0));
        camera.grounded = true;
        let mut landed_on_ledge = false;
        for _ in 0..100 {
            let input = InputState {
                forward: true,
                jump: camera.position.z <= 2.2,
                ..InputState::default()
            };
            camera.update(&input, &world, PHYSICS_TICK_SECONDS);
            let feet_y = camera.position.y - camera.eye_height();
            if camera.grounded && feet_y >= 1.0 && camera.position.z < 1.0 {
                landed_on_ledge = true;
                break;
            }
        }

        assert!(
            landed_on_ledge,
            "expected jump and forward movement from a short distance to land on one-block ledge"
        );
    }

    #[test]
    fn outline_has_twelve_edges() {
        let vertices = build_outline_vertices(WorldBlockPosition { x: 8, y: 64, z: 8 });

        assert_eq!(vertices.len(), 24);
    }

    #[test]
    fn camera_can_look_nearly_straight_up_and_down() {
        let mut camera = Camera::new(Vec3::ZERO);

        camera.apply_mouse_delta(0.0, -10_000.0);
        assert!(camera.forward().y > 0.99);

        camera.apply_mouse_delta(0.0, 20_000.0);
        assert!(camera.forward().y < -0.99);
    }

    #[test]
    fn crosshair_compensates_for_widescreen_aspect() {
        let (vertices, _) = build_crosshair_mesh(1280, 720);
        let horizontal_length = vertices[1].position[0] - vertices[0].position[0];
        let vertical_length = vertices[6].position[1] - vertices[5].position[1];
        let aspect = 1280.0 / 720.0;

        assert!((horizontal_length * aspect - vertical_length).abs() < 0.001);
    }

    #[test]
    fn raycast_returns_hit_and_previous_empty_block() {
        let content = bootstrap_content().unwrap();
        let mut world = test_client_world(&content);
        let mut chunk = Chunk::filled(ChunkPosition { x: 0, z: 0 }, content.block_ids.air);
        chunk
            .set_block(BlockPosition { x: 4, y: 65, z: 4 }, content.block_ids.stone)
            .unwrap();
        world.insert_chunk(chunk);

        let hit = world
            .raycast(Vec3::new(-4.0, 1.5, -5.0), Vec3::new(0.0, 0.0, 1.0))
            .unwrap();

        assert_eq!(hit.block, WorldBlockPosition { x: 4, y: 65, z: 4 });
        assert_eq!(hit.previous, WorldBlockPosition { x: 4, y: 65, z: 3 });
    }

    #[test]
    fn block_texture_keys_map_to_asset_paths() {
        let content = bootstrap_content().unwrap();
        let (_, grass) = content
            .blocks
            .get_by_key("humancraft:grass")
            .expect("grass should be registered");
        let (_, stone) = content
            .blocks
            .get_by_key("humancraft:stone")
            .expect("stone should be registered");
        let (_, sand) = content
            .blocks
            .get_by_key("humancraft:sand")
            .expect("sand should be registered");
        let (_, sandstone) = content
            .blocks
            .get_by_key("humancraft:sandstone")
            .expect("sandstone should be registered");
        let (_, bedrock) = content
            .blocks
            .get_by_key("humancraft:bedrock")
            .expect("bedrock should be registered");

        assert_eq!(
            texture_key_for_direction(grass, FaceDirection::Up),
            "humancraft:block/grass/top"
        );
        assert_eq!(
            texture_key_for_direction(grass, FaceDirection::Down),
            "humancraft:block/dirt/bottom"
        );
        assert_eq!(
            texture_path(texture_key_for_direction(stone, FaceDirection::North)).unwrap(),
            PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                .join("textures")
                .join("blocks")
                .join("stone")
                .join("top.png")
        );
        assert!(
            load_texture_pixels(texture_key_for_direction(stone, FaceDirection::North)).is_some()
        );
        assert!(load_texture_pixels(texture_key_for_direction(sand, FaceDirection::Up)).is_some());
        assert!(
            load_texture_pixels(texture_key_for_direction(sandstone, FaceDirection::Up)).is_some()
        );
        assert!(
            load_texture_pixels(texture_key_for_direction(bedrock, FaceDirection::Up)).is_some()
        );
    }

    #[test]
    fn every_registered_non_air_block_uses_loadable_textures() {
        let content = bootstrap_content().unwrap();

        for (_, definition) in content.blocks.iter() {
            if definition.key == "humancraft:air" {
                continue;
            }

            for key in block_texture_keys(definition) {
                assert_ne!(
                    key, "humancraft:missing",
                    "{} should not use the missing texture fallback",
                    definition.key
                );
                assert!(
                    load_texture_pixels(key).is_some(),
                    "{} references an invalid texture key {key}",
                    definition.key
                );
            }
        }
    }

    #[test]
    fn every_registered_item_uses_loadable_texture() {
        let content = bootstrap_content().unwrap();

        for (_, definition) in content.items.iter() {
            assert_ne!(
                definition.texture, "humancraft:missing",
                "{} should not use the missing texture fallback",
                definition.key
            );
            assert!(
                load_texture_pixels(&definition.texture).is_some(),
                "{} references an invalid texture key {}",
                definition.key,
                definition.texture
            );
        }
    }

    #[test]
    fn oak_textures_load_and_leaves_have_cutout_alpha() {
        let content = bootstrap_content().unwrap();
        let (_, log) = content
            .blocks
            .get_by_key("humancraft:oak_log")
            .expect("oak log should be registered");
        let (_, leaves) = content
            .blocks
            .get_by_key("humancraft:oak_leaves")
            .expect("oak leaves should be registered");

        assert!(load_texture_pixels(texture_key_for_direction(log, FaceDirection::Up)).is_some());

        let pixels = load_texture_pixels(texture_key_for_direction(leaves, FaceDirection::North))
            .expect("oak leaves texture should load");
        let transparent_pixels = pixels.chunks_exact(4).filter(|pixel| pixel[3] < 32).count();

        assert!(transparent_pixels > 72);
    }
}
