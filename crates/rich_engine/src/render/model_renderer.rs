use crate::render::model::{Model, ModelTexture};
use ash::vk;
use crate::render::buffer::Buffer;
use crate::render::render_context::{RenderContext, PerFrameData, DummyResources};
use crate::render::graphic_pipeline::{GraphicPipeline, PipelineVertexInputInfo, PipelineLayoutInfo};
use crate::render::swapchain_mgr::SwapChainMgr;
use crate::render::{vertex, util};
use std::mem::size_of;
use crate::render::texture::Texture;
use bevy::prelude::*;
use crate::render::uniform::UniformObject;
use crate::render::aabb::Aabb;
use crate::render::animation_system::AnimationRuntime;
use crate::render::animation::Animations;
use crate::render::gltf_asset_loader::{GltfAsset};
use crate::render::material::Material;
use crate::render::vertex_layout::VertexLayout;
use crate::render::mesh::Primitive;
use crate::render::forward_render::ForwardRenderPass;
use crate::render::node::{Node, Nodes};

const UBO_BINDING: u32 = 0;
const COLOR_SAMPLER_BINDING: u32 = 1;

pub struct ShadeNames {
    pub vertex: &'static str,
    pub frag: &'static str,
    pub shadow_vertex: &'static str,
    pub shadow_frag: &'static str,
}


#[repr(C)]
#[derive(Clone, Debug, Copy)]
pub struct ModelData {
    pub transform: Mat4,
}

impl Default for ModelData {
    fn default() -> Self {
        Self {
            transform: Mat4::IDENTITY,
        }
    }
}

struct AnimationWithNodes {
    animations: Animations,
    nodes: Nodes,
}

pub struct ModelRenderer {
    model: Model,
    primitive_renders: Vec<PrimitiveRender>,
    anim_nodes: Option<AnimationWithNodes>,
}

impl ModelRenderer {
    pub fn destroy(&mut self, context: &mut RenderContext) {
        let mut rs = std::mem::take(&mut self.primitive_renders);
        for r in &mut rs {
            r.destroy(context);
        }
        self.model.destroy(context);
    }

    pub fn get_center_transform(model_aabb: Aabb) -> Mat4 {
        let aabb = model_aabb * Mat4::from_rotation_x(0f32.to_radians());
        let CAMERA_DIS: f32 = 1.0;
        let CAMERA_FOV: f32 = 45f32.to_radians() / 2f32;
        let target_z = (aabb.max.z - aabb.min.z) / 2f32;
        let target_y = (aabb.max.y - aabb.min.y) / 2f32;

        let offset_x = (aabb.max.x + aabb.min.x) / 2f32;
        let offset_z = (aabb.max.z + aabb.min.z) / 2f32;
        let offset_y = (aabb.max.y + aabb.min.y) / 2f32;

        assert!(target_y > 0f32, "error model half y");
        let to_z = -1f32;
        let desire_y = CAMERA_FOV.tan() * (CAMERA_DIS - to_z);
        let s = desire_y / target_y;
        //let scale = 1f32;

        let scale = Vec3::new(s, s, s);
        let rot = Quat::IDENTITY;
        let pos = Vec3::new(-offset_x, 0f32, to_z);
        return Mat4::from_scale_rotation_translation(scale, rot, pos);
    }

    pub fn create(context: &mut RenderContext, swapchain_mgr: &SwapChainMgr, render_pass: &ForwardRenderPass,
                  command_buffer: vk::CommandBuffer, gltf_asset: &GltfAsset, shader_names: &ShadeNames) -> ModelRenderer {
        let model = Model::from_gltf(context, command_buffer, gltf_asset).expect("load error");

        let mut primitive_renders = Vec::new();
        for node in model.get_nodes() {
            if let Some(mesh_idx) = node.mesh_index() {
                let mesh = &model.get_meshes()[mesh_idx];
                for primitive in mesh.primitives() {
                    let r = PrimitiveRender::create(context, swapchain_mgr, render_pass, primitive, &model, shader_names);
                    primitive_renders.push(r);
                }
            }
        }

        let mut anim_nodes = None;

        if let Some(anim) = model.clone_animations() {
            anim_nodes = Some(AnimationWithNodes { animations: anim, nodes: model.clone_nodes() });
        }

        ModelRenderer {
            primitive_renders,
            model,
            anim_nodes,
        }
    }

    fn get_nodes<'a>(&'a self, runtime: &'a AnimationRuntime) -> &'a [Node] {
        if let Some(anim_nodes) = runtime.data.as_ref() {
            return anim_nodes.nodes.nodes();
        }

        self.model.get_nodes()
    }

    pub fn draw_shadow(&self, context: &RenderContext, command_buffer: vk::CommandBuffer, model_data: &ModelData, runtime: &AnimationRuntime) {
        let mut primitive_idx = 0;
        let uniform = context.per_frame_uniform.as_ref().unwrap();
        for node in self.get_nodes(runtime) {
            if let Some(mesh_idx) = node.mesh_index() {
                let node_transform = node.transform();
                let m_data = ModelData { transform: model_data.transform * node_transform };
                let model_data_bytes: &[u8] = unsafe { util::any_as_u8_slice(&m_data) };

                let mesh = &self.model.get_meshes()[mesh_idx];
                for primitive in mesh.primitives() {
                    let render = &self.primitive_renders[primitive_idx];
                    let vertex_layout = &primitive.get_vertex_layout();
                    primitive_idx += 1;
                    let set = render.descriptor_set;
                    unsafe {
                        context.device.cmd_bind_pipeline(command_buffer, vk::PipelineBindPoint::GRAPHICS, render.shadow_pipeline.get_pipeline());

                        context.device.cmd_push_constants(command_buffer, render.shadow_pipeline.get_layout(),
                                                          vk::ShaderStageFlags::VERTEX, 0, model_data_bytes);

                        context.device.cmd_bind_vertex_buffers(command_buffer,
                                                               0,
                                                               &render.buffers_ref_for_draw,
                                                               &vertex_layout.buffers_ref_offsets);
                        context.device.cmd_bind_index_buffer(command_buffer,
                                                             self.model.get_buffer().buffer,
                                                             vertex_layout.indices.index as _,
                                                             vertex_layout.indices_type);

                        let mut descriptor_sets = vec![uniform.descriptor_set];
                        if self.model.has_animation() {
                            let set = runtime.data.as_ref().unwrap().skin_descriptor_set;
                            descriptor_sets.push(set);
                        }

                        context.device.cmd_bind_descriptor_sets(command_buffer,
                                                                vk::PipelineBindPoint::GRAPHICS,
                                                                render.shadow_pipeline.get_layout(),
                                                                0,
                                                                &descriptor_sets, &[]);

                        context.device.cmd_draw_indexed(command_buffer, vertex_layout.indices.count as _, 1, 0, 0, 0);
                    }
                }
            }
        }
    }

    pub fn draw(&self, context: &RenderContext, command_buffer: vk::CommandBuffer, model_data: &ModelData, runtime: &AnimationRuntime) {
        let mut primitive_idx = 0;
        let uniform = context.per_frame_uniform.as_ref().unwrap();

        for node in self.get_nodes(runtime) {
            if let Some(mesh_idx) = node.mesh_index() {
                let m_data = ModelData { transform: model_data.transform * node.transform() };
                let model_data_bytes: &[u8] = unsafe { util::any_as_u8_slice(&m_data) };

                let mesh = &self.model.get_meshes()[mesh_idx];
                for primitive in mesh.primitives() {
                    let render = &self.primitive_renders[primitive_idx];
                    let vertex_layout = &primitive.get_vertex_layout();
                    primitive_idx += 1;
                    let set = render.descriptor_set;
                    unsafe {
                        context.device.cmd_bind_pipeline(command_buffer, vk::PipelineBindPoint::GRAPHICS, render.graphic_pipeline.get_pipeline());

                        context.device.cmd_push_constants(command_buffer, render.graphic_pipeline.get_layout(),
                                                          vk::ShaderStageFlags::VERTEX, 0, model_data_bytes);

                        context.device.cmd_bind_vertex_buffers(command_buffer,
                                                               0,
                                                               &render.buffers_ref_for_draw,
                                                               &vertex_layout.buffers_ref_offsets);
                        context.device.cmd_bind_index_buffer(command_buffer,
                                                             self.model.get_buffer().buffer,
                                                             vertex_layout.indices.index as _,
                                                             vertex_layout.indices_type);

                        let mut descriptor_sets = vec![uniform.descriptor_set, set];
                        if self.model.has_animation() {
                            let set = runtime.data.as_ref().unwrap().skin_descriptor_set;
                            descriptor_sets.push(set);
                        }

                        context.device.cmd_bind_descriptor_sets(command_buffer,
                                                                vk::PipelineBindPoint::GRAPHICS,
                                                                render.graphic_pipeline.get_layout(),
                                                                0,
                                                                &descriptor_sets, &[]);

                        context.device.cmd_draw_indexed(command_buffer, vertex_layout.indices.count as _, 1, 0, 0, 0);
                    }
                }
            }
        }
    }

    pub fn get_model(&self) -> &Model {
        &self.model
    }
}


struct PrimitiveRender {
    pub descriptor_set_layout: vk::DescriptorSetLayout,
    pub descriptor_set: vk::DescriptorSet,
    pub graphic_pipeline: GraphicPipeline,
    pub shadow_pipeline: GraphicPipeline,
    pub buffers_ref_for_draw: Vec<vk::Buffer>,
}

impl PrimitiveRender {
    fn create_descriptors(context: &mut RenderContext,
                          render_pass: &ForwardRenderPass,
                          material: &Material, model: &Model) -> (vk::DescriptorSetLayout, vk::DescriptorSet) {
        let mut bindings = vec![
            vk::DescriptorSetLayoutBinding::builder()
                .binding(0)
                .descriptor_type(vk::DescriptorType::COMBINED_IMAGE_SAMPLER)
                .descriptor_count(1)
                .stage_flags(vk::ShaderStageFlags::FRAGMENT)
                .build(),
            vk::DescriptorSetLayoutBinding::builder()
                .binding(1)
                .descriptor_type(vk::DescriptorType::COMBINED_IMAGE_SAMPLER)
                .descriptor_count(1)
                .stage_flags(vk::ShaderStageFlags::FRAGMENT)
                .build(),
        ];

        let layout_info = vk::DescriptorSetLayoutCreateInfo::builder().bindings(&bindings).build();

        let set_layout = unsafe {
            context.device
                .create_descriptor_set_layout(&layout_info, None)
                .unwrap()
        };

        let layouts = [set_layout];

        let allocate_info = vk::DescriptorSetAllocateInfo::builder()
            .descriptor_pool(context.descriptor_pool)
            .set_layouts(&layouts);
        let set = unsafe {
            context
                .device
                .allocate_descriptor_sets(&allocate_info)
                .unwrap()[0]
        };

        let textures = model.get_textures();
        let texture = material.get_color_texture_index().map_or_else(|| {
            let dr = context.get_resource::<DummyResources>();
            &dr.white_texture
        }, |idx| &textures[idx]);

        let albedo_info = {
            let (view, sampler) = (texture.view, texture.sampler);

            [vk::DescriptorImageInfo::builder()
                .image_layout(vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL)
                .image_view(view)
                .sampler(sampler)
                .build()]
        };

        let shadow_info = {
            let shadow = render_pass.get_shadow();
            let (view, sampler) = (shadow.shadow_view, shadow.sampler);

            [vk::DescriptorImageInfo::builder()
                .image_layout(vk::ImageLayout::DEPTH_STENCIL_READ_ONLY_OPTIMAL)
                .image_view(view)
                .sampler(sampler)
                .build()]
        };

        let descriptor_writes = [
            vk::WriteDescriptorSet::builder()
                .dst_set(set)
                .dst_binding(0)
                .descriptor_type(vk::DescriptorType::COMBINED_IMAGE_SAMPLER)
                .image_info(&albedo_info)
                .build(),
            vk::WriteDescriptorSet::builder()
                .dst_set(set)
                .dst_binding(1)
                .descriptor_type(vk::DescriptorType::COMBINED_IMAGE_SAMPLER)
                .image_info(&shadow_info)
                .build(),
        ];

        unsafe {
            context
                .device
                .update_descriptor_sets(&descriptor_writes, &[])
        }


        (set_layout, set)
    }

    pub fn destroy(self: &mut Self, context: &mut RenderContext)
    {
        self.graphic_pipeline.destroy(context);
        self.shadow_pipeline.destroy(context);
        unsafe {
            context.device.free_descriptor_sets(context.descriptor_pool, &[self.descriptor_set]);
            context.device.destroy_descriptor_set_layout(self.descriptor_set_layout, None);
        }
    }

    pub fn create(context: &mut RenderContext,
                  swapchain_mgr: &SwapChainMgr,
                  render_pass: &ForwardRenderPass,
                  primitive: &Primitive,
                  model: &Model,
                  shader_names: &ShadeNames,
    ) -> Self {
        let vertex_layout = primitive.get_vertex_layout();
        let material = primitive.get_material();
        let vertex_bindings = vertex_layout.build_vk_bindings();
        let vertex_attributes = vertex_layout.build_vk_attributes();
        let vertex_input = PipelineVertexInputInfo::from(&vertex_bindings, &vertex_attributes);
        let mut shader_defines = vertex_layout.get_shader_defines();
        if model.has_animation() {
            shader_defines.push("SKIN");
        }

        let buffers_ref_for_draw = (0..vertex_bindings.len()).map(|_| model.get_buffer().buffer).collect::<Vec<_>>();
        let frame_uniform_layout = context.per_frame_uniform.as_mut().unwrap().descriptor_set_layout;

        let (descriptor_set_layout, descriptor_set) = Self::create_descriptors(context, render_pass, &material, model);

        let mut all_layout = vec![frame_uniform_layout, descriptor_set_layout];

        if model.has_animation() {
            all_layout.push(context.skin_buffer_mgr.descriptor_set_layout);
        }

        let constant_ranges = [
            vk::PushConstantRange::builder().offset(0).size(size_of::<ModelData>() as _).stage_flags(vk::ShaderStageFlags::VERTEX).build()
        ];

        let pipeline_layout_ci = vk::PipelineLayoutCreateInfo::builder().set_layouts(&all_layout).push_constant_ranges(&constant_ranges).build();


        let graphic_pipeline = GraphicPipeline::create(context,
                                                       swapchain_mgr,
                                                       render_pass.get_native_render_pass(),
                                                       &vertex_input, &pipeline_layout_ci, context.render_config.msaa,
                                                       shader_names.vertex, shader_names.frag, &shader_defines);

        let mut shadow_layout = vec![frame_uniform_layout];
        if model.has_animation() {
            shadow_layout.push(context.skin_buffer_mgr.descriptor_set_layout);
        }
        let shadow_layout_ci = vk::PipelineLayoutCreateInfo::builder()
            .set_layouts(&shadow_layout)
            .push_constant_ranges(&constant_ranges)
            .build();
        let shadow_pipeline = GraphicPipeline::create_vert_only(context,
                                                                swapchain_mgr,
                                                                render_pass.get_shadow_render_pass(),
                                                                &vertex_input,
                                                                &shadow_layout_ci,
                                                                context.render_config.msaa,
                                                                shader_names.shadow_vertex,
                                                                &shader_defines);

        Self {
            graphic_pipeline,
            shadow_pipeline,
            descriptor_set_layout,
            descriptor_set,
            buffers_ref_for_draw,
        }
    }
}