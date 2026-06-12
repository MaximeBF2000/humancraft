//! Native windowed client using winit and wgpu.

use std::collections::HashMap;
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
use crate::engine::world::generation::GenerationContext;
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

const RENDER_MIN_CHUNK: i32 = -2;
const RENDER_MAX_CHUNK: i32 = 2;
const CHUNK_WORLD_SIZE: f32 = 16.0;
const RENDER_MIN_WORLD: f32 = RENDER_MIN_CHUNK as f32 * CHUNK_WORLD_SIZE;
const RENDER_MAX_WORLD: f32 = (RENDER_MAX_CHUNK + 1) as f32 * CHUNK_WORLD_SIZE;
const PLAYER_HEIGHT: f32 = 1.8;
const PLAYER_EYE_HEIGHT: f32 = 1.62;
const PLAYER_RADIUS: f32 = 0.3;

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
    vertex_buffer: wgpu::Buffer,
    index_buffer: wgpu::Buffer,
    index_count: u32,
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
        let mut world = ClientWorld::new(content.blocks.clone(), content.block_ids);
        for chunk_z in RENDER_MIN_CHUNK..=RENDER_MAX_CHUNK {
            for chunk_x in RENDER_MIN_CHUNK..=RENDER_MAX_CHUNK {
                let position = ChunkPosition {
                    x: chunk_x,
                    z: chunk_z,
                };
                let chunk = pipeline.generate_chunk(position, &generation_context);
                world.insert_chunk(chunk);
            }
        }
        let texture_atlas = TextureAtlas::load(&device, &queue, &content.blocks);
        let (vertices, indices) = world.build_render_mesh(&texture_atlas);

        let camera = Camera::new(world.safe_spawn_eye_position(Vec3::new(0.0, 0.0, 20.0)));
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
                cull_mode: None,
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
                depth_compare: wgpu::CompareFunction::LessEqual,
                stencil: wgpu::StencilState::default(),
                bias: wgpu::DepthBiasState::default(),
            }),
            multisample: wgpu::MultisampleState::default(),
            multiview: None,
            cache: None,
        });
        let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Chunk Vertex Buffer"),
            contents: bytemuck::cast_slice(&vertices),
            usage: wgpu::BufferUsages::VERTEX,
        });
        let index_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Chunk Index Buffer"),
            contents: bytemuck::cast_slice(&indices),
            usage: wgpu::BufferUsages::INDEX,
        });
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
        let (crosshair_vertices, crosshair_indices) = build_crosshair_mesh();
        let crosshair_vertex_buffer =
            device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("Crosshair Vertex Buffer"),
                contents: bytemuck::cast_slice(&crosshair_vertices),
                usage: wgpu::BufferUsages::VERTEX,
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
            vertex_buffer,
            index_buffer,
            index_count: indices.len() as u32,
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

        let changed = match button {
            MouseButton::Left => self.world.break_block(hit.block),
            MouseButton::Right => self
                .world
                .place_block(hit.previous, self.world.block_ids.dirt),
            _ => false,
        };

        if changed {
            self.rebuild_world_mesh();
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
        if !self.paused {
            self.camera.update(&self.input, &self.world, delta_seconds);
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
            pass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
            pass.set_index_buffer(self.index_buffer.slice(..), wgpu::IndexFormat::Uint32);
            pass.draw_indexed(0..self.index_count, 0, 0..1);

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

    fn rebuild_world_mesh(&mut self) {
        let (vertices, indices) = self.world.build_render_mesh(&self.texture_atlas);
        self.vertex_buffer = self
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("Chunk Vertex Buffer"),
                contents: bytemuck::cast_slice(&vertices),
                usage: wgpu::BufferUsages::VERTEX,
            });
        self.index_buffer = self
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("Chunk Index Buffer"),
                contents: bytemuck::cast_slice(&indices),
                usage: wgpu::BufferUsages::INDEX,
            });
        self.index_count = indices.len() as u32;
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
}

#[derive(Debug, Copy, Clone)]
struct Camera {
    position: Vec3,
    yaw: f32,
    pitch: f32,
    vertical_velocity: f32,
    grounded: bool,
}

impl Camera {
    fn new(position: Vec3) -> Self {
        Self {
            position,
            yaw: -90.0_f32.to_radians(),
            pitch: -18.0_f32.to_radians(),
            vertical_velocity: 0.0,
            grounded: false,
        }
    }

    fn update(&mut self, input: &InputState, world: &ClientWorld, delta_seconds: f32) {
        let move_speed = 8.0;
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
            let delta = movement.normalize() * move_speed * delta_seconds;
            self.try_move(Vec3::new(delta.x, 0.0, 0.0), world);
            self.try_move(Vec3::new(0.0, 0.0, delta.z), world);
        }

        if input.jump && self.grounded {
            self.vertical_velocity = 8.0;
            self.grounded = false;
        }

        self.vertical_velocity -= 22.0 * delta_seconds;
        self.grounded = false;
        let vertical_delta = Vec3::new(0.0, self.vertical_velocity * delta_seconds, 0.0);
        if !self.try_move(vertical_delta, world) {
            if vertical_delta.y < 0.0 {
                self.grounded = true;
            }
            self.vertical_velocity = 0.0;
        }

        if self.position.y < -80.0 {
            self.position = world.safe_spawn_eye_position(Vec3::new(0.0, 0.0, 20.0));
            self.vertical_velocity = 0.0;
            self.grounded = false;
        }
    }

    fn try_move(&mut self, delta: Vec3, world: &ClientWorld) -> bool {
        if delta.length_squared() == 0.0 {
            return true;
        }

        let next = self.position + delta;
        if world.collides_player_at(next) {
            return false;
        }

        self.position = next;
        true
    }

    fn apply_mouse_delta(&mut self, delta_x: f32, delta_y: f32) {
        let sensitivity = 0.0025;
        self.yaw += delta_x * sensitivity;
        self.pitch = (self.pitch - delta_y * sensitivity).clamp(-1.35, 1.20);
    }

    fn view_projection(&self, width: u32, height: u32) -> Mat4 {
        let aspect = width.max(1) as f32 / height.max(1) as f32;
        let view = Mat4::look_to_rh(self.position, self.forward(), Vec3::Y);
        let projection = Mat4::perspective_rh(60.0_f32.to_radians(), aspect, 0.1, 500.0);
        projection * view
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
}

impl InputState {
    fn handle_key(&mut self, event: &KeyEvent) {
        let pressed = event.state == ElementState::Pressed;
        self.handle_logical_key(event.logical_key.as_ref(), pressed);

        if let PhysicalKey::Code(code) = event.physical_key {
            match code {
                KeyCode::Space => self.jump = pressed,
                _ => {}
            }
        }
    }

    fn handle_logical_key(&mut self, key: Key<&str>, pressed: bool) {
        match key {
            Key::Character(character) => match character.to_lowercase().as_str() {
                "z" => self.forward = pressed,
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
    mesher: ChunkMesher,
}

impl ClientWorld {
    fn new(blocks: BlockRegistry, block_ids: BlockIds) -> Self {
        Self {
            chunks: HashMap::new(),
            blocks,
            block_ids,
            mesher: ChunkMesher,
        }
    }

    fn insert_chunk(&mut self, chunk: Chunk) {
        self.chunks.insert(chunk.position(), chunk);
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

    fn build_render_mesh(&self, texture_atlas: &TextureAtlas) -> (Vec<Vertex>, Vec<u32>) {
        let mut chunk_meshes = Vec::with_capacity(self.chunks.len());
        for (position, chunk) in &self.chunks {
            chunk_meshes.push((*position, self.mesher.mesh_chunk(chunk, &self.blocks)));
        }
        build_render_mesh(&chunk_meshes, &self.blocks, texture_atlas)
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
            let eye_y = feet_y as f32 - 64.0 + PLAYER_EYE_HEIGHT + 0.05;
            if !self.collides_player_at(Vec3::new(render_x, eye_y, render_z)) {
                return Some(eye_y);
            }
        }

        None
    }

    fn collides_player_at(&self, eye_position: Vec3) -> bool {
        let min = Vec3::new(
            eye_position.x - PLAYER_RADIUS,
            eye_position.y - PLAYER_EYE_HEIGHT,
            eye_position.z - PLAYER_RADIUS,
        );
        let max = Vec3::new(
            eye_position.x + PLAYER_RADIUS,
            eye_position.y - PLAYER_EYE_HEIGHT + PLAYER_HEIGHT,
            eye_position.z + PLAYER_RADIUS,
        );
        self.collides_aabb(min, max)
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

    fn break_block(&mut self, position: WorldBlockPosition) -> bool {
        self.set_block(position, self.block_ids.air)
    }

    fn place_block(&mut self, position: WorldBlockPosition, block: BlockId) -> bool {
        if self.is_solid(position) {
            return false;
        }
        self.set_block(position, block)
    }

    fn is_solid(&self, position: WorldBlockPosition) -> bool {
        self.block(position)
            .and_then(|block| self.blocks.get(block))
            .map(|definition| definition.solid)
            .unwrap_or(false)
    }

    fn block(&self, position: WorldBlockPosition) -> Option<BlockId> {
        let (chunk_position, block_position) = split_world_block_position(position)?;
        self.chunks
            .get(&chunk_position)
            .and_then(|chunk| chunk.block(block_position))
    }

    fn set_block(&mut self, position: WorldBlockPosition, block: BlockId) -> bool {
        let Some((chunk_position, block_position)) = split_world_block_position(position) else {
            return false;
        };
        let Some(chunk) = self.chunks.get_mut(&chunk_position) else {
            return false;
        };
        chunk.set_block(block_position, block).is_ok()
    }
}

fn world_block_from_render(position: Vec3) -> WorldBlockPosition {
    WorldBlockPosition {
        x: render_x_to_block_world(position.x).floor() as i32,
        y: render_y_to_block_world(position.y).floor() as i32,
        z: render_z_to_block_world(position.z).floor() as i32,
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

fn split_world_block_position(
    position: WorldBlockPosition,
) -> Option<(ChunkPosition, BlockPosition)> {
    if position.y < 0 || position.y >= CHUNK_HEIGHT as i32 {
        return None;
    }

    let chunk_position = ChunkPosition {
        x: position.x.div_euclid(CHUNK_SIZE as i32),
        z: position.z.div_euclid(CHUNK_SIZE as i32),
    };
    let block_position = BlockPosition {
        x: position.x.rem_euclid(CHUNK_SIZE as i32) as usize,
        y: position.y as usize,
        z: position.z.rem_euclid(CHUNK_SIZE as i32) as usize,
    };

    Some((chunk_position, block_position))
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
            if !should_render_preview_quad(quad, *chunk_position) {
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

fn should_render_preview_quad(
    quad: &crate::engine::mesh::chunk_mesher::MeshQuad,
    chunk_position: ChunkPosition,
) -> bool {
    use crate::engine::mesh::chunk_mesher::FaceDirection;

    match quad.direction {
        FaceDirection::Up => true,
        FaceDirection::North | FaceDirection::South | FaceDirection::East | FaceDirection::West => {
            if is_outer_render_boundary(quad, chunk_position) {
                return false;
            }
            quad.vertices
                .iter()
                .map(|vertex| vertex[1])
                .fold(f32::MIN, f32::max)
                >= 60.0
        }
        FaceDirection::Down => false,
    }
}

fn is_outer_render_boundary(
    quad: &crate::engine::mesh::chunk_mesher::MeshQuad,
    chunk_position: ChunkPosition,
) -> bool {
    use crate::engine::mesh::chunk_mesher::FaceDirection;

    let offset_x = chunk_position.x as f32 * CHUNK_WORLD_SIZE;
    let offset_z = chunk_position.z as f32 * CHUNK_WORLD_SIZE;
    match quad.direction {
        FaceDirection::West => quad
            .vertices
            .iter()
            .all(|vertex| nearly_equal(vertex[0] + offset_x, RENDER_MIN_WORLD)),
        FaceDirection::East => quad
            .vertices
            .iter()
            .all(|vertex| nearly_equal(vertex[0] + offset_x, RENDER_MAX_WORLD)),
        FaceDirection::North => quad
            .vertices
            .iter()
            .all(|vertex| nearly_equal(vertex[2] + offset_z, RENDER_MIN_WORLD)),
        FaceDirection::South => quad
            .vertices
            .iter()
            .all(|vertex| nearly_equal(vertex[2] + offset_z, RENDER_MAX_WORLD)),
        FaceDirection::Up | FaceDirection::Down => false,
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

fn build_crosshair_mesh() -> (Vec<Vertex>, Vec<u32>) {
    let color = [0.02, 0.02, 0.02];
    let thickness = 0.003;
    let length = 0.035;
    let vertices = vec![
        Vertex {
            position: [-length, -thickness, 0.0],
            color,
            tex_coords: [0.0, 0.0],
        },
        Vertex {
            position: [length, -thickness, 0.0],
            color,
            tex_coords: [0.0, 0.0],
        },
        Vertex {
            position: [length, thickness, 0.0],
            color,
            tex_coords: [0.0, 0.0],
        },
        Vertex {
            position: [-length, thickness, 0.0],
            color,
            tex_coords: [0.0, 0.0],
        },
        Vertex {
            position: [-thickness, -length, 0.0],
            color,
            tex_coords: [0.0, 0.0],
        },
        Vertex {
            position: [thickness, -length, 0.0],
            color,
            tex_coords: [0.0, 0.0],
        },
        Vertex {
            position: [thickness, length, 0.0],
            color,
            tex_coords: [0.0, 0.0],
        },
        Vertex {
            position: [-thickness, length, 0.0],
            color,
            tex_coords: [0.0, 0.0],
        },
    ];
    let indices = vec![0, 1, 2, 0, 2, 3, 4, 5, 6, 4, 6, 7];
    (vertices, indices)
}

fn build_outline_vertices(block: WorldBlockPosition) -> Vec<Vertex> {
    let color = [0.01, 0.01, 0.01];
    let min = Vec3::new(
        block.x as f32 - 8.0 - 0.01,
        block.y as f32 - 64.0 - 0.01,
        block.z as f32 - 8.0 - 0.01,
    );
    let max = min + Vec3::splat(1.02);
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
    use crate::content::bootstrap_content;
    use crate::engine::mesh::chunk_mesher::{FaceDirection, MeshQuad};

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
    fn preview_filter_hides_deep_chunk_wall_faces() {
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

        assert!(!should_render_preview_quad(
            &wall,
            ChunkPosition { x: 0, z: 0 }
        ));
        assert!(should_render_preview_quad(
            &surface_side,
            ChunkPosition { x: 0, z: 0 }
        ));
        assert!(should_render_preview_quad(
            &top,
            ChunkPosition { x: 0, z: 0 }
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

        assert!(!should_render_preview_quad(
            &outer_west_wall,
            ChunkPosition {
                x: RENDER_MIN_CHUNK,
                z: 0
            }
        ));
        assert!(should_render_preview_quad(
            &inner_west_wall,
            ChunkPosition {
                x: RENDER_MIN_CHUNK,
                z: 0
            }
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
    fn client_world_can_break_and_place_blocks() {
        let content = bootstrap_content().unwrap();
        let mut world = ClientWorld::new(content.blocks, content.block_ids);
        let mut chunk = Chunk::filled(ChunkPosition { x: 0, z: 0 }, content.block_ids.air);
        chunk
            .set_block(BlockPosition { x: 1, y: 1, z: 1 }, content.block_ids.stone)
            .unwrap();
        world.insert_chunk(chunk);
        let position = WorldBlockPosition { x: 1, y: 1, z: 1 };

        assert!(world.is_solid(position));
        assert!(world.break_block(position));
        assert!(!world.is_solid(position));
        assert!(world.place_block(position, content.block_ids.dirt));
        assert!(world.is_solid(position));
    }

    #[test]
    fn player_aabb_detects_wall_collision_without_surface_snap() {
        let content = bootstrap_content().unwrap();
        let mut world = ClientWorld::new(content.blocks, content.block_ids);
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
        let mut world = ClientWorld::new(content.blocks, content.block_ids);
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
    fn outline_has_twelve_edges() {
        let vertices = build_outline_vertices(WorldBlockPosition { x: 8, y: 64, z: 8 });

        assert_eq!(vertices.len(), 24);
    }

    #[test]
    fn raycast_returns_hit_and_previous_empty_block() {
        let content = bootstrap_content().unwrap();
        let mut world = ClientWorld::new(content.blocks, content.block_ids);
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
    }
}
