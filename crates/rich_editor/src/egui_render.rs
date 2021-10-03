use std::ffi::{c_void, CString};
use std::include_bytes;
use std::sync::Arc;
use std::time::Instant;

use rich_engine::ash::{extensions::khr::Swapchain, vk, Device};
use bytemuck::bytes_of;
use egui::{
    math::{pos2, vec2},
    paint::ClippedShape,
    CtxRef, Key,
};
use rich_engine::{Buffer, ForwardRenderPass, RenderContext, Texture};

pub struct FontRes {
    image: Texture,
    image_view: vk::ImageView,
}

/// egui integration with winit, ash and vk_mem.
pub struct EguiRender {
    physical_width: u32,
    physical_height: u32,
    scale_factor: f64,
    egui_ctx: CtxRef,

    descriptor_set_layout: vk::DescriptorSetLayout,
    pipeline_layout: vk::PipelineLayout,
    pipeline: vk::Pipeline,
    sampler: vk::Sampler,
    render_pass: vk::RenderPass,
    framebuffer: vk::Framebuffer,
    vertex_buffer: Buffer,
    index_buffer: Buffer,

    font: Option<FontRes>,
    font_image_version: u64,
    font_descriptor_sets: Vec<vk::DescriptorSet>,

    user_texture_layout: vk::DescriptorSetLayout,
    user_textures: Vec<Option<vk::DescriptorSet>>,
}


impl EguiRender {
    /// Create an instance of the integration.
    pub fn new(
        physical_width: u32,
        physical_height: u32,
        scale_factor: f64,
        egui_ctx: CtxRef,
        surface_format: vk::Format,
        context: &mut RenderContext,
        forward: &ForwardRenderPass,
    ) -> Self {
        // Create DescriptorSetLayouts
        let descriptor_set_layout = {
            unsafe {
                context.device.create_descriptor_set_layout(
                    &vk::DescriptorSetLayoutCreateInfo::builder().bindings(&[
                        vk::DescriptorSetLayoutBinding::builder()
                            .descriptor_type(vk::DescriptorType::COMBINED_IMAGE_SAMPLER)
                            .descriptor_count(1)
                            .binding(0)
                            .stage_flags(vk::ShaderStageFlags::FRAGMENT)
                            .build(),
                    ]),
                    None,
                )
            }.expect("Failed to create descriptor set layout.")
        };

        let descriptor_set_layouts = [descriptor_set_layout];

        // Create RenderPass
        let render_pass = unsafe {
            context.device.create_render_pass(
                &vk::RenderPassCreateInfo::builder()
                    .attachments(&[vk::AttachmentDescription::builder()
                        .format(surface_format)
                        .samples(vk::SampleCountFlags::TYPE_1)
                        .load_op(vk::AttachmentLoadOp::LOAD)
                        .store_op(vk::AttachmentStoreOp::STORE)
                        .stencil_load_op(vk::AttachmentLoadOp::DONT_CARE)
                        .stencil_store_op(vk::AttachmentStoreOp::DONT_CARE)
                        .initial_layout(vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL)
                        .final_layout(vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL)
                        .build()])
                    .subpasses(&[vk::SubpassDescription::builder()
                        .pipeline_bind_point(vk::PipelineBindPoint::GRAPHICS)
                        .color_attachments(&[vk::AttachmentReference::builder()
                            .attachment(0)
                            .layout(vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL)
                            .build()])
                        .build()])
                    .dependencies(&[vk::SubpassDependency::builder()
                        .src_subpass(vk::SUBPASS_EXTERNAL)
                        .dst_subpass(0)
                        .src_access_mask(vk::AccessFlags::COLOR_ATTACHMENT_WRITE)
                        .dst_access_mask(vk::AccessFlags::COLOR_ATTACHMENT_WRITE)
                        .src_stage_mask(vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT)
                        .dst_stage_mask(vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT)
                        .build()]),
                None,
            )
        }.expect("Failed to create render pass.");

        // Create PipelineLayout
        let pipeline_layout = unsafe {
            context.device.create_pipeline_layout(
                &vk::PipelineLayoutCreateInfo::builder()
                    .set_layouts(&descriptor_set_layouts)
                    .push_constant_ranges(&[vk::PushConstantRange::builder()
                        .stage_flags(vk::ShaderStageFlags::VERTEX)
                        .offset(0)
                        .size(std::mem::size_of::<f32>() as u32 * 2) // screen size
                        .build()]),
                None,
            )
        }.expect("Failed to create pipeline layout.");

        // Create Pipeline
        let pipeline = {
            let bindings = [vk::VertexInputBindingDescription::builder()
                .binding(0)
                .input_rate(vk::VertexInputRate::VERTEX)
                .stride(
                    4 * std::mem::size_of::<f32>() as u32 + 4 * std::mem::size_of::<u8>() as u32,
                )
                .build()];

            let attributes = [
                // position
                vk::VertexInputAttributeDescription::builder()
                    .binding(0)
                    .offset(0)
                    .location(0)
                    .format(vk::Format::R32G32_SFLOAT)
                    .build(),
                // uv
                vk::VertexInputAttributeDescription::builder()
                    .binding(0)
                    .offset(8)
                    .location(1)
                    .format(vk::Format::R32G32_SFLOAT)
                    .build(),
                // color
                vk::VertexInputAttributeDescription::builder()
                    .binding(0)
                    .offset(16)
                    .location(2)
                    .format(vk::Format::R8G8B8A8_UNORM)
                    .build(),
            ];

            let vertex_shader_module = {
                let bytes_code = include_bytes!("shaders/spv/vert.spv");
                let shader_module_create_info = vk::ShaderModuleCreateInfo {
                    code_size: bytes_code.len(),
                    p_code: bytes_code.as_ptr() as *const u32,
                    ..Default::default()
                };
                unsafe { context.device.create_shader_module(&shader_module_create_info, None) }
                    .expect("Failed to create vertex shader module.")
            };
            let fragment_shader_module = {
                let bytes_code = include_bytes!("shaders/spv/frag.spv");
                let shader_module_create_info = vk::ShaderModuleCreateInfo {
                    code_size: bytes_code.len(),
                    p_code: bytes_code.as_ptr() as *const u32,
                    ..Default::default()
                };
                unsafe { context.device.create_shader_module(&shader_module_create_info, None) }
                    .expect("Failed to create fragment shader module.")
            };
            let main_function_name = CString::new("main").unwrap();
            let pipeline_shader_stages = [
                vk::PipelineShaderStageCreateInfo::builder()
                    .stage(vk::ShaderStageFlags::VERTEX)
                    .module(vertex_shader_module)
                    .name(&main_function_name)
                    .build(),
                vk::PipelineShaderStageCreateInfo::builder()
                    .stage(vk::ShaderStageFlags::FRAGMENT)
                    .module(fragment_shader_module)
                    .name(&main_function_name)
                    .build(),
            ];

            let input_assembly_info = vk::PipelineInputAssemblyStateCreateInfo::builder()
                .topology(vk::PrimitiveTopology::TRIANGLE_LIST);
            let viewport_info = vk::PipelineViewportStateCreateInfo::builder()
                .viewport_count(1)
                .scissor_count(1);
            let rasterization_info = vk::PipelineRasterizationStateCreateInfo::builder()
                .depth_clamp_enable(false)
                .rasterizer_discard_enable(false)
                .polygon_mode(vk::PolygonMode::FILL)
                .cull_mode(vk::CullModeFlags::NONE)
                .front_face(vk::FrontFace::COUNTER_CLOCKWISE)
                .depth_bias_enable(false)
                .line_width(1.0);
            let stencil_op = vk::StencilOpState::builder()
                .fail_op(vk::StencilOp::KEEP)
                .pass_op(vk::StencilOp::KEEP)
                .compare_op(vk::CompareOp::ALWAYS)
                .build();
            let depth_stencil_info = vk::PipelineDepthStencilStateCreateInfo::builder()
                .depth_test_enable(false)
                .depth_write_enable(false)
                .depth_compare_op(vk::CompareOp::ALWAYS)
                .depth_bounds_test_enable(false)
                .stencil_test_enable(false)
                .front(stencil_op)
                .back(stencil_op);
            let color_blend_attachments = [vk::PipelineColorBlendAttachmentState::builder()
                .color_write_mask(
                    vk::ColorComponentFlags::R
                        | vk::ColorComponentFlags::G
                        | vk::ColorComponentFlags::B
                        | vk::ColorComponentFlags::A,
                )
                .blend_enable(true)
                .src_color_blend_factor(vk::BlendFactor::ONE)
                .dst_color_blend_factor(vk::BlendFactor::ONE_MINUS_SRC_ALPHA)
                .build()];
            let color_blend_info = vk::PipelineColorBlendStateCreateInfo::builder()
                .attachments(&color_blend_attachments);
            let dynamic_states = [vk::DynamicState::VIEWPORT, vk::DynamicState::SCISSOR];
            let dynamic_state_info =
                vk::PipelineDynamicStateCreateInfo::builder().dynamic_states(&dynamic_states);
            let vertex_input_state = vk::PipelineVertexInputStateCreateInfo::builder()
                .vertex_attribute_descriptions(&attributes)
                .vertex_binding_descriptions(&bindings);
            let multisample_info = vk::PipelineMultisampleStateCreateInfo::builder()
                .rasterization_samples(vk::SampleCountFlags::TYPE_1);

            let pipeline_create_info = [vk::GraphicsPipelineCreateInfo::builder()
                .stages(&pipeline_shader_stages)
                .vertex_input_state(&vertex_input_state)
                .input_assembly_state(&input_assembly_info)
                .viewport_state(&viewport_info)
                .rasterization_state(&rasterization_info)
                .multisample_state(&multisample_info)
                .depth_stencil_state(&depth_stencil_info)
                .color_blend_state(&color_blend_info)
                .dynamic_state(&dynamic_state_info)
                .layout(pipeline_layout)
                .render_pass(render_pass)
                .subpass(0)
                .build()];

            let pipeline = unsafe {
                context.device.create_graphics_pipelines(
                    vk::PipelineCache::null(),
                    &pipeline_create_info,
                    None,
                )
            }
                .expect("Failed to create graphics pipeline.")[0];
            unsafe {
                context.device.destroy_shader_module(vertex_shader_module, None);
                context.device.destroy_shader_module(fragment_shader_module, None);
            }
            pipeline
        };

        // Create Sampler
        let sampler = unsafe {
            context.device.create_sampler(
                &vk::SamplerCreateInfo::builder()
                    .address_mode_u(vk::SamplerAddressMode::CLAMP_TO_EDGE)
                    .address_mode_v(vk::SamplerAddressMode::CLAMP_TO_EDGE)
                    .address_mode_w(vk::SamplerAddressMode::CLAMP_TO_EDGE)
                    .anisotropy_enable(false)
                    .min_filter(vk::Filter::LINEAR)
                    .mag_filter(vk::Filter::LINEAR)
                    .mipmap_mode(vk::SamplerMipmapMode::LINEAR)
                    .min_lod(0.0)
                    .max_lod(vk::LOD_CLAMP_NONE),
                None,
            )
        }.expect("Failed to create sampler.");

        // Create Framebuffer
        let target_image_view = forward.get_final_render_image_view();
        let framebuffer = unsafe {
            let attachments = &[target_image_view];
            context.device
                .create_framebuffer(
                    &vk::FramebufferCreateInfo::builder()
                        .render_pass(render_pass)
                        .attachments(attachments)
                        .width(physical_width)
                        .height(physical_height)
                        .layers(1),
                    None,
                ).expect("Failed to create framebuffer.")
        };

        // Create vertex buffer and index buffer
        let vertex_buffer = Buffer::create_host_visible_buffer_with_size(context, vk::BufferUsageFlags::VERTEX_BUFFER, Self::vertex_buffer_size() as _);
        let index_buffer = Buffer::create_host_visible_buffer_with_size(context, vk::BufferUsageFlags::INDEX_BUFFER, Self::index_buffer_size() as _);

        // User Textures
        let user_texture_layout = unsafe {
            context.device.create_descriptor_set_layout(
                &vk::DescriptorSetLayoutCreateInfo::builder().bindings(&[
                    vk::DescriptorSetLayoutBinding::builder()
                        .descriptor_type(vk::DescriptorType::COMBINED_IMAGE_SAMPLER)
                        .descriptor_count(1)
                        .binding(0)
                        .stage_flags(vk::ShaderStageFlags::FRAGMENT)
                        .build(),
                ]),
                None,
            )
        }.expect("Failed to create descriptor set layout.");

        let user_textures = vec![];

        let font_descriptor_sets = unsafe {
            context.device.allocate_descriptor_sets(
                &vk::DescriptorSetAllocateInfo::builder()
                    .descriptor_pool(context.descriptor_pool)
                    .set_layouts(&descriptor_set_layouts),
            )
        }.expect("failed to create sets");

        Self {
            physical_width,
            physical_height,
            scale_factor,
            egui_ctx,
            //clipboard,
            descriptor_set_layout,
            pipeline_layout,
            pipeline,
            sampler,
            render_pass,
            framebuffer,
            vertex_buffer,
            index_buffer,
            font: None,
            font_image_version: 0,
            font_descriptor_sets,
            user_texture_layout,
            user_textures,
        }
    }

    // vertex buffer size
    fn vertex_buffer_size() -> u64 {
        1024 * 1024 * 4
    }

    // index buffer size
    fn index_buffer_size() -> u64 {
        1024 * 1024 * 2
    }

    /// Get [`egui::CtxRef`].
    pub fn egui_ctx(&self) -> CtxRef {
        self.egui_ctx.clone()
    }

    pub fn prepare(&mut self, context: &mut RenderContext, command_buffer:vk::CommandBuffer) {
        // update font texture
        self.upload_font_texture(context, command_buffer, &self.egui_ctx.fonts().texture());
    }

    /// Record paint commands.
    pub fn paint(
        &mut self,
        context: &mut RenderContext,
        command_buffer: vk::CommandBuffer,
        clipped_meshes: Vec<egui::ClippedMesh>,
    ) {

        // map buffers
        let mut vertex_buffer_ptr = self.vertex_buffer.map_memory(context);
        let vertex_buffer_ptr_end =
            unsafe { vertex_buffer_ptr.add(Self::vertex_buffer_size() as usize) };
        let mut index_buffer_ptr = self.index_buffer.map_memory(context);
        let index_buffer_ptr_end =
            unsafe { index_buffer_ptr.add(Self::index_buffer_size() as usize) };

        // begin render pass
        unsafe {
            context.device.cmd_begin_render_pass(
                command_buffer,
                &vk::RenderPassBeginInfo::builder()
                    .render_pass(self.render_pass)
                    .framebuffer(self.framebuffer)
                    .clear_values(&[])
                    .render_area(
                        vk::Rect2D::builder()
                            .extent(
                                vk::Extent2D::builder()
                                    .width(self.physical_width)
                                    .height(self.physical_height)
                                    .build(),
                            )
                            .build(),
                    ),
                vk::SubpassContents::INLINE,
            );
        }

        // bind resources
        unsafe {
            context.device.cmd_bind_pipeline(
                command_buffer,
                vk::PipelineBindPoint::GRAPHICS,
                self.pipeline,
            );
            context.device.cmd_bind_vertex_buffers(
                command_buffer,
                0,
                &[self.vertex_buffer.buffer],
                &[0],
            );
            context.device.cmd_bind_index_buffer(
                command_buffer,
                self.index_buffer.buffer,
                0,
                vk::IndexType::UINT32,
            );
            context.device.cmd_set_viewport(
                command_buffer,
                0,
                &[vk::Viewport::builder()
                    .x(0.0)
                    .y(0.0)
                    .width(self.physical_width as f32)
                    .height(self.physical_height as f32)
                    .min_depth(0.0)
                    .max_depth(1.0)
                    .build()],
            );
            let width_points = self.physical_width as f32 / self.scale_factor as f32;
            let height_points = self.physical_height as f32 / self.scale_factor as f32;
            context.device.cmd_push_constants(
                command_buffer,
                self.pipeline_layout,
                vk::ShaderStageFlags::VERTEX,
                0,
                bytes_of(&width_points),
            );
            context.device.cmd_push_constants(
                command_buffer,
                self.pipeline_layout,
                vk::ShaderStageFlags::VERTEX,
                std::mem::size_of_val(&width_points) as u32,
                bytes_of(&height_points),
            );
        }

        // render meshes
        let mut vertex_base = 0;
        let mut index_base = 0;
        for egui::ClippedMesh(rect, mesh) in clipped_meshes {
            // update texture
            unsafe {
                if let egui::TextureId::User(id) = mesh.texture_id {
                    if let Some(descriptor_set) = self.user_textures[id as usize] {
                        context.device.cmd_bind_descriptor_sets(
                            command_buffer,
                            vk::PipelineBindPoint::GRAPHICS,
                            self.pipeline_layout,
                            0,
                            &[descriptor_set],
                            &[],
                        );
                    } else {
                        eprintln!(
                            "This UserTexture has already been unregistered: {:?}",
                            mesh.texture_id
                        );
                        continue;
                    }
                } else {
                    context.device.cmd_bind_descriptor_sets(
                        command_buffer,
                        vk::PipelineBindPoint::GRAPHICS,
                        self.pipeline_layout,
                        0,
                        &self.font_descriptor_sets,
                        &[],
                    );
                }
            }

            if mesh.vertices.is_empty() || mesh.indices.is_empty() {
                continue;
            }

            let v_slice = &mesh.vertices;
            let v_size = std::mem::size_of_val(&v_slice[0]);
            let v_copy_size = v_slice.len() * v_size;

            let i_slice = &mesh.indices;
            let i_size = std::mem::size_of_val(&i_slice[0]);
            let i_copy_size = i_slice.len() * i_size;

            let vertex_buffer_ptr_next = unsafe { vertex_buffer_ptr.add(v_copy_size) };
            let index_buffer_ptr_next = unsafe { index_buffer_ptr.add(i_copy_size) };

            if vertex_buffer_ptr_next >= vertex_buffer_ptr_end
                || index_buffer_ptr_next >= index_buffer_ptr_end
            {
                panic!("egui paint out of memory");
            }

            // map memory
            unsafe { vertex_buffer_ptr.copy_from(v_slice.as_ptr() as *const c_void, v_copy_size) };
            unsafe { index_buffer_ptr.copy_from(i_slice.as_ptr() as *const c_void, i_copy_size) };

            vertex_buffer_ptr = vertex_buffer_ptr_next;
            index_buffer_ptr = index_buffer_ptr_next;

            // record draw commands
            unsafe {
                let min = rect.min;
                let min = egui::Pos2 {
                    x: min.x * self.scale_factor as f32,
                    y: min.y * self.scale_factor as f32,
                };
                let min = egui::Pos2 {
                    x: f32::clamp(min.x, 0.0, self.physical_width as f32),
                    y: f32::clamp(min.y, 0.0, self.physical_height as f32),
                };
                let max = rect.max;
                let max = egui::Pos2 {
                    x: max.x * self.scale_factor as f32,
                    y: max.y * self.scale_factor as f32,
                };
                let max = egui::Pos2 {
                    x: f32::clamp(max.x, min.x, self.physical_width as f32),
                    y: f32::clamp(max.y, min.y, self.physical_height as f32),
                };
                context.device.cmd_set_scissor(
                    command_buffer,
                    0,
                    &[vk::Rect2D::builder()
                        .offset(
                            vk::Offset2D::builder()
                                .x(min.x.round() as i32)
                                .y(min.y.round() as i32)
                                .build(),
                        )
                        .extent(
                            vk::Extent2D::builder()
                                .width((max.x.round() - min.x) as u32)
                                .height((max.y.round() - min.y) as u32)
                                .build(),
                        )
                        .build()],
                );
                context.device.cmd_draw_indexed(
                    command_buffer,
                    mesh.indices.len() as u32,
                    1,
                    index_base,
                    vertex_base,
                    0,
                );
            }

            vertex_base += mesh.vertices.len() as i32;
            index_base += mesh.indices.len() as u32;
        }

        // end render pass
        unsafe {
            context.device.cmd_end_render_pass(command_buffer);
        }
    }

    fn upload_font_texture(&mut self, context: &mut RenderContext, command_buffer: vk::CommandBuffer, texture: &egui::Texture) {
        debug_assert_eq!(texture.pixels.len(), texture.width * texture.height);
        // check version
        if texture.version == self.font_image_version {
            return;
        }

        if let Some(ft) = &mut self.font {
            ft.image.destroy(context);
            unsafe {
                context.device.destroy_image_view(ft.image_view, None);
            }
        }

        let dimensions = (texture.width as u64, texture.height as u64);
        let data = texture
            .pixels
            .iter()
            .flat_map(|&r| vec![r, r, r, r])
            .collect::<Vec<_>>();


        // create font image
        let image = Texture::create_from_rgba(context, command_buffer, dimensions.0 as _, dimensions.1 as _, &data);
        let image_view = unsafe {
            context.device.create_image_view(
                &vk::ImageViewCreateInfo::builder()
                    .image(image.get_image())
                    .format(vk::Format::R8G8B8A8_UNORM)
                    .view_type(vk::ImageViewType::TYPE_2D)
                    .subresource_range(
                        vk::ImageSubresourceRange::builder()
                            .aspect_mask(vk::ImageAspectFlags::COLOR)
                            .base_array_layer(0)
                            .base_mip_level(0)
                            .layer_count(1)
                            .level_count(1)
                            .build(),
                    ),
                None,
            )
        }.expect("Failed to create image view.");
        self.font_image_version = texture.version;

        // update descriptor set
        for &font_descriptor_set in self.font_descriptor_sets.iter() {
            unsafe {
                context.device.update_descriptor_sets(
                    &[vk::WriteDescriptorSet::builder()
                        .descriptor_type(vk::DescriptorType::COMBINED_IMAGE_SAMPLER)
                        .dst_set(font_descriptor_set)
                        .image_info(&[vk::DescriptorImageInfo::builder()
                            .image_view(image_view)
                            .image_layout(vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL)
                            .sampler(self.sampler)
                            .build()])
                        .dst_binding(0)
                        .build()],
                    &[],
                );
            }
        }

        self.font = Some(FontRes {
            image,
            image_view,
        });
    }

    pub unsafe fn destroy(&mut self, context: &mut RenderContext) {
        context.device
            .destroy_descriptor_set_layout(self.user_texture_layout, None);

        if let Some(ft) = &mut self.font {
            ft.image.destroy(context);
            context.device.destroy_image_view(ft.image_view, None);
        }

        self.vertex_buffer.destroy(context);
        self.index_buffer.destroy(context);

        context.device.destroy_framebuffer(self.framebuffer, None);
        context.device.destroy_render_pass(self.render_pass, None);
        context.device.destroy_sampler(self.sampler, None);
        context.device.destroy_pipeline(self.pipeline, None);
        context.device
            .destroy_pipeline_layout(self.pipeline_layout, None);
        context.device.free_descriptor_sets(context.descriptor_pool, &self.font_descriptor_sets);
        context.device
            .destroy_descriptor_set_layout(self.descriptor_set_layout, None);
    }
}
