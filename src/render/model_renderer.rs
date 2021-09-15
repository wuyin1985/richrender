use crate::render::model::{Model, ModelTexture};
use ash::vk;
use crate::render::buffer::Buffer;
use crate::render::render_context::{RenderContext, PerFrameData};
use crate::render::graphic_pipeline::{GraphicPipeline, PipelineVertexInputInfo, PipelineLayoutInfo};
use crate::render::swapchain_mgr::SwapChainMgr;
use crate::render::{vertex, util};
use glam::{Mat4, Vec3, Quat};
use std::mem::size_of;
use crate::render::texture::Texture;
use bevy::prelude::*;
use crate::render::uniform::UniformObject;
use crate::render::aabb::Aabb;

const UBO_BINDING: u32 = 0;
const COLOR_SAMPLER_BINDING: u32 = 1;


#[repr(C)]
#[derive(Clone, Debug, Copy)]
pub struct ModelData {
    transform: Mat4,
}

pub struct ModelRenderer {
    model: Model,
    model_data: ModelData,
    pipeline: GraphicPipeline,
    buffers_ref_for_draw: Vec<vk::Buffer>,
    descriptor_set_layout: vk::DescriptorSetLayout,
    descriptor_sets: Vec<vk::DescriptorSet>,
}

impl ModelRenderer {
    pub fn destroy(&mut self, context: &RenderContext) {
        unsafe {
            context.device.free_descriptor_sets(context.descriptor_pool, &self.descriptor_sets);
            context.device.destroy_descriptor_set_layout(self.descriptor_set_layout, None);
        }
        self.pipeline.destroy(context);
        self.model.destroy(context);
    }

    fn create_model_descriptor_set_layout(context: &RenderContext) -> vk::DescriptorSetLayout {
        let bindings = [
            vk::DescriptorSetLayoutBinding::builder()
                .binding(0)
                .descriptor_type(vk::DescriptorType::COMBINED_IMAGE_SAMPLER)
                .descriptor_count(1)
                .stage_flags(vk::ShaderStageFlags::FRAGMENT)
                .build()];

        let layout_info = vk::DescriptorSetLayoutCreateInfo::builder().bindings(&bindings).build();

        unsafe {
            context.device
                .create_descriptor_set_layout(&layout_info, None)
                .unwrap()
        }
    }

    fn create_descriptor_image_info(
        index: usize,
        textures: &[ModelTexture],
    ) -> [vk::DescriptorImageInfo; 1] {
        let texture = &textures[index];
        let (view, sampler) = (texture.view, texture.sampler);

        [vk::DescriptorImageInfo::builder()
            .image_layout(vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL)
            .image_view(view)
            .sampler(sampler)
            .build()]
    }

    fn create_model_descriptors(context: &mut RenderContext, model: &Model) -> (vk::DescriptorSetLayout, Vec::<vk::DescriptorSet>) {
        let set_layout = Self::create_model_descriptor_set_layout(context);

        let layouts = (0..model.primitive_count())
            .map(|_| set_layout)
            .collect::<Vec<_>>();

        let allocate_info = vk::DescriptorSetAllocateInfo::builder()
            .descriptor_pool(context.descriptor_pool)
            .set_layouts(&layouts);
        let sets = unsafe {
            context
                .device
                .allocate_descriptor_sets(&allocate_info)
                .unwrap()
        };

        let textures = model.get_textures();
        let mut primitive_index = 0;
        for mesh in model.get_meshes() {
            for primitive in mesh.get_primitives() {
                let material = primitive.get_material();
                let texture_index = material.get_color_texture_index();
                if texture_index.is_none() { continue; };
                let albedo_info = Self::create_descriptor_image_info(
                    texture_index.unwrap(),
                    textures,
                );

                let set = sets[primitive_index];
                primitive_index += 1;

                let descriptor_writes = [vk::WriteDescriptorSet::builder()
                    .dst_set(set)
                    .dst_binding(0)
                    .descriptor_type(vk::DescriptorType::COMBINED_IMAGE_SAMPLER)
                    .image_info(&albedo_info)
                    .build()];

                unsafe {
                    context
                        .device
                        .update_descriptor_sets(&descriptor_writes, &[])
                }
            }
        }

        (set_layout, sets)
    }

    fn get_center_transform(model_aabb: Aabb) -> Mat4 {
        let aabb = model_aabb * Mat4::from_rotation_x(0f32.to_radians());
        let CAMERA_DIS: f32 = 1.0;
        let CAMERA_FOV: f32 = 45f32.to_radians() / 2f32;
        let target_z = (aabb.max.z - aabb.min.z) / 2f32;
        let target_y = (aabb.max.y - aabb.min.y) / 2f32;

        let offset_x = (aabb.max.x + aabb.min.x) / 2f32;
        let offset_z = (aabb.max.z + aabb.min.z) / 2f32;
        let offset_y = (aabb.max.y + aabb.min.y) / 2f32;

        println!("aabb {:?} {:?}", model_aabb.min, model_aabb.max);

        assert!(target_y > 0f32, "error model half y");
        let to_z = -1f32;
        let desire_y = CAMERA_FOV.tan() * (CAMERA_DIS - to_z);
        let scale = desire_y / target_y;
        //let scale = 1f32;
        return Mat4::from_scale_rotation_translation(Vec3::new(scale, scale, scale),
                                                     Quat::from_axis_angle(Vec3::X, 0f32.to_radians()),
                                                     Vec3::new(-offset_x, 0f32, to_z));
    }

    pub fn create(context: &mut RenderContext, swapchain_mgr: &SwapChainMgr, render_pass: vk::RenderPass,
                  command_buffer: vk::CommandBuffer, gltf_path: &str, vert_shader_path: &str, frag_shader_path: &str) -> ModelRenderer {
        info!("start load model {}", gltf_path);
        let model = Model::from_gltf(context, command_buffer, gltf_path).expect("load error");
        info!("load model {} complete", gltf_path);

        let vertex_layout = model.get_vertex_layout();
        let vertex_bindings = vertex_layout.build_vk_bindings();
        let vertex_attributes = vertex_layout.build_vk_attributes();
        let vertex_input = PipelineVertexInputInfo::from(&vertex_bindings, &vertex_attributes);

        let buffers_ref_for_draw = (0..vertex_bindings.len()).map(|_| model.get_buffer().buffer).collect::<Vec<_>>();

        let frame_uniform_layout = context.per_frame_uniform.as_mut().unwrap().descriptor_set_layout;

        let (descriptor_set_layout, descriptor_sets) = Self::create_model_descriptors(context, &model);

        let all_layout = [frame_uniform_layout, descriptor_set_layout];

        let constant_ranges = [
            vk::PushConstantRange::builder().offset(0).size(size_of::<ModelData>() as _).stage_flags(vk::ShaderStageFlags::VERTEX).build()
        ];

        let pipeline_layout_ci = vk::PipelineLayoutCreateInfo::builder().set_layouts(&all_layout).push_constant_ranges(&constant_ranges).build();

        let pipeline = GraphicPipeline::create(context,
                                               swapchain_mgr,
                                               render_pass,
                                               &vertex_input, &pipeline_layout_ci, context.render_config.msaa,
                                               vert_shader_path, frag_shader_path);

        // let pos = Vec3::new(0.00248157978, -0.0000104904175, 0.187154889);
        // let s = 0.249999f32;
        // let scale = Vec3::new(s, s, s);
        // let t = Mat4::from_scale_rotation_translation(scale, Quat::from_axis_angle(Vec3::new(1.0, 0.0, 0.0), 90f32), pos);

        let t = Self::get_center_transform(model.aabb());
        let model_data = ModelData {
            transform: t
        };
        ModelRenderer {
            pipeline,
            model_data,
            model,
            buffers_ref_for_draw,
            descriptor_set_layout,
            descriptor_sets,
        }
    }

    pub fn draw(&self, context: &RenderContext, command_buffer: vk::CommandBuffer) {
        unsafe {
            context.device.cmd_bind_pipeline(command_buffer, vk::PipelineBindPoint::GRAPHICS, self.pipeline.get_pipeline());

            let mut set_idx = 0;
            let uniform = context.per_frame_uniform.as_ref().unwrap();


            for node in self.model.get_nodes() {
                if let Some(mesh_idx) = node.mesh_index() {
                    let m_data = ModelData { transform: self.model_data.transform * node.transform() };
                    let model_data_bytes: &[u8] = unsafe { util::any_as_u8_slice(&m_data) };
                    context.device.cmd_push_constants(command_buffer, self.pipeline.get_layout(),
                                                      vk::ShaderStageFlags::VERTEX, 0, model_data_bytes);

                    let mesh = &self.model.get_meshes()[mesh_idx];
                    for primitive in mesh.primitives() {
                        let vertex_layout = &primitive.get_vertex_layout();
                        context.device.cmd_bind_vertex_buffers(command_buffer,
                                                               0,
                                                               &self.buffers_ref_for_draw,
                                                               &vertex_layout.buffers_ref_offsets);
                        context.device.cmd_bind_index_buffer(command_buffer,
                                                             self.model.get_buffer().buffer,
                                                             vertex_layout.indices.index as _,
                                                             vertex_layout.indices_type);

                        let set = self.descriptor_sets[set_idx];
                        context.device.cmd_bind_descriptor_sets(command_buffer,
                                                                vk::PipelineBindPoint::GRAPHICS,
                                                                self.pipeline.get_layout(),
                                                                0,
                                                                &[uniform.descriptor_set, set], &[]);

                        context.device.cmd_draw_indexed(command_buffer, vertex_layout.indices.count as _, 1, 0, 0, 0);
                        set_idx += 1;
                    }
                }
            }
        }
    }
}