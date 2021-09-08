use gltf;
use gltf::Gltf;
use glam;
use std::error::Error;
use crate::render::aabb::Aabb;
use std::mem::size_of;
use crate::render::material::Material;
use crate::render::render_context::RenderContext;
use crate::render::texture::Texture;

use ash::vk;

struct Node {
    parent: Option<usize>,
    children: Option<Vec<usize>>,
    translation: glam::Vec3,
    scale: glam::Vec3,
    rotation: glam::Quat,
    mesh: Option<usize>,
}


struct Model {
    // root: usize,
    // nodes: Vec<Node>,
}


fn test_load_gltf(path: &str) -> Result<Model, Box<dyn Error>> {
    Ok(Model {})
}

pub struct ModelTexture {
    pub texture: Texture,
    pub view: vk::ImageView,
    pub sample: vk::Sampler,
}


pub struct ModelTextures {
    pub textures: Vec<ModelTexture>,
}


fn gltf_texture_format_2_vk_format(gltf_format: gltf::image::Format) -> vk::Format {
    match gltf_format {
        gltf::image::Format::R8 => vk::Format::R8_UNORM,
        gltf::image::Format::R8G8 => vk::Format::R8G8_UNORM,
        gltf::image::Format::R8G8B8 => vk::Format::R8G8B8_UNORM,
        gltf::image::Format::R8G8B8A8 => vk::Format::R8G8B8A8_UNORM,
        gltf::image::Format::B8G8R8 => vk::Format::B8G8R8_UNORM,
        gltf::image::Format::B8G8R8A8 => vk::Format::B8G8R8A8_UNORM,
        gltf::image::Format::R16 => vk::Format::R16_UNORM,
        gltf::image::Format::R16G16 => vk::Format::R16G16_UNORM,
        gltf::image::Format::R16G16B16 => vk::Format::R16G16B16_UNORM,
        gltf::image::Format::R16G16B16A16 => vk::Format::R16G16B16A16_UNORM,
    }
}


impl ModelTextures {
    pub fn create_from_gltf_texture(context: &mut RenderContext, command_buffer: vk::CommandBuffer,
                                    gltf_textures: gltf::iter::Textures, gltf_image_datas: &[gltf::image::Data]) {
        let res = gltf_image_datas
            .iter()
            .map(|image|
                {
                    let max_mip_levels = ((image.width.min(image.height) as f32).log2().floor() + 1.0) as u32;
                    let vk_format = gltf_texture_format_2_vk_format(image.format);
                    let image_ci = vk::ImageCreateInfo::builder()
                        .extent(vk::Extent3D { width: image.width, height: image.height, depth: 1 })
                        .usage(vk::ImageUsageFlags::TRANSFER_DST | vk::ImageUsageFlags::SAMPLED)
                        .sharing_mode(vk::SharingMode::EXCLUSIVE)
                        .mip_levels(max_mip_levels)
                        .initial_layout(vk::ImageLayout::UNDEFINED)
                        .samples(vk::SampleCountFlags::TYPE_1)
                        .format(vk_format)
                        .flags(vk::ImageCreateFlags::empty())
                        .build();
                       
                    let texture = Texture::create_from_data(context, command_buffer, &image_ci, &image.pixels);
                });
            // .unzip::<_, _, Vec<_>, _>();
    }
}