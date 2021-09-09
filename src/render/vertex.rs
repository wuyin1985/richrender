

use crate::render::buffer::Buffer;
use ash::vk;
use crate::render::render_context::RenderContext;
use std::mem::size_of;

const POS_OFFSET: u32 = 0;
const NORMAL_OFFSET: u32 = POS_OFFSET + 3 * 4;
const TEX_COORD_OFFSET: u32 = NORMAL_OFFSET + 3 * 4;
const WEIGHT_OFFSET: u32 = TEX_COORD_OFFSET + 2 * 4;
const JOINT_OFFSET: u32 = WEIGHT_OFFSET + 4 * 4;

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

lazy_static! {
    static ref VERTEX_BINDING_DESCS: [vk::VertexInputBindingDescription; 1] = Vertex::get_bindings_descs();

    static ref VERTEX_ATTRIBUTE_DESCS: [vk::VertexInputAttributeDescription; 5] = Vertex::get_attribute_descs();
}

impl Vertex {
    fn get_bindings_descs() -> [vk::VertexInputBindingDescription; 1] {
        [
            vk::VertexInputBindingDescription::builder().binding(0).
                input_rate(vk::VertexInputRate::VERTEX).stride(size_of::<Vertex>() as _).build(),
        ]
    }

    fn get_attribute_descs() -> [vk::VertexInputAttributeDescription; 5] {
        [
            //position
            vk::VertexInputAttributeDescription::builder().
                format(vk::Format::R32G32B32_SFLOAT).
                binding(0).
                location(1).
                offset(POS_OFFSET).build(),

            //normal
            vk::VertexInputAttributeDescription::builder().
                format(vk::Format::R32G32B32_SFLOAT).
                binding(0).
                location(2).
                offset(NORMAL_OFFSET).build(),

            //text_coord
            vk::VertexInputAttributeDescription::builder().
                format(vk::Format::R32G32_SFLOAT).
                binding(0).
                location(3).
                offset(NORMAL_OFFSET).build(),

            //weight
            vk::VertexInputAttributeDescription::builder().
                format(vk::Format::R32G32B32A32_SFLOAT).
                binding(0).
                location(4).
                offset(WEIGHT_OFFSET).build(),

            //joint
            vk::VertexInputAttributeDescription::builder().
                format(vk::Format::R32G32B32A32_SFLOAT).
                binding(0).
                location(5).
                offset(JOINT_OFFSET).build(),
        ]
    }
}