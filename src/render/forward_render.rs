use crate::render::render_texture::RenderTexture;
use ash::vk;
use crate::render::device_mgr::DeviceMgr;
use crate::render::swapchain_mgr::SwapChainMgr;
use ash::vk::ImageLayout;

pub struct ForwardRenderConfig {
    msaa: vk::SampleCountFlags,
    apply_post_effect: bool,
    apply_shadow: bool,
    color_format: vk::Format,
    depth_format: vk::Format,
}

struct ForwardRender {
    color_texture: RenderTexture,
    color_view: vk::ImageView,
    depth_texture: RenderTexture,
    depth_view: vk::ImageView,
    resolve_texture: Option<RenderTexture>,
    resolve_view: Option<vk::ImageView>,
    render_pass: vk::RenderPass,
    frame_buffer: vk::Framebuffer,
}

impl ForwardRender {
    pub fn destroy(&mut self, device_mgr: &DeviceMgr) {
        unsafe {
            if let Some(&mut rt) = self.resolve_texture {
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

    pub fn create(device_mgr: &DeviceMgr, swap_chain_mgr: &SwapChainMgr, render_config: &ForwardRenderConfig) -> Self {
        unsafe {
            let msaa_on = render_config.msaa != vk::SampleCountFlags::TYPE_1;
            let msaa = render_config.msaa;

            let color_texture =
                RenderTexture::create_as_render_target(device_mgr, device_mgr.window_width,
                                                       device_mgr.window_height, render_config.color_format,
                                                       mass,
                                                       vk::ImageUsageFlags::COLOR_ATTACHMENT | vk::ImageUsageFlags::STORAGE,
                                                       "color_render_texture", vk::ImageCreateFlags::empty());

            let color_view = color_texture.create_color_view(device_mgr);

            let depth_texture =
                RenderTexture::create_as_depth_stencil(device_mgr, device_mgr.window_width,
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
                    load_op: vk::AttachmentLoadOp::DONT_CARE,
                    store_op: vk::AttachmentStoreOp::STORE,
                    stencil_load_op: vk::AttachmentLoadOp::DONT_CARE,
                    stencil_store_op: vk::AttachmentStoreOp::DONT_CARE,
                    initial_layout: vk::ImageLayout::UNDEFINED,
                    final_layout: vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL,
                },
                // depth
                vk::AttachmentDescription {
                    flags: Default::default(),
                    format: render_config.depth_format,
                    samples: msaa,
                    load_op: vk::AttachmentLoadOp::DONT_CARE,
                    store_op: vk::AttachmentStoreOp::DONT_CARE,
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

            let dependencies = [vk::SubpassDependency {
                src_subpass: vk::SUBPASS_EXTERNAL,
                src_stage_mask: vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT,
                dst_access_mask: vk::AccessFlags::COLOR_ATTACHMENT_WRITE,
                dst_stage_mask: vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT,
                ..Default::default()
            }];

            let mut subpass_builder = vk::SubpassDescription::builder().color_attachments(&color_attachment_refs)
                .depth_stencil_attachment(&depth_attachment_ref).pipeline_bind_point(vk::PipelineBindPoint::GRAPHICS);

            if msaa_on {
                subpass_builder = subpass_builder.resolve_attachments(&[vk::AttachmentReference {
                    attachment: 2,
                    layout: vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL,
                }]);
            }

            let subpasses = [subpass_builder.build()];

            let renderpass_create_info = vk::RenderPassCreateInfo::builder()
                .attachments(&renderpass_attachment).subpasses(&subpasses).dependencies(&dependencies).build();
            let renderpass = device_mgr.device.create_render_pass(&renderpass_create_info, None).unwrap();

            let mut frame_buffer_views = vec![
                color_view,
                depth_view,
            ];

            let mut resolve_texture: Option<RenderTexture> = None;

            let mut resolve_view: Option<vk::ImageView> = None;

            if msaa_on {
                resolve_texture =
                    Some(RenderTexture::create_as_render_target(device_mgr, device_mgr.window_width,
                                                                device_mgr.window_height, render_config.color_format,
                                                                vk::SampleCountFlags::TYPE_1,
                                                                vk::ImageUsageFlags::COLOR_ATTACHMENT | vk::ImageUsageFlags::STORAGE,
                                                                "resolve_texture", vk::ImageCreateFlags::empty()));


                resolve_view = Some(resolve_texture.unwrap().create_color_view(device_mgr));

                frame_buffer_views.push(resolve_view.unwrap());
            }

            let frame_buffer_ci = vk::FramebufferCreateInfo::builder().render_pass(renderpass).layers(1).
                width(device_mgr.window_width).height(device_mgr.window_height).attachments(&frame_buffer_views).build();

            let frame_buffer = device_mgr.device.create_framebuffer(&frame_buffer_ci, None).unwrap();

            ForwardRender {
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
}