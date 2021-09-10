use crate::render::model::{Model, ModelTexture};
use ash::vk;
use crate::render::buffer::Buffer;
use crate::render::render_context::RenderContext;
use crate::render::graphic_pipeline::{GraphicPipeline, PipelineVertexInputInfo, PipelineLayoutInfo};
use crate::render::swapchain_mgr::SwapChainMgr;
use crate::render::vertex;
use glam::{Mat4, Vec3};
use std::mem::size_of;
use crate::render::texture::Texture;

const UBO_BINDING: u32 = 0;
const COLOR_SAMPLER_BINDING: u32 = 1;

#[repr(C)]
#[derive(Clone, Debug, Copy)]
struct UniformBufferData {
    model: Mat4,
    view: Mat4,
    proj: Mat4,
}

impl UniformBufferData {
    pub fn create(window_width: u32, window_height: u32) -> UniformBufferData {
        UniformBufferData {
            model: Mat4::IDENTITY,
            view: Mat4::look_at_rh(
                Vec3::new(2.0, 2.0, 2.0),
                Vec3::new(0.0, 0.0, 0.0),
                Vec3::new(0.0, 0.0, 1.0),
            ),
            proj: Mat4::perspective_rh(
                45f32.to_radians() as _,
                window_width as f32
                    / window_height as f32,
                0.1,
                10.0,
            ),
        }
    }
}

fn create_uniform_descriptor_layout(context: &mut RenderContext) -> vk::DescriptorSetLayout {
    let bindings = [vk::DescriptorSetLayoutBinding::builder().
        binding(0).descriptor_count(1).descriptor_type(vk::DescriptorType::UNIFORM_BUFFER)
        .stage_flags(vk::ShaderStageFlags::VERTEX).build()];
    let descriptor_layout = unsafe {
        context.
            device.create_descriptor_set_layout(
            &vk::DescriptorSetLayoutCreateInfo::builder().bindings(&bindings).build(), None)
            .expect("create descriptor layout failed")
    };

    descriptor_layout
}

fn create_uniform_descriptor_sets(context: &mut RenderContext) {}

struct UniformObject {
    data: UniformBufferData,
    descriptor_set_layout: vk::DescriptorSetLayout,
    descriptor_set: vk::DescriptorSet,
}

impl UniformObject {
    pub fn destroy(&mut self, context: &RenderContext) {
        unsafe {
            context.device.free_descriptor_sets(context.descriptor_pool, &[self.descriptor_set]);
            context.device.destroy_descriptor_set_layout(self.descriptor_set_layout, None);
        }
    }

    pub fn create(context: &mut RenderContext, data: UniformBufferData) -> UniformObject
    {
        let uniform_buffer = Buffer::create_host_visible_buffer(context, vk::BufferUsageFlags::UNIFORM_BUFFER, &[data]);

        let descriptor_set_layout = create_uniform_descriptor_layout(context);

        let descriptor_alloc_ci = vk::DescriptorSetAllocateInfo::builder().descriptor_pool(context.descriptor_pool)
            .set_layouts(&[descriptor_set_layout]).build();

        let descriptor_set = unsafe
            { context.device.allocate_descriptor_sets(&descriptor_alloc_ci).expect("failed to allocate descriptor sets") }[0];

        let descriptor_buffer_info = vk::DescriptorBufferInfo::builder()
            .buffer(uniform_buffer.buffer).offset(0).range(size_of::<UniformBufferData>() as _).build();

        let descriptor_write_info = vk::WriteDescriptorSet::builder().descriptor_type(vk::DescriptorType::UNIFORM_BUFFER)
            .dst_set(descriptor_set).dst_binding(0).buffer_info(&[descriptor_buffer_info]).build();

        unsafe {
            context.device.update_descriptor_sets(&[descriptor_write_info], &[]);
        }

        UniformObject {
            data,
            descriptor_set,
            descriptor_set_layout,
        }
    }
}


pub struct ModelRenderer {
    model: Model,
    pipeline: GraphicPipeline,
    uniform: UniformObject,
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
        self.uniform.destroy(context);
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

    pub fn create(context: &mut RenderContext, swapchain_mgr: &SwapChainMgr, render_pass: vk::RenderPass,
                  command_buffer: vk::CommandBuffer, gltf_path: &str, vert_shader_path: &str, frag_shader_path: &str) -> ModelRenderer {
        let vertex_input = PipelineVertexInputInfo::from(&vertex::VERTEX_BINDING_DESCS[..],
                                                         &vertex::VERTEX_ATTRIBUTE_DESCS[..]);

        println!("start load model {}", gltf_path);
        let model = Model::from_gltf(context, command_buffer, gltf_path).expect("load error");
        println!("load model {} complete", gltf_path);
        let uniform = UniformObject::create(context, UniformBufferData::create(context.window_width, context.window_height));

        let (descriptor_set_layout, descriptor_sets) = Self::create_model_descriptors(context, &model);

        let all_layout = [uniform.descriptor_set_layout, descriptor_set_layout];

        let pipeline_layout_ci = vk::PipelineLayoutCreateInfo::builder().set_layouts(&all_layout).build();

        let pipeline = GraphicPipeline::create(context,
                                               swapchain_mgr,
                                               render_pass,
                                               &vertex_input, &pipeline_layout_ci, context.render_config.msaa,
                                               vert_shader_path, frag_shader_path);

        ModelRenderer {
            pipeline,
            model,
            uniform,
            descriptor_set_layout,
            descriptor_sets,
        }
    }

    pub fn draw(&self, context: &RenderContext, command_buffer: vk::CommandBuffer) {
        unsafe {
            context.device.cmd_bind_pipeline(command_buffer, vk::PipelineBindPoint::GRAPHICS, self.pipeline.get_pipeline());

            let mut set_idx = 0;
            context.device.cmd_bind_vertex_buffers(command_buffer, 0, &[self.model.get_vertex_buffer().buffer], &[0]);
            //todo index type
            context.device.cmd_bind_index_buffer(command_buffer, self.model.get_vertex_buffer().buffer, 0, vk::IndexType::UINT32);
            for mesh in self.model.get_meshes() {
                for primitive in mesh.primitives() {
                    let set = self.descriptor_sets[set_idx];
                    context.device.cmd_bind_descriptor_sets(command_buffer,
                                                            vk::PipelineBindPoint::GRAPHICS,
                                                            self.pipeline.get_layout(),
                                                            0,
                                                            &[self.uniform.descriptor_set, set], &[]);

                    context.device.cmd_draw_indexed(command_buffer, primitive.indices.1 as _, 1, primitive.indices.0 as _, 0, 0);
                    set_idx += 1;
                }
            }
        }
    }
}