use glam::{Mat4, Vec3};

use super::constants::{
    SKY_COLOR, WORLD_FOG_END_BLOCKS, WORLD_FOG_MAX_AMOUNT, WORLD_FOG_START_BLOCKS,
};

#[repr(C)]
#[derive(Debug, Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub(super) struct CameraUniform {
    view_proj: [[f32; 4]; 4],
    camera_position_fog_start: [f32; 4],
    fog_color_fog_end: [f32; 4],
    fog_max_padding: [f32; 4],
}

impl CameraUniform {
    pub(super) fn new(view_projection: Mat4, camera_position: Vec3) -> Self {
        Self {
            view_proj: view_projection.to_cols_array_2d(),
            camera_position_fog_start: [
                camera_position.x,
                camera_position.y,
                camera_position.z,
                WORLD_FOG_START_BLOCKS,
            ],
            fog_color_fog_end: [
                SKY_COLOR[0],
                SKY_COLOR[1],
                SKY_COLOR[2],
                WORLD_FOG_END_BLOCKS,
            ],
            fog_max_padding: [WORLD_FOG_MAX_AMOUNT, 0.0, 0.0, 0.0],
        }
    }
}

#[repr(C)]
#[derive(Debug, Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub(super) struct Vertex {
    pub(super) position: [f32; 3],
    pub(super) color: [f32; 3],
    pub(super) tex_coords: [f32; 2],
}

impl Vertex {
    pub(super) fn layout() -> wgpu::VertexBufferLayout<'static> {
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
