use gltf;
use gltf::Gltf;
use std::error::Error;
use crate::render::aabb::Aabb;
use std::mem::size_of;
use crate::render::material::Material;
use crate::render::render_context::RenderContext;
use crate::render::texture::Texture;
use crate::render::util;

use ash::vk;
use crate::render::mesh::{Meshes, Mesh};
use crate::render::node::{Nodes, Node};
use crate::render::buffer::Buffer;
use crate::render::vertex_layout::VertexLayout;
use crate::render::gltf_asset_loader::GltfAsset;

use bevy::prelude::*;
use crate::render::animation::{Animations, load_animations};
use crate::render::skin::{create_skins_from_gltf, Skin};


pub struct Model {
    meshes: Meshes,
    nodes: Nodes,
    textures: ModelTextures,
    animations: Option<Animations>,
    skins: Vec<Skin>,
    pub aabb: Aabb,
}

impl Model {
    pub fn destroy(&mut self, context: &RenderContext) {
        self.meshes.destroy(context);
        self.textures.destroy(context);
    }

    pub fn from_gltf(context: &mut RenderContext, upload_command_buffer: vk::CommandBuffer, asset: &GltfAsset) -> Result<Model, Box<dyn Error>> {
        let (document, buffers, images) = asset.export();
        let meshes = Meshes::from_gltf(context, upload_command_buffer, &document, &buffers);
        let textures = ModelTextures::from_gltf(context, upload_command_buffer, document.textures(), &images);
        let nodes = Nodes::from_gltf(document.nodes(), &document.default_scene().unwrap());
        let animations = load_animations(document.animations(), &buffers);
        info!("animations is some {}", animations.is_some());

        let mut skins = create_skins_from_gltf(document.skins(), &buffers);

        nodes.get_skins_transform()
            .iter()
            .for_each(|(index, transform)| {
                let skin = &mut skins[*index];
                skin.compute_joints_matrices(*transform, &nodes.nodes());
            });

        let aabbs = nodes
            .nodes()
            .iter()
            .filter(|n| n.mesh_index().is_some())
            .map(|n| {
                let mesh = &meshes.meshes[n.mesh_index().unwrap()];
                mesh.aabb() * n.transform()
            })
            .collect::<Vec<_>>();

        let aabb = Aabb::union(&aabbs).unwrap();

        Ok(
            Model {
                aabb,
                nodes,
                textures,
                meshes,
                animations,
                skins,
            })
    }

    pub fn primitive_count(&self) -> usize {
        self.meshes.meshes.iter().map(Mesh::primitive_count).sum()
    }

    pub fn get_meshes(&self) -> &Vec<Mesh> {
        &self.meshes.meshes
    }

    pub fn get_nodes(&self) -> &[Node] {
        self.nodes.nodes()
    }

    pub fn nodes(&self) -> &Nodes{
        &self.nodes
    }

    pub fn aabb(&self) -> Aabb {
        self.aabb
    }

    pub fn get_vertex_layout(&self) -> &VertexLayout {
        &self.meshes.meshes[0].primitives()[0].get_vertex_layout()
    }

    pub fn get_buffer(&self) -> &Buffer {
        &self.meshes.buffer
    }

    pub fn get_textures(&self) -> &Vec<ModelTexture> {
        &self.textures.textures
    }

    pub fn clone_animations(&self) -> Option<Animations> {
        self.animations.clone()
    }

    pub fn clone_nodes(&self) -> Nodes {
        self.nodes.clone()
    }

    pub fn clone_skins(&self) -> Vec<Skin> { self.skins.clone() }

    pub fn get_skins(&self) -> &Vec<Skin> {
        &self.skins
    }

    pub fn has_animation(&self) -> bool {
        self.animations.is_some()
    }
}


pub struct ModelTexture {
    pub texture: Texture,
    pub view: vk::ImageView,
    pub sampler: vk::Sampler,
}

impl ModelTexture {
    pub fn destroy(&mut self, context: &RenderContext) {
        unsafe {
            context.device.destroy_image_view(self.view, None);
            context.device.destroy_sampler(self.sampler, None);
        }
        self.texture.destroy(context);
    }

    pub fn from(context: &RenderContext, texture: Texture) -> Self
    {
        let view = texture.create_color_view(context);

        let sampler = {
            let sampler_info = vk::SamplerCreateInfo::builder()
                .mag_filter(vk::Filter::LINEAR)
                .min_filter(vk::Filter::LINEAR)
                .address_mode_u(vk::SamplerAddressMode::REPEAT)
                .address_mode_v(vk::SamplerAddressMode::REPEAT)
                .address_mode_w(vk::SamplerAddressMode::REPEAT)
                .anisotropy_enable(false)
                .max_anisotropy(16.0)
                .border_color(vk::BorderColor::INT_OPAQUE_BLACK)
                .unnormalized_coordinates(false)
                .compare_enable(false)
                .compare_op(vk::CompareOp::ALWAYS)
                .mipmap_mode(vk::SamplerMipmapMode::LINEAR)
                .mip_lod_bias(0.0)
                .min_lod(0.0)
                .max_lod(0.25);

            unsafe {
                context
                    .device
                    .create_sampler(&sampler_info, None)
                    .expect("Failed to create sampler")
            }
        };


        Self { texture, view, sampler }
    }
}


pub struct ModelTextures {
    pub textures: Vec<ModelTexture>,
}

fn get_next_rgba(pixels: &[u8], format: gltf::image::Format, index: usize) -> [u8; 4] {
    use gltf::image::Format::*;
    match format {
        R8 => [pixels[index], 0, 0, std::u8::MAX],
        R8G8 => [pixels[index * 2], pixels[index * 2 + 1], 0, std::u8::MAX],
        R8G8B8 => [
            pixels[index * 3],
            pixels[index * 3 + 1],
            pixels[index * 3 + 2],
            std::u8::MAX,
        ],
        B8G8R8 => [
            pixels[index * 3 + 2],
            pixels[index * 3 + 1],
            pixels[index * 3],
            std::u8::MAX,
        ],
        R8G8B8A8 => [
            pixels[index * 4],
            pixels[index * 4 + 1],
            pixels[index * 4 + 2],
            pixels[index * 4 + 3],
        ],
        B8G8R8A8 => [
            pixels[index * 4 + 2],
            pixels[index * 4 + 1],
            pixels[index * 4],
            pixels[index * 4 + 3],
        ],
        R16 | R16G16 | R16G16B16 | R16G16B16A16 => {
            panic!("16 bits colors are not supported for model textures")
        }
    }
}

fn build_rgba_buffer(image: &gltf::image::Data) -> Vec<u8> {
    let mut buffer = Vec::new();
    let size = image.width * image.height;
    for index in 0..size {
        let rgba = get_next_rgba(&image.pixels, image.format, index as usize);
        buffer.extend_from_slice(&rgba);
    }
    buffer
}

fn create_texture_by_gltf_image_data(context: &mut RenderContext, upload_command_buffer: vk::CommandBuffer, image: &gltf::image::Data) -> Texture {
    let max_mip_levels = ((image.width.min(image.height) as f32).log2().floor() + 1.0) as u32;
    //todo better way convert image
    let vk_format = vk::Format::R8G8B8A8_UNORM;
    //let vk_format = util::Gltf2VkConvertor::format(image.format);
    let image_ci = vk::ImageCreateInfo::builder()
        .extent(vk::Extent3D { width: image.width, height: image.height, depth: 1 })
        .usage(vk::ImageUsageFlags::TRANSFER_SRC | vk::ImageUsageFlags::TRANSFER_DST | vk::ImageUsageFlags::SAMPLED)
        .sharing_mode(vk::SharingMode::EXCLUSIVE)
        .mip_levels(max_mip_levels)
        .array_layers(1)
        .initial_layout(vk::ImageLayout::UNDEFINED)
        .samples(vk::SampleCountFlags::TYPE_1)
        .format(vk_format)
        .flags(vk::ImageCreateFlags::empty())
        .image_type(vk::ImageType::TYPE_2D)
        .tiling(vk::ImageTiling::OPTIMAL)
        .build();

    let rgba = build_rgba_buffer(&image);

    let texture = Texture::create_from_data(context, upload_command_buffer, &image_ci, &rgba);
    texture.cmd_transition_image_layout(context, upload_command_buffer, vk::ImageLayout::UNDEFINED, vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL);
    texture
}

impl ModelTextures {
    pub fn destroy(&mut self, context: &RenderContext) {
        for t in self.textures.iter_mut() {
            t.destroy(context);
        }
    }

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