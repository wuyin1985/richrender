use crate::render::texture::Texture;
use ash::vk;
use crate::render::render_context::{RenderContext, RenderConfig};
use crate::render::swapchain_mgr::SwapChainMgr;
use ash::vk::ImageLayout;

pub struct ForwardRenderPass {
    color_texture: Texture,
    color_view: vk::ImageView,
    depth_texture: Texture,
    depth_view: vk::ImageView,
    resolve_texture: Option<Texture>,
    resolve_view: Option<vk::ImageView>,
    render_pass: vk::RenderPass,
    frame_buffer: vk::Framebuffer,
}

impl ForwardRenderPass {
    pub fn destroy(&mut self, device_mgr: &RenderContext) {
        unsafe {
            if let Some(rt) = self.resolve_texture.as_mut() {
                rt.destroy(device_mgr);
                device_mgr.device.destroy_image_view(self.resolve_view.unwrap(), None);
            }

            self.color_texture.destroy(device_mgr);
            device_mgr.device.destroy_image_view(self.color_view, None);

            self.depth_texture.destroy(device_mgr);
            device_mgr.device.destroy_image_view(self.depth_view, None);

            device_mgr.device.destroy_framebuffer(self.frame_buffer, None);
            device_mgr.device.destroy_render_pass(self.render_pass, None);
        }
    }

    pub fn create(device_mgr: &RenderContext, swap_chain_mgr: &SwapChainMgr) -> Self {
        unsafe {
            let render_config = &device_mgr.render_config;
            let msaa_on = render_config.msaa != vk::SampleCountFlags::TYPE_1;
            let msaa = render_config.msaa;

            let color_texture =
                Texture::create_as_render_target(device_mgr, device_mgr.window_width,
                                                       device_mgr.window_height, render_config.color_format,
                                                       msaa,
                                                       vk::ImageUsageFlags::COLOR_ATTACHMENT | vk::ImageUsageFlags::STORAGE | vk::ImageUsageFlags::TRANSFER_SRC,
                                                       "color_render_texture", vk::ImageCreateFlags::empty());

            let color_view = color_texture.create_color_view(device_mgr);

            let depth_texture =
                Texture::create_as_depth_stencil(device_mgr, device_mgr.window_width,
                                                       device_mgr.window_height, render_config.depth_format,
                                                       vk::SampleCountFlags::TYPE_1,
                                                       "color_render_texture");
            let depth_view = depth_texture.create_depth_view(device_mgr);

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
                // depth
                // vk::AttachmentDescription {
                //     flags: Default::default(),
                //     format: render_config.depth_format,
                //     samples: msaa,
                //     load_op: vk::AttachmentLoadOp::CLEAR,
                //     store_op: vk::AttachmentStoreOp::DONT_CARE,
                //     stencil_load_op: vk::AttachmentLoadOp::DONT_CARE,
                //     stencil_store_op: vk::AttachmentStoreOp::DONT_CARE,
                //     initial_layout: vk::ImageLayout::UNDEFINED,
                //     final_layout: vk::ImageLayout::DEPTH_STENCIL_ATTACHMENT_OPTIMAL,
                // },
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

            // let depth_attachment_ref = vk::AttachmentReference {
            //     attachment: 1,
            //     layout: vk::ImageLayout::DEPTH_STENCIL_ATTACHMENT_OPTIMAL,
            // };

            let dependencies = [vk::SubpassDependency {
                src_subpass: vk::SUBPASS_EXTERNAL,
                dst_subpass: 0,
                src_stage_mask: vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT,
                src_access_mask: vk::AccessFlags::empty(),
                dst_stage_mask: vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT,
                dst_access_mask: vk::AccessFlags::COLOR_ATTACHMENT_WRITE,
                dependency_flags: vk::DependencyFlags::empty()
            }];

            let mut subpass_builder = vk::SubpassDescription::builder().color_attachments(&color_attachment_refs)
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
            let render_pass = device_mgr.device.create_render_pass(&renderpass_create_info, None).unwrap();

            let mut frame_buffer_views = vec![
                color_view,
                //depth_view,
            ];

            let mut resolve_texture: Option<Texture> = None;

            let mut resolve_view: Option<vk::ImageView> = None;

            if msaa_on {
                let l_resolve_texture =
                    Texture::create_as_render_target(device_mgr, device_mgr.window_width,
                                                           device_mgr.window_height, render_config.color_format,
                                                           vk::SampleCountFlags::TYPE_1,
                                                           vk::ImageUsageFlags::COLOR_ATTACHMENT | vk::ImageUsageFlags::STORAGE,
                                                           "resolve_texture", vk::ImageCreateFlags::empty());


                let l_resolve_view = l_resolve_texture.create_color_view(device_mgr);
                frame_buffer_views.push(l_resolve_view);

                resolve_texture = Some(l_resolve_texture);
                resolve_view = Some(l_resolve_view);
            }

            let frame_buffer_ci = vk::FramebufferCreateInfo::builder().render_pass(render_pass).layers(1).
                width(device_mgr.window_width).height(device_mgr.window_height).attachments(&frame_buffer_views).build();

            let frame_buffer = device_mgr.device.create_framebuffer(&frame_buffer_ci, None).unwrap();

            ForwardRenderPass {
                depth_texture,
                depth_view,
                color_texture,
                color_view,
                render_pass,
                frame_buffer,
                resolve_texture,
                resolve_view,
            }
        }
    }

    pub fn get_native_render_pass(&self) -> vk::RenderPass {
        self.render_pass
    }

    pub fn begin_render_pass(&self, device_mgr: &RenderContext, swapchain_mgr: &SwapChainMgr, command_buffer: vk::CommandBuffer) {
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
                    width: device_mgr.window_width,
                    height: device_mgr.window_height,
                },
            })
            .clear_values(&clear_values)
            .build();

        unsafe {
            device_mgr.device.cmd_begin_render_pass(
                command_buffer,
                &render_pass_begin_info,
                vk::SubpassContents::INLINE,
            )
        };
    }

    pub fn end_pass(&self, device_mgr: &RenderContext, command_buffer: vk::CommandBuffer) {
        unsafe {
            device_mgr.device.cmd_end_render_pass(command_buffer);
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