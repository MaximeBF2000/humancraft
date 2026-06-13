//! Native windowed client using winit and wgpu.

use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Instant;

use glam::{Mat4, Vec3};
use image::GenericImageView;
use wgpu::util::DeviceExt;
use winit::application::ApplicationHandler;
use winit::dpi::PhysicalSize;
use winit::event::{DeviceEvent, ElementState, KeyEvent, MouseButton, WindowEvent};
use winit::event_loop::{ActiveEventLoop, ControlFlow, EventLoop};
use winit::keyboard::{Key, KeyCode, NamedKey, PhysicalKey};
use winit::window::{CursorGrabMode, Window, WindowAttributes, WindowId};

use crate::content::{BlockIds, bootstrap_content, default_generation_pipeline};
use crate::engine::mesh::chunk_mesher::{ChunkMesh, ChunkMesher};
use crate::engine::world::generation::{GenerationContext, GenerationPipeline};
use crate::engine::world::{
    BlockId, BlockPosition, BlockRegistry, CHUNK_HEIGHT, CHUNK_SIZE, Chunk, ChunkPosition,
};

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

const CLIENT_RENDER_DISTANCE_CHUNKS: i32 = 2;
const MAX_CHUNK_LOADS_PER_FRAME: usize = 2;
const MAX_CHUNK_REMESHES_PER_FRAME: usize = 3;
const CHUNK_WORLD_SIZE: f32 = 16.0;
const PLAYER_HEIGHT: f32 = 1.8;
const PLAYER_STANDING_EYE_HEIGHT: f32 = 1.62;
const PLAYER_SNEAKING_EYE_HEIGHT: f32 = 1.54;
const PLAYER_RADIUS: f32 = 0.3;
const PHYSICS_TICK_SECONDS: f32 = 0.05;
const WALK_ACCELERATION: f32 = 0.13;
const AIR_ACCELERATION: f32 = 0.03;
const GROUND_FRICTION: f32 = 0.546;
const AIR_HORIZONTAL_DRAG: f32 = 0.91;
const SPRINT_MULTIPLIER: f32 = 1.3;
const SNEAK_MULTIPLIER: f32 = 0.3;
const JUMP_VELOCITY: f32 = 0.46;
const SPRINT_JUMP_BOOST: f32 = 0.2;
const GRAVITY_PER_TICK: f32 = 0.08;
const AIR_DRAG: f32 = 0.98;
const STEP_HEIGHT: f32 = 0.6;
const SNEAK_EDGE_PROBE_DEPTH: f32 = 0.08;
const SPRINT_DOUBLE_TAP_SECONDS: f32 = 0.3;
const NORMAL_FOV_DEGREES: f32 = 70.0;
const SPRINT_FOV_DEGREES: f32 = 78.0;

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
            WindowEvent::CloseRequested => event_loop.exit(),
            WindowEvent::Resized(size) => state.resize(size),
            WindowEvent::KeyboardInput { event, .. } => {
                if state.handle_key(&event) {
                    return;
                }
            }
            WindowEvent::MouseInput {
                state: ElementState::Pressed,
                button,
                ..
            } => state.handle_mouse_button(button),
            WindowEvent::Focused(false) => state.set_paused(true),
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
    line_pipeline: wgpu::RenderPipeline,
    texture_atlas: TextureAtlas,
    texture_bind_group: wgpu::BindGroup,
    camera_buffer: wgpu::Buffer,
    camera_bind_group: wgpu::BindGroup,
    chunk_buffers: HashMap<ChunkPosition, ChunkRenderBuffer>,
    pending_chunk_remeshes: HashSet<ChunkPosition>,
    menu_vertex_buffer: wgpu::Buffer,
    menu_index_buffer: wgpu::Buffer,
    menu_index_count: u32,
    crosshair_vertex_buffer: wgpu::Buffer,
    crosshair_index_buffer: wgpu::Buffer,
    crosshair_index_count: u32,
    outline_vertex_buffer: wgpu::Buffer,
    outline_vertex_count: u32,
    depth_texture: Texture,
    camera: Camera,
    world: ClientWorld,
    targeted_block: Option<WorldBlockPosition>,
    input: InputState,
    paused: bool,
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
        let pipeline = default_generation_pipeline(content.block_ids);
        let generation_context = GenerationContext {
            seed: 1,
            air: content.block_ids.air,
        };
        let mut world = ClientWorld::new(
            content.blocks.clone(),
            content.block_ids,
            pipeline,
            generation_context,
            CLIENT_RENDER_DISTANCE_CHUNKS,
        );
        let preferred_spawn = Vec3::new(0.0, 0.0, 20.0);
        let generated_chunks =
            world.ensure_chunks_around_render_position(preferred_spawn, usize::MAX);
        let texture_atlas = TextureAtlas::load(&device, &queue, &content.blocks);

        let camera = Camera::new(world.safe_spawn_eye_position(preferred_spawn));
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
        let chunk_buffers =
            build_chunk_render_buffers(&device, &world, &texture_atlas, &generated_chunks);
        let (menu_vertices, menu_indices) = build_menu_mesh();
        let menu_vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Menu Vertex Buffer"),
            contents: bytemuck::cast_slice(&menu_vertices),
            usage: wgpu::BufferUsages::VERTEX,
        });
        let menu_index_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Menu Index Buffer"),
            contents: bytemuck::cast_slice(&menu_indices),
            usage: wgpu::BufferUsages::INDEX,
        });
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

        capture_cursor(&window);

        Self {
            window,
            surface,
            device,
            queue,
            config,
            size,
            render_pipeline,
            ui_pipeline,
            line_pipeline,
            texture_atlas,
            texture_bind_group,
            camera_buffer,
            camera_bind_group,
            chunk_buffers,
            pending_chunk_remeshes: HashSet::new(),
            menu_vertex_buffer,
            menu_index_buffer,
            menu_index_count: menu_indices.len() as u32,
            crosshair_vertex_buffer,
            crosshair_index_buffer,
            crosshair_index_count: crosshair_indices.len() as u32,
            outline_vertex_buffer,
            outline_vertex_count: 0,
            depth_texture,
            camera,
            world,
            targeted_block: None,
            input: InputState::default(),
            paused: false,
            last_frame: Instant::now(),
        }
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
        if event.state == ElementState::Pressed
            && matches!(event.logical_key.as_ref(), Key::Named(NamedKey::Escape))
        {
            self.set_paused(!self.paused);
            return true;
        }

        if !self.paused {
            self.input.handle_key(event);
        }
        true
    }

    fn handle_mouse_motion(&mut self, delta_x: f32, delta_y: f32) {
        if !self.paused {
            self.camera.apply_mouse_delta(delta_x, delta_y);
        }
    }

    fn handle_mouse_button(&mut self, button: MouseButton) {
        if self.paused {
            return;
        }

        let Some(hit) = self
            .world
            .raycast(self.camera.position, self.camera.forward())
        else {
            return;
        };

        let dirty_chunks = match button {
            MouseButton::Left => self.world.break_block(hit.block),
            MouseButton::Right => self.world.place_block_for_player(
                hit.previous,
                self.world.block_ids.dirt,
                self.camera.position,
            ),
            _ => Vec::new(),
        };

        if !dirty_chunks.is_empty() {
            self.rebuild_chunk_meshes(&dirty_chunks);
        }
    }

    fn set_paused(&mut self, paused: bool) {
        self.paused = paused;
        self.input.clear_movement();
        if paused {
            release_cursor(&self.window);
            self.window
                .set_title("HumanCraft - Paused (Esc to resume, close window to quit)");
        } else {
            capture_cursor(&self.window);
            self.window.set_title("HumanCraft");
        }
    }

    fn update(&mut self) {
        let now = Instant::now();
        let delta_seconds = now.duration_since(self.last_frame).as_secs_f32().min(0.1);
        self.last_frame = now;
        let mut dirty_chunks = Vec::new();
        if !self.paused {
            self.camera.update(&self.input, &self.world, delta_seconds);
            dirty_chunks.extend(self.world.ensure_chunks_around_render_position(
                self.camera.position,
                MAX_CHUNK_LOADS_PER_FRAME,
            ));
        }
        if !dirty_chunks.is_empty() {
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

    fn render(&mut self) -> Result<(), wgpu::SurfaceError> {
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

            if self.outline_vertex_count > 0 {
                pass.set_pipeline(&self.line_pipeline);
                pass.set_bind_group(0, &self.camera_bind_group, &[]);
                pass.set_vertex_buffer(0, self.outline_vertex_buffer.slice(..));
                pass.draw(0..self.outline_vertex_count, 0..1);
            }

            pass.set_pipeline(&self.ui_pipeline);
            pass.set_vertex_buffer(0, self.crosshair_vertex_buffer.slice(..));
            pass.set_index_buffer(
                self.crosshair_index_buffer.slice(..),
                wgpu::IndexFormat::Uint32,
            );
            pass.draw_indexed(0..self.crosshair_index_count, 0, 0..1);

            if self.paused {
                pass.set_vertex_buffer(0, self.menu_vertex_buffer.slice(..));
                pass.set_index_buffer(self.menu_index_buffer.slice(..), wgpu::IndexFormat::Uint32);
                pass.draw_indexed(0..self.menu_index_count, 0, 0..1);
            }
        }

        self.queue.submit(Some(encoder.finish()));
        frame.present();
        Ok(())
    }

    fn rebuild_chunk_meshes(&mut self, dirty_chunks: &[ChunkPosition]) {
        for chunk_position in unique_loaded_chunk_positions(dirty_chunks, &self.world) {
            let Some((vertices, indices)) = self
                .world
                .build_chunk_render_mesh(chunk_position, &self.texture_atlas)
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
        for chunk_position in unique_loaded_chunk_positions(dirty_chunks, &self.world) {
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
        self.targeted_block = if self.paused {
            None
        } else {
            self.world
                .raycast(self.camera.position, self.camera.forward())
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

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
struct RaycastHit {
    block: WorldBlockPosition,
    previous: WorldBlockPosition,
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
struct WorldBlockPosition {
    x: i32,
    y: i32,
    z: i32,
}

struct ClientWorld {
    chunks: HashMap<ChunkPosition, Chunk>,
    blocks: BlockRegistry,
    block_ids: BlockIds,
    generation_pipeline: GenerationPipeline,
    generation_context: GenerationContext,
    render_distance_chunks: i32,
    mesher: ChunkMesher,
}

impl ClientWorld {
    fn new(
        blocks: BlockRegistry,
        block_ids: BlockIds,
        generation_pipeline: GenerationPipeline,
        generation_context: GenerationContext,
        render_distance_chunks: i32,
    ) -> Self {
        Self {
            chunks: HashMap::new(),
            blocks,
            block_ids,
            generation_pipeline,
            generation_context,
            render_distance_chunks,
            mesher: ChunkMesher,
        }
    }

    fn insert_chunk(&mut self, chunk: Chunk) {
        self.chunks.insert(chunk.position(), chunk);
    }

    fn ensure_chunks_around_render_position(
        &mut self,
        position: Vec3,
        max_new_chunks: usize,
    ) -> Vec<ChunkPosition> {
        let center = chunk_position_for_render_position(position);
        let mut dirty_chunks = HashSet::new();
        let mut missing_chunks = Vec::new();

        for z in center.z - self.render_distance_chunks..=center.z + self.render_distance_chunks {
            for x in center.x - self.render_distance_chunks..=center.x + self.render_distance_chunks
            {
                let chunk_position = ChunkPosition { x, z };
                if self.chunks.contains_key(&chunk_position) {
                    continue;
                }
                missing_chunks.push(chunk_position);
            }
        }

        missing_chunks.sort_by_key(|chunk| {
            let dx = (chunk.x - center.x).abs();
            let dz = (chunk.z - center.z).abs();
            (dx.max(dz), dx + dz, chunk.z, chunk.x)
        });

        for chunk_position in missing_chunks.into_iter().take(max_new_chunks) {
            let chunk = self
                .generation_pipeline
                .generate_chunk(chunk_position, &self.generation_context);
            self.insert_chunk(chunk);
            self.mark_chunk_and_horizontal_neighbors_dirty(chunk_position, &mut dirty_chunks);
        }

        dirty_chunks.into_iter().collect()
    }

    fn safe_spawn_eye_position(&self, preferred_position: Vec3) -> Vec3 {
        for radius in 0_i32..=8 {
            for dz in -radius..=radius {
                for dx in -radius..=radius {
                    if dx.abs() != radius && dz.abs() != radius {
                        continue;
                    }

                    let candidate_x = preferred_position.x + dx as f32;
                    let candidate_z = preferred_position.z + dz as f32;
                    let Some(eye_y) = self.ground_eye_y(candidate_x, candidate_z) else {
                        continue;
                    };
                    let candidate = Vec3::new(candidate_x, eye_y, candidate_z);
                    if !self.collides_player_at(candidate) {
                        return candidate;
                    }
                }
            }
        }

        let fallback = Vec3::new(
            preferred_position.x,
            preferred_position.y + 96.0,
            preferred_position.z,
        );
        self.first_clear_eye_position_above(fallback)
            .unwrap_or(fallback)
    }

    fn build_chunk_render_mesh(
        &self,
        chunk_position: ChunkPosition,
        texture_atlas: &TextureAtlas,
    ) -> Option<(Vec<Vertex>, Vec<u32>)> {
        let chunk = self.chunks.get(&chunk_position)?;
        let mesh = self.mesh_chunk_for_render(chunk_position, chunk);
        Some(build_render_mesh(
            &[(chunk_position, mesh)],
            &self.blocks,
            texture_atlas,
            RenderChunkBounds::from_chunk_positions(self.chunks.keys().copied()),
        ))
    }

    fn mesh_chunk_for_render(&self, chunk_position: ChunkPosition, chunk: &Chunk) -> ChunkMesh {
        self.mesher
            .mesh_chunk_with_neighbor_lookup(chunk, &self.blocks, |position, direction| {
                let world_position =
                    world_block_position_from_chunk_position(chunk_position, position);
                self.block(neighbor_world_block_position(world_position, direction))
            })
    }

    fn ground_eye_y(&self, render_x: f32, render_z: f32) -> Option<f32> {
        self.surface_block_y_at(render_x, render_z)
            .and_then(|block_y| {
                self.first_clear_eye_position_above_world_y(render_x, render_z, block_y + 1)
            })
    }

    fn first_clear_eye_position_above(&self, start: Vec3) -> Option<Vec3> {
        let world_start_y = render_y_to_block_world(start.y).floor() as i32;
        self.first_clear_eye_position_above_world_y(start.x, start.z, world_start_y)
            .map(|eye_y| Vec3::new(start.x, eye_y, start.z))
    }

    fn first_clear_eye_position_above_world_y(
        &self,
        render_x: f32,
        render_z: f32,
        world_start_y: i32,
    ) -> Option<f32> {
        for feet_y in world_start_y.max(0)..CHUNK_HEIGHT as i32 {
            let eye_y = feet_y as f32 - 64.0 + PLAYER_STANDING_EYE_HEIGHT + 0.05;
            if !self.collides_player_at(Vec3::new(render_x, eye_y, render_z)) {
                return Some(eye_y);
            }
        }

        None
    }

    fn collides_player_at(&self, eye_position: Vec3) -> bool {
        self.collides_player_at_eye_height(eye_position, PLAYER_STANDING_EYE_HEIGHT)
    }

    fn collides_player_at_eye_height(&self, eye_position: Vec3, eye_height: f32) -> bool {
        let (min, max) = player_aabb_at_eye_height(eye_position, eye_height);
        self.collides_aabb(min, max)
    }

    fn has_player_ground_support(&self, eye_position: Vec3, eye_height: f32) -> bool {
        let (min, max) = player_aabb_at_eye_height(eye_position, eye_height);
        let probe_min = Vec3::new(min.x, min.y - SNEAK_EDGE_PROBE_DEPTH, min.z);
        let probe_max = Vec3::new(max.x, min.y, max.z);
        self.collides_aabb(probe_min, probe_max)
    }

    fn block_intersects_player(
        &self,
        position: WorldBlockPosition,
        player_eye_position: Vec3,
    ) -> bool {
        let block_min = Vec3::new(
            position.x as f32 - 8.0,
            position.y as f32 - 64.0,
            position.z as f32 - 8.0,
        );
        let block_max = block_min + Vec3::splat(1.0);
        let (player_min, player_max) = player_aabb(player_eye_position);

        aabb_intersects(block_min, block_max, player_min, player_max)
    }

    fn place_block_for_player(
        &mut self,
        position: WorldBlockPosition,
        block: BlockId,
        player_eye_position: Vec3,
    ) -> Vec<ChunkPosition> {
        if self.block_intersects_player(position, player_eye_position) {
            return Vec::new();
        }
        self.place_block(position, block)
    }

    fn collides_aabb(&self, min: Vec3, max: Vec3) -> bool {
        let epsilon = 0.001;
        let min_x = render_x_to_block_world(min.x).floor() as i32;
        let max_x = render_x_to_block_world(max.x - epsilon).floor() as i32;
        let min_y = render_y_to_block_world(min.y).floor() as i32;
        let max_y = render_y_to_block_world(max.y - epsilon).floor() as i32;
        let min_z = render_z_to_block_world(min.z).floor() as i32;
        let max_z = render_z_to_block_world(max.z - epsilon).floor() as i32;

        for y in min_y..=max_y {
            for z in min_z..=max_z {
                for x in min_x..=max_x {
                    if self.is_solid(WorldBlockPosition { x, y, z }) {
                        return true;
                    }
                }
            }
        }

        false
    }

    fn surface_block_y_at(&self, render_x: f32, render_z: f32) -> Option<i32> {
        let world_x = render_x_to_block_world(render_x).floor() as i32;
        let world_z = render_z_to_block_world(render_z).floor() as i32;
        (0..CHUNK_HEIGHT as i32).rev().find(|y| {
            self.is_solid(WorldBlockPosition {
                x: world_x,
                y: *y,
                z: world_z,
            })
        })
    }

    fn raycast(&self, origin: Vec3, direction: Vec3) -> Option<RaycastHit> {
        let mut previous = world_block_from_render(origin);
        let step = 0.08;
        let max_distance = 6.0;
        let steps = (max_distance / step) as usize;

        for index in 1..=steps {
            let sample = origin + direction * (index as f32 * step);
            let current = world_block_from_render(sample);
            if current == previous {
                continue;
            }
            if self.is_solid(current) {
                return Some(RaycastHit {
                    block: current,
                    previous,
                });
            }
            previous = current;
        }

        None
    }

    fn break_block(&mut self, position: WorldBlockPosition) -> Vec<ChunkPosition> {
        if self.is_unbreakable(position) {
            return Vec::new();
        }
        self.set_block(position, self.block_ids.air)
    }

    fn place_block(&mut self, position: WorldBlockPosition, block: BlockId) -> Vec<ChunkPosition> {
        if self.is_solid(position) {
            return Vec::new();
        }
        self.set_block(position, block)
    }

    fn is_solid(&self, position: WorldBlockPosition) -> bool {
        self.block(position)
            .and_then(|block| self.blocks.get(block))
            .map(|definition| definition.solid)
            .unwrap_or(false)
    }

    fn is_unbreakable(&self, position: WorldBlockPosition) -> bool {
        self.block(position)
            .and_then(|block| self.blocks.get(block))
            .map(|definition| definition.has_tag("unbreakable"))
            .unwrap_or(false)
    }

    fn block(&self, position: WorldBlockPosition) -> Option<BlockId> {
        let (chunk_position, block_position) = split_world_block_position(position)?;
        self.chunks
            .get(&chunk_position)
            .and_then(|chunk| chunk.block(block_position))
    }

    fn set_block(&mut self, position: WorldBlockPosition, block: BlockId) -> Vec<ChunkPosition> {
        let Some((chunk_position, block_position)) = split_world_block_position(position) else {
            return Vec::new();
        };
        let Some(chunk) = self.chunks.get_mut(&chunk_position) else {
            return Vec::new();
        };
        let Ok(()) = chunk.set_block(block_position, block) else {
            return Vec::new();
        };

        let mut dirty_chunks = HashSet::new();
        dirty_chunks.insert(chunk_position);
        for neighbor in dirty_horizontal_chunk_positions_for_block(chunk_position, block_position) {
            if self.chunks.contains_key(&neighbor) {
                dirty_chunks.insert(neighbor);
            }
        }
        dirty_chunks.into_iter().collect()
    }

    fn mark_chunk_and_horizontal_neighbors_dirty(
        &self,
        chunk_position: ChunkPosition,
        dirty_chunks: &mut HashSet<ChunkPosition>,
    ) {
        dirty_chunks.insert(chunk_position);
        for neighbor in horizontal_neighbor_chunk_positions(chunk_position) {
            if self.chunks.contains_key(&neighbor) {
                dirty_chunks.insert(neighbor);
            }
        }
    }
}

fn world_block_from_render(position: Vec3) -> WorldBlockPosition {
    WorldBlockPosition {
        x: render_x_to_block_world(position.x).floor() as i32,
        y: render_y_to_block_world(position.y).floor() as i32,
        z: render_z_to_block_world(position.z).floor() as i32,
    }
}

fn chunk_position_for_render_position(position: Vec3) -> ChunkPosition {
    chunk_position_for_world_xz(
        render_x_to_block_world(position.x).floor() as i32,
        render_z_to_block_world(position.z).floor() as i32,
    )
}

fn chunk_position_for_world_xz(x: i32, z: i32) -> ChunkPosition {
    ChunkPosition {
        x: x.div_euclid(CHUNK_SIZE as i32),
        z: z.div_euclid(CHUNK_SIZE as i32),
    }
}

fn world_block_position_from_chunk_position(
    chunk_position: ChunkPosition,
    block_position: BlockPosition,
) -> WorldBlockPosition {
    WorldBlockPosition {
        x: chunk_position.x * CHUNK_SIZE as i32 + block_position.x as i32,
        y: block_position.y as i32,
        z: chunk_position.z * CHUNK_SIZE as i32 + block_position.z as i32,
    }
}

fn neighbor_world_block_position(
    position: WorldBlockPosition,
    direction: crate::engine::mesh::chunk_mesher::FaceDirection,
) -> WorldBlockPosition {
    use crate::engine::mesh::chunk_mesher::FaceDirection;

    match direction {
        FaceDirection::North => WorldBlockPosition {
            z: position.z - 1,
            ..position
        },
        FaceDirection::South => WorldBlockPosition {
            z: position.z + 1,
            ..position
        },
        FaceDirection::East => WorldBlockPosition {
            x: position.x + 1,
            ..position
        },
        FaceDirection::West => WorldBlockPosition {
            x: position.x - 1,
            ..position
        },
        FaceDirection::Up => WorldBlockPosition {
            y: position.y + 1,
            ..position
        },
        FaceDirection::Down => WorldBlockPosition {
            y: position.y - 1,
            ..position
        },
    }
}

fn render_x_to_block_world(render_x: f32) -> f32 {
    render_x + 8.0
}

fn render_y_to_block_world(render_y: f32) -> f32 {
    render_y + 64.0
}

fn render_z_to_block_world(render_z: f32) -> f32 {
    render_z + 8.0
}

fn player_aabb(eye_position: Vec3) -> (Vec3, Vec3) {
    player_aabb_at_eye_height(eye_position, PLAYER_STANDING_EYE_HEIGHT)
}

fn player_aabb_at_eye_height(eye_position: Vec3, eye_height: f32) -> (Vec3, Vec3) {
    (
        Vec3::new(
            eye_position.x - PLAYER_RADIUS,
            eye_position.y - eye_height,
            eye_position.z - PLAYER_RADIUS,
        ),
        Vec3::new(
            eye_position.x + PLAYER_RADIUS,
            eye_position.y - eye_height + PLAYER_HEIGHT,
            eye_position.z + PLAYER_RADIUS,
        ),
    )
}

fn aabb_intersects(left_min: Vec3, left_max: Vec3, right_min: Vec3, right_max: Vec3) -> bool {
    left_min.x < right_max.x
        && left_max.x > right_min.x
        && left_min.y < right_max.y
        && left_max.y > right_min.y
        && left_min.z < right_max.z
        && left_max.z > right_min.z
}

fn split_world_block_position(
    position: WorldBlockPosition,
) -> Option<(ChunkPosition, BlockPosition)> {
    if position.y < 0 || position.y >= CHUNK_HEIGHT as i32 {
        return None;
    }

    let chunk_position = chunk_position_for_world_xz(position.x, position.z);
    let block_position = BlockPosition {
        x: position.x.rem_euclid(CHUNK_SIZE as i32) as usize,
        y: position.y as usize,
        z: position.z.rem_euclid(CHUNK_SIZE as i32) as usize,
    };

    Some((chunk_position, block_position))
}

fn horizontal_neighbor_chunk_positions(chunk_position: ChunkPosition) -> [ChunkPosition; 4] {
    [
        ChunkPosition {
            x: chunk_position.x - 1,
            z: chunk_position.z,
        },
        ChunkPosition {
            x: chunk_position.x + 1,
            z: chunk_position.z,
        },
        ChunkPosition {
            x: chunk_position.x,
            z: chunk_position.z - 1,
        },
        ChunkPosition {
            x: chunk_position.x,
            z: chunk_position.z + 1,
        },
    ]
}

fn dirty_horizontal_chunk_positions_for_block(
    chunk_position: ChunkPosition,
    block_position: BlockPosition,
) -> Vec<ChunkPosition> {
    let mut dirty = Vec::with_capacity(4);
    if block_position.x == 0 {
        dirty.push(ChunkPosition {
            x: chunk_position.x - 1,
            z: chunk_position.z,
        });
    }
    if block_position.x + 1 == CHUNK_SIZE {
        dirty.push(ChunkPosition {
            x: chunk_position.x + 1,
            z: chunk_position.z,
        });
    }
    if block_position.z == 0 {
        dirty.push(ChunkPosition {
            x: chunk_position.x,
            z: chunk_position.z - 1,
        });
    }
    if block_position.z + 1 == CHUNK_SIZE {
        dirty.push(ChunkPosition {
            x: chunk_position.x,
            z: chunk_position.z + 1,
        });
    }
    dirty
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

    fn load(device: &wgpu::Device, queue: &wgpu::Queue, blocks: &BlockRegistry) -> Self {
        let mut texture_keys = vec!["humancraft:missing".to_string()];
        for (_, definition) in blocks.iter() {
            for key in block_texture_keys(definition) {
                if key != "humancraft:missing" && !texture_keys.contains(&key.to_string()) {
                    texture_keys.push(key.to_string());
                }
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
                eprintln!("missing block texture {key}; using fallback texture");
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
            "block texture atlas built: {} loaded, {} fallback",
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

fn build_menu_mesh() -> (Vec<Vertex>, Vec<u32>) {
    let vertices = vec![
        Vertex {
            position: [-0.42, -0.22, 0.0],
            color: [0.05, 0.06, 0.07],
            tex_coords: [0.0, 0.0],
        },
        Vertex {
            position: [0.42, -0.22, 0.0],
            color: [0.05, 0.06, 0.07],
            tex_coords: [0.0, 0.0],
        },
        Vertex {
            position: [0.42, 0.22, 0.0],
            color: [0.05, 0.06, 0.07],
            tex_coords: [0.0, 0.0],
        },
        Vertex {
            position: [-0.42, 0.22, 0.0],
            color: [0.05, 0.06, 0.07],
            tex_coords: [0.0, 0.0],
        },
        Vertex {
            position: [-0.28, -0.03, 0.0],
            color: [0.22, 0.62, 0.18],
            tex_coords: [0.0, 0.0],
        },
        Vertex {
            position: [0.28, -0.03, 0.0],
            color: [0.22, 0.62, 0.18],
            tex_coords: [0.0, 0.0],
        },
        Vertex {
            position: [0.28, 0.07, 0.0],
            color: [0.22, 0.62, 0.18],
            tex_coords: [0.0, 0.0],
        },
        Vertex {
            position: [-0.28, 0.07, 0.0],
            color: [0.22, 0.62, 0.18],
            tex_coords: [0.0, 0.0],
        },
    ];
    let indices = vec![0, 1, 2, 0, 2, 3, 4, 5, 6, 4, 6, 7];
    (vertices, indices)
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

fn load_texture_pixels(key: &str) -> Option<Vec<u8>> {
    let path = texture_path(key)?;
    let image = image::open(path).ok()?;
    if image.dimensions() != (TextureAtlas::TILE_SIZE, TextureAtlas::TILE_SIZE) {
        return None;
    }
    Some(image.to_rgba8().into_raw())
}

fn texture_path(key: &str) -> Option<PathBuf> {
    let path = key.strip_prefix("humancraft:block/")?;
    let (block, face) = path.split_once('/')?;
    Some(
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("textures")
            .join("blocks")
            .join(block)
            .join(format!("{face}.png")),
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
    use super::*;
    use crate::content::{GameContent, bootstrap_content, default_generation_pipeline};
    use crate::engine::mesh::chunk_mesher::{FaceDirection, MeshQuad};

    fn test_client_world(content: &GameContent) -> ClientWorld {
        ClientWorld::new(
            content.blocks.clone(),
            content.block_ids,
            default_generation_pipeline(content.block_ids),
            GenerationContext {
                seed: 1,
                air: content.block_ids.air,
            },
            CLIENT_RENDER_DISTANCE_CHUNKS,
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
