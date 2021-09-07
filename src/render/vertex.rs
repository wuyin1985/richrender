use crate::render::buffer::Buffer;
use ash::vk;
use crate::render::render_context::RenderContext;

#[derive(Clone, Copy, Debug)]
pub struct Vertex {
    pub position: [f32; 3],
    pub normal: [f32; 3],
    pub tex_coord: [f32; 2],
    pub weight: [f32; 4],
    pub joint: [u32; 4],
}

/// Vertex buffer byte offset / element count
pub type VertexBufferPart = (usize, usize);

/// Index buffer byte offset / element count
pub type IndexBufferPart = (usize, usize);