use gltf;
use gltf::Gltf;
use glam;
use std::error::Error;
use crate::render::aabb::Aabb;
use std::mem::size_of;
use crate::render::material::Material;
use crate::render::render_context::RenderContext;
use crate::render::texture::Texture;
use crate::render::util;

use ash::vk;
use crate::render::mesh::Meshes;
use crate::render::node::Nodes;


pub struct Model {
    meshes: Meshes,
    nodes: Nodes,
    textures: ModelTextures,
}

impl Model {
    pub fn from_gltf(context: &mut RenderContext, upload_command_buffer: vk::CommandBuffer, path: &str) -> Result<Model, Box<dyn Error>> {
        let (document, buffers, images) = gltf::import(&path)?;
        let meshes = Meshes::from_gltf(context, upload_command_buffer, &document, &buffers);
        let textures = ModelTextures::from_gltf(context, upload_command_buffer, document.textures(), &images);
        let nodes = Nodes::from_gltf(document.nodes(), &document.default_scene().unwrap());
        Ok(
            Model {
                nodes,
                textures,
                meshes,
            })
    }
}


pub struct ModelTexture {
    pub texture: Texture,
    pub view: vk::ImageView,
    pub sampler: vk::Sampler,
}


pub struct ModelTextures {
    pub textures: Vec<ModelTexture>,
}


fn create_texture_by_gltf_image_data(context: &mut RenderContext, upload_command_buffer: vk::CommandBuffer, image: &gltf::image::Data) -> Texture {
    let max_mip_levels = ((image.width.min(image.height) as f32).log2().floor() + 1.0) as u32;
    let vk_format = util::Gltf2VkConvertor::format(image.format);
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

    let texture = Texture::create_from_data(context, upload_command_buffer, &image_ci, &image.pixels);
    texture
}

impl ModelTextures {
    pub fn from_gltf(context: &mut RenderContext, command_buffer: vk::CommandBuffer,
                     gltf_textures: gltf::iter::Textures, gltf_image_datas: &[gltf::image::Data]) -> ModelTextures {
        let model_textures = gltf_textures.map(|t| {
            let texture = create_texture_by_gltf_image_data(context, command_buffer, &gltf_image_datas[t.source().index()]);
            let view = texture.create_color_view(context);
            let sampler = util::Gltf2VkConvertor::sampler(context, &texture, t.sampler());
            ModelTexture { texture, view, sampler }
        }).collect::<Vec<_>>();

        ModelTextures {
            textures: model_textures
        }
    }
}