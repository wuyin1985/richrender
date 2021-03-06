use crate::render::texture::Texture;
use ash::vk;
use crate::render::render_context::{RenderContext, RenderConfig};
use crate::render::swapchain_mgr::SwapChainMgr;
use ash::vk::{ImageLayout, ImageView};
use crate::render::model_renderer::ModelRenderer;
use crate::render::command_buffer_list::CommandBufferList;

pub struct ForwardRenderPass {
    color_texture: Texture,
    color_view: vk::ImageView,
    depth_texture: Texture,
    depth_view: vk::ImageView,
    resolve_texture: Option<Texture>,
    resolve_view: Option<vk::ImageView>,
    render_pass: vk::RenderPass,
    frame_buffer: vk::Framebuffer,
    shadow: ShadowPass,
}

pub struct ShadowPass {
    pub shadow_texture: Texture,
    pub shadow_view: ImageView,
    pub shadow_pass: vk::RenderPass,
    pub shadow_buffer: vk::Framebuffer,
    pub sampler: vk::Sampler,
}

impl ShadowPass {
    pub fn destroy(&mut self, context: &RenderContext) {
        self.shadow_texture.destroy(context);
        let device = &context.device;
        unsafe {
            device.destroy_sampler(self.sampler, None);
            device.destroy_image_view(self.shadow_view, None);
            device.destroy_framebuffer(self.shadow_buffer, None);
            device.destroy_render_pass(self.shadow_pass, None);
        }
    }
}

impl ForwardRenderPass {
    pub fn destroy(&mut self, context: &RenderContext) {
        let device = &context.device;
        if let Some(rt) = self.resolve_texture.as_mut() {
            rt.destroy(context);
            unsafe {
                device.destroy_image_view(self.resolve_view.unwrap(), None);
            }
        }

        self.color_texture.destroy(context);
        unsafe {
            device.destroy_image_view(self.color_view, None);
        }

        self.depth_texture.destroy(context);

        unsafe {
            device.destroy_image_view(self.depth_view, None);
            device.destroy_framebuffer(self.frame_buffer, None);
            device.destroy_render_pass(self.render_pass, None);
        }

        self.shadow.destroy(context);
    }

    pub fn create(context: &mut RenderContext, swap_chain_mgr: &SwapChainMgr, command_list: &CommandBufferList) -> Self {
        unsafe {
            let render_config = &context.render_config;
            let msaa_on = render_config.msaa != vk::SampleCountFlags::TYPE_1;
            let msaa = render_config.msaa;

            let color_texture =
                Texture::create_as_render_target(context, context.window_width,
                                                 context.window_height, render_config.color_format,
                                                 msaa,
                                                 vk::ImageUsageFlags::COLOR_ATTACHMENT | vk::ImageUsageFlags::STORAGE |
                                                     vk::ImageUsageFlags::TRANSFER_SRC | vk::ImageUsageFlags::SAMPLED,
                                                 "color_render_texture", vk::ImageCreateFlags::empty());

            let color_view = color_texture.create_color_view(context);

            let depth_texture =
                Texture::create_as_depth_stencil(context, context.window_width,
                                                 context.window_height, render_config.depth_format,
                                                 vk::SampleCountFlags::TYPE_1,
                                                 "color_render_texture");
            let depth_view = depth_texture.create_depth_view(context);

            let mut renderpass_attachment = vec![
                // render target
                vk::AttachmentDescription {
                    flags: Default::default(),
                    format: render_config.color_format,
                    samples: msaa,
                    load_op: vk::AttachmentLoadOp::CLEAR,
                    store_op: vk::AttachmentStoreOp::STORE,
                    stencil_load_op: vk::AttachmentLoadOp::DONT_CARE,
                    stencil_store_op: vk::AttachmentStoreOp::DONT_CARE,
                    initial_layout: vk::ImageLayout::UNDEFINED,
                    final_layout: vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL,
                },
                //depth
                vk::AttachmentDescription {
                    flags: Default::default(),
                    format: render_config.depth_format,
                    samples: msaa,
                    load_op: vk::AttachmentLoadOp::CLEAR,
                    store_op: vk::AttachmentStoreOp::STORE,
                    stencil_load_op: vk::AttachmentLoadOp::DONT_CARE,
                    stencil_store_op: vk::AttachmentStoreOp::DONT_CARE,
                    initial_layout: vk::ImageLayout::UNDEFINED,
                    final_layout: vk::ImageLayout::DEPTH_STENCIL_ATTACHMENT_OPTIMAL,
                },
            ];

            if msaa_on {
                // present as resolve
                renderpass_attachment.push(
                    vk::AttachmentDescription {
                        flags: Default::default(),
                        format: swap_chain_mgr.format,
                        samples: vk::SampleCountFlags::TYPE_1,
                        load_op: vk::AttachmentLoadOp::DONT_CARE,
                        store_op: vk::AttachmentStoreOp::STORE,
                        stencil_load_op: vk::AttachmentLoadOp::DONT_CARE,
                        stencil_store_op: vk::AttachmentStoreOp::DONT_CARE,
                        initial_layout: vk::ImageLayout::UNDEFINED,
                        final_layout: vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL,
                    }
                )
            }

            let color_attachment_refs = [vk::AttachmentReference {
                attachment: 0,
                layout: vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL,
            }];

            let depth_attachment_ref = vk::AttachmentReference {
                attachment: 1,
                layout: vk::ImageLayout::DEPTH_STENCIL_ATTACHMENT_OPTIMAL,
            };

            let dependencies = [
                vk::SubpassDependency {
                    src_subpass: vk::SUBPASS_EXTERNAL,
                    dst_subpass: 0,
                    src_stage_mask: vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT,
                    src_access_mask: vk::AccessFlags::empty(),
                    dst_stage_mask: vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT,
                    dst_access_mask: vk::AccessFlags::COLOR_ATTACHMENT_WRITE,
                    dependency_flags: vk::DependencyFlags::empty(),
                },
            ];

            let mut subpass_builder = vk::SubpassDescription::builder()
                .color_attachments(&color_attachment_refs)
                .depth_stencil_attachment(&depth_attachment_ref)
                .pipeline_bind_point(vk::PipelineBindPoint::GRAPHICS);

            if msaa_on {
                subpass_builder = subpass_builder.resolve_attachments(&[vk::AttachmentReference {
                    attachment: 2,
                    layout: vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL,
                }]);
            }

            let subpasses = [subpass_builder.build()];

            let renderpass_create_info = vk::RenderPassCreateInfo::builder()
                .attachments(&renderpass_attachment).subpasses(&subpasses).dependencies(&dependencies).build();
            let render_pass = context.device.create_render_pass(&renderpass_create_info, None).unwrap();

            let mut frame_buffer_views = vec![
                color_view,
                depth_view,
            ];

            let mut resolve_texture: Option<Texture> = None;

            let mut resolve_view: Option<vk::ImageView> = None;

            if msaa_on {
                let l_resolve_texture =
                    Texture::create_as_render_target(context, context.window_width,
                                                     context.window_height, render_config.color_format,
                                                     vk::SampleCountFlags::TYPE_1,
                                                     vk::ImageUsageFlags::COLOR_ATTACHMENT | vk::ImageUsageFlags::STORAGE,
                                                     "resolve_texture", vk::ImageCreateFlags::empty());


                let l_resolve_view = l_resolve_texture.create_color_view(context);
                frame_buffer_views.push(l_resolve_view);

                resolve_texture = Some(l_resolve_texture);
                resolve_view = Some(l_resolve_view);
            }

            let frame_buffer_ci = vk::FramebufferCreateInfo::builder().render_pass(render_pass).layers(1).
                width(context.window_width).height(context.window_height).attachments(&frame_buffer_views).build();

            let frame_buffer = context.device.create_framebuffer(&frame_buffer_ci, None).unwrap();

            let shadow = Self::create_shadow(context, msaa);

            ForwardRenderPass {
                depth_texture,
                depth_view,
                color_texture,
                color_view,
                render_pass,
                frame_buffer,
                resolve_texture,
                resolve_view,
                shadow,
            }
        }
    }

    fn create_shadow(context: &RenderContext, msaa: vk::SampleCountFlags) -> ShadowPass {
        let shadow_format = context.render_config.depth_format;
        let sd = context.render_config.shadow_map_dim;
        let shadow_texture = Texture::create_as_depth_stencil(context,
                                                              sd as _, sd as _,
                                                              shadow_format, msaa, "shadow map");

        let shadow_view = shadow_texture.create_depth_view(context);

        let sampler = {
            let sampler_info = vk::SamplerCreateInfo::builder()
                .mag_filter(vk::Filter::LINEAR)
                .min_filter(vk::Filter::LINEAR)
                .address_mode_u(vk::SamplerAddressMode::CLAMP_TO_BORDER)
                .address_mode_v(vk::SamplerAddressMode::CLAMP_TO_BORDER)
                .address_mode_w(vk::SamplerAddressMode::CLAMP_TO_BORDER)
                .anisotropy_enable(false)
                .max_anisotropy(1.0)
                .border_color(vk::BorderColor::FLOAT_OPAQUE_WHITE)
                .unnormalized_coordinates(false)
                .compare_enable(false)
                .compare_op(vk::CompareOp::NEVER)
                .mipmap_mode(vk::SamplerMipmapMode::LINEAR)
                .mip_lod_bias(0.0)
                .min_lod(0.0)
                .max_lod(0.0);

            unsafe {
                context
                    .device
                    .create_sampler(&sampler_info, None)
                    .expect("Failed to create sampler")
            }
        };


        let shadow_attachments = [
            vk::AttachmentDescription {
                format: shadow_format,
                samples: msaa,
                initial_layout: vk::ImageLayout::UNDEFINED,
                final_layout: vk::ImageLayout::DEPTH_STENCIL_READ_ONLY_OPTIMAL,
                load_op: vk::AttachmentLoadOp::CLEAR,
                store_op: vk::AttachmentStoreOp::STORE,
                ..Default::default()
            },
        ];

        let depth_ref = vk::AttachmentReference {
            attachment: 0,
            layout: vk::ImageLayout::DEPTH_STENCIL_ATTACHMENT_OPTIMAL,
        };

        let dependence = [
            vk::SubpassDependency::builder()
                .src_subpass(vk::SUBPASS_EXTERNAL)
                .src_stage_mask(vk::PipelineStageFlags::EARLY_FRAGMENT_TESTS | vk::PipelineStageFlags::LATE_FRAGMENT_TESTS)
                .src_access_mask(vk::AccessFlags::DEPTH_STENCIL_ATTACHMENT_WRITE)
                .dst_subpass(0)
                .dst_stage_mask(vk::PipelineStageFlags::FRAGMENT_SHADER | vk::PipelineStageFlags::EARLY_FRAGMENT_TESTS | vk::PipelineStageFlags::LATE_FRAGMENT_TESTS)
                .dst_access_mask(vk::AccessFlags::DEPTH_STENCIL_ATTACHMENT_WRITE).build(),
        ];

        let subpasses = [vk::SubpassDescription::builder().depth_stencil_attachment(&depth_ref).build()];

        let render_pass_ci = vk::RenderPassCreateInfo::builder().attachments(&shadow_attachments)
            .subpasses(&subpasses).dependencies(&dependence).build();

        let shadow_pass = unsafe { context.device.create_render_pass(&render_pass_ci, None).expect("failed to create shadow render pass") };

        let views = [shadow_view];
        let frame_buffer_ci = vk::FramebufferCreateInfo::builder()
            .attachments(&views)
            .width(sd as _)
            .render_pass(shadow_pass)
            .layers(1)
            .height(sd as _).build();

        let shadow_buffer = unsafe {
            context.device.create_framebuffer(&frame_buffer_ci, None).expect("failed to create shadow frame buffer")
        };


        ShadowPass {
            shadow_texture,
            shadow_view,
            shadow_buffer,
            shadow_pass,
            sampler,
        }
    }

    pub fn get_color_view(&self) -> vk::ImageView {
        self.color_view
    }

    pub fn get_color_texture(&self) -> &Texture {
        &self.color_texture
    }

    pub fn get_depth_view(&self) -> vk::ImageView {
        self.depth_view
    }

    pub fn get_depth_texture(&self) -> &Texture {
        &self.depth_texture
    }

    pub fn get_native_render_pass(&self) -> vk::RenderPass {
        self.render_pass
    }

    pub fn get_shadow_render_pass(&self) -> vk::RenderPass {
        self.shadow.shadow_pass
    }

    pub fn get_shadow(&self) -> &ShadowPass {
        &self.shadow
    }

    pub fn begin_shadow_pass(&self, context: &RenderContext, command_buffer: vk::CommandBuffer) {
        let clear_values = [
            vk::ClearValue {
                depth_stencil: vk::ClearDepthStencilValue {
                    depth: 1.0,
                    stencil: 0,
                },
            },
        ];

        let sd = context.render_config.shadow_map_dim;

        let render_pass_begin_info = vk::RenderPassBeginInfo::builder()
            .render_pass(self.shadow.shadow_pass)
            .framebuffer(self.shadow.shadow_buffer)
            .render_area(vk::Rect2D {
                offset: vk::Offset2D { x: 0, y: 0 },
                extent: vk::Extent2D {
                    width: sd as _,
                    height: sd as _,
                },
            })
            .clear_values(&clear_values)
            .build();

        unsafe {
            context.device.cmd_begin_render_pass(
                command_buffer,
                &render_pass_begin_info,
                vk::SubpassContents::INLINE,
            )
        };
    }

    pub fn end_shadow_pass(&self, context: &RenderContext, command_buffer: vk::CommandBuffer) {
        unsafe {
            context.device.cmd_end_render_pass(command_buffer);
        }
    }

    pub fn begin_render_pass(&self, context: &RenderContext, command_buffer: vk::CommandBuffer) {
        let clear_values = [
            vk::ClearValue {
                color: vk::ClearColorValue {
                    float32: [0.0, 0.0, 0.0, 1.0],
                },
            },
            vk::ClearValue {
                depth_stencil: vk::ClearDepthStencilValue {
                    depth: 1.0,
                    stencil: 0,
                },
            },
        ];

        let render_pass_begin_info = vk::RenderPassBeginInfo::builder()
            .render_pass(self.render_pass)
            .framebuffer(self.frame_buffer)
            .render_area(vk::Rect2D {
                offset: vk::Offset2D { x: 0, y: 0 },
                extent: vk::Extent2D {
                    width: context.window_width,
                    height: context.window_height,
                },
            })
            .clear_values(&clear_values)
            .build();

        unsafe {
            context.device.cmd_begin_render_pass(
                command_buffer,
                &render_pass_begin_info,
                vk::SubpassContents::INLINE,
            )
        };
    }

    pub fn end_render_pass(&self, context: &RenderContext, command_buffer: vk::CommandBuffer) {
        unsafe {
            context.device.cmd_end_render_pass(command_buffer);
        }
    }

    pub fn get_final_render_image_view(&self) -> vk::ImageView {
        match &self.resolve_texture {
            Some(rt) => {
                self.resolve_view.unwrap()
            }
            _ => {
                self.color_view
            }
        }
    }

    pub fn get_final_render_image(&self) -> vk::Image {
        match &self.resolve_texture {
            Some(rt) => {
                rt.get_image()
            }
            _ => {
                self.color_texture.get_image()
            }
        }
    }
}