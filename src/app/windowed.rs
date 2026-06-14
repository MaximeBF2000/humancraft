//! Native windowed client using winit and wgpu.

use std::collections::{HashMap, HashSet};
#[cfg(test)]
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Instant;

use glam::Vec3;
use wgpu::util::DeviceExt;
use winit::application::ApplicationHandler;
use winit::dpi::{PhysicalPosition, PhysicalSize};
use winit::event::{DeviceEvent, ElementState, Ime, KeyEvent, MouseButton, WindowEvent};
use winit::event_loop::{ActiveEventLoop, ControlFlow, EventLoop};
use winit::keyboard::{Key, NamedKey};
use winit::window::{CursorGrabMode, Window, WindowAttributes, WindowId};

use crate::content::{bootstrap_content, default_generation_pipeline};
#[cfg(test)]
use crate::engine::world::Inventory;
use crate::engine::world::generation::GenerationContext;
use crate::engine::world::save::{
    PlayerSave, WorldMetadata, WorldSaveError, WorldSaveStore, default_world_name, new_world_seed,
};
use crate::engine::world::{ChunkPosition, ItemStack};

mod app_input;
mod block_break_overlay;
mod camera;
mod client_world;
mod constants;
mod frame;
mod hud;
mod input;
mod inventory_interaction;
mod inventory_ui;
mod loot;
mod player_collision;
mod render_types;
mod session;
mod shaders;
mod spatial;
mod texture;
mod ui;
mod ui_builder;
mod world_lifecycle;
mod world_render;

use block_break_overlay::build_block_break_overlay_mesh;
use camera::Camera;
use client_world::ClientWorld;
use constants::*;
use hud::{build_crosshair_mesh, build_outline_vertices};
use input::InputState;
use inventory_interaction::{
    InventoryDrag, InventoryMouseButton, distribute_carried_stack_evenly, inventory_from_save,
    inventory_to_save, left_click_inventory_slot, place_one_carried_item,
    right_click_inventory_slot,
};
use inventory_ui::{
    build_gameplay_ui_mesh, build_inventory_icon_mesh, build_loot_mesh, inventory_slot_at_point,
};
#[cfg(test)]
use inventory_ui::{
    held_block_overlay_faces, inventory_slot_rect, loot_billboard_corners,
    player_arm_overlay_faces, slot_height, slot_width,
};
use render_types::{CameraUniform, Vertex};
use session::{AppMode, ConfigField, HeldBlockInteraction, NewWorldConfig, TextEntry};
use spatial::{WorldBlockPosition, chunk_position_for_render_position};
use texture::{Texture, TextureAtlas};
#[cfg(test)]
use texture::{block_texture_keys, load_texture_pixels, texture_key_for_direction, texture_path};
use ui::{
    UI_CONFIG_BACK, UI_CONFIG_CREATE, UI_CONFIG_NAME_FIELD, UI_CONFIG_SEED_FIELD, UI_MAIN_PLAY,
    UI_PAUSE_KEEP_PLAYING, UI_PAUSE_SAVE_QUIT, UI_RENAME_BACK, UI_RENAME_SAVE, UI_WORLDS_BACK,
    UI_WORLDS_DELETE, UI_WORLDS_NEW, UI_WORLDS_PLAY, UI_WORLDS_RENAME, build_menu_mesh,
    cursor_to_ui_point, world_list_hit_index,
};
use world_render::{ChunkRenderBuffer, build_chunk_render_buffers, unique_loaded_chunk_positions};
#[cfg(test)]
use world_render::{RenderChunkBounds, should_render_preview_quad};

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
    solid_world_overlay_pipeline: wgpu::RenderPipeline,
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
    block_break_vertex_buffer: wgpu::Buffer,
    block_break_index_buffer: wgpu::Buffer,
    block_break_index_count: u32,
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
            source: wgpu::ShaderSource::Wgsl(shaders::WORLD_TEXTURED.into()),
        });
        let line_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("HumanCraft Line Shader"),
            source: wgpu::ShaderSource::Wgsl(shaders::WORLD_SOLID.into()),
        });
        let ui_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("HumanCraft UI Shader"),
            source: wgpu::ShaderSource::Wgsl(shaders::UI_SOLID.into()),
        });
        let textured_ui_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("HumanCraft Textured UI Shader"),
            source: wgpu::ShaderSource::Wgsl(shaders::UI_TEXTURED.into()),
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
        let solid_world_overlay_pipeline =
            device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: Some("HumanCraft Solid World Overlay Pipeline"),
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
                    topology: wgpu::PrimitiveTopology::TriangleList,
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
        let block_break_vertex_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Block Break Overlay Vertex Buffer"),
            size: (std::mem::size_of::<Vertex>() * 2400) as u64,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let block_break_index_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Block Break Overlay Index Buffer"),
            size: (std::mem::size_of::<u32>() * 3600) as u64,
            usage: wgpu::BufferUsages::INDEX | wgpu::BufferUsages::COPY_DST,
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
            solid_world_overlay_pipeline,
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
            block_break_vertex_buffer,
            block_break_index_buffer,
            block_break_index_count: 0,
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

#[cfg(test)]
mod tests;
