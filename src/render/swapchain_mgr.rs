use ash::vk;
use crate::render::device_mgr::DeviceMgr;
use ash::vk::ImageView;

pub struct SwapChainMgr {
    swapchain: vk::SwapchainKHR,
    present_images: Vec<vk::Image>,
    present_image_views: Vec<vk::ImageView>,
    render_pass: vk::RenderPass,
    frame_buffers: Vec<vk::Framebuffer>,

    cmd_buf_execute_fences: Vec<vk::Fence>,
    image_available_semaphores: Vec<vk::Semaphore>,
    render_finish_semaphores: Vec<vk::Semaphore>,

    image_index_to_present: usize,
    semaphore_index: usize,
    prev_semaphore_index: usize,
    pub format: ash::vk::Format,
}

impl SwapChainMgr {
    pub unsafe fn create(device: &DeviceMgr, window_width: u32, window_height: u32) -> Self {
        let surface_loader = &device.surface_loader;
        let surface_capabilities = surface_loader
            .get_physical_device_surface_capabilities(device.physical_device, device.surface)
            .unwrap();
        let mut desired_image_count = surface_capabilities.min_image_count + 1;
        if surface_capabilities.max_image_count > 0
            && desired_image_count > surface_capabilities.max_image_count
        {
            desired_image_count = surface_capabilities.max_image_count;
        }

        let surface_format = surface_loader
            .get_physical_device_surface_formats(device.physical_device, device.surface)
            .unwrap()[0];

        let pre_transform = if surface_capabilities
            .supported_transforms
            .contains(vk::SurfaceTransformFlagsKHR::IDENTITY)
        {
            vk::SurfaceTransformFlagsKHR::IDENTITY
        } else {
            surface_capabilities.current_transform
        };
        let present_modes = device.surface_loader
            .get_physical_device_surface_present_modes(device.physical_device, device.surface)
            .unwrap();
        let present_mode = present_modes
            .iter()
            .cloned()
            .find(|&mode| mode == vk::PresentModeKHR::FIFO)
            .expect("failed to find present mode FIFO");

        let surface_resolution = match surface_capabilities.current_extent.width {
            std::u32::MAX => vk::Extent2D {
                width: window_width,
                height: window_height,
            },
            _ => surface_capabilities.current_extent,
        };

        let swapchain_create_info = vk::SwapchainCreateInfoKHR::builder()
            .surface(device.surface)
            .min_image_count(desired_image_count)
            .image_color_space(surface_format.color_space)
            .image_format(surface_format.format)
            .image_extent(surface_resolution)
            .image_usage(vk::ImageUsageFlags::COLOR_ATTACHMENT)
            .image_sharing_mode(vk::SharingMode::EXCLUSIVE)
            .pre_transform(pre_transform)
            .composite_alpha(vk::CompositeAlphaFlagsKHR::OPAQUE)
            .present_mode(present_mode)
            .clipped(true)
            .image_array_layers(1);

        let swapchain = device.swapchain_loader
            .create_swapchain(&swapchain_create_info, None)
            .unwrap();

        let present_images = device.swapchain_loader.get_swapchain_images(swapchain).unwrap();

        let present_image_views = present_images.iter().map(|&image| {
            let create_view_info = vk::ImageViewCreateInfo::builder().
                view_type(vk::ImageViewType::TYPE_2D).format(surface_format.format).components(vk::ComponentMapping {
                r: vk::ComponentSwizzle::R,
                g: vk::ComponentSwizzle::G,
                b: vk::ComponentSwizzle::B,
                a: vk::ComponentSwizzle::A,
            }).subresource_range(vk::ImageSubresourceRange {
                aspect_mask: vk::ImageAspectFlags::COLOR,
                base_mip_level: 0,
                level_count: 1,
                base_array_layer: 0,
                layer_count: 1,
            }).image(image);

            device.device.create_image_view(&create_view_info, None).unwrap()
        }).collect();

        let render_pass = SwapChainMgr::create_render_pass(surface_format.format, device);
        let frame_buffers = SwapChainMgr::create_frame_buffers(device, &present_image_views, &render_pass);
        let mut cmd_buf_execute_fences = vec![];
        let mut image_available_semaphores = vec![];
        let mut render_finish_semaphores = vec![];
        for i in 0..present_images.len() {
            let mut flag = vk::FenceCreateFlags::default();
            if i == 0 {
                flag = vk::FenceCreateFlags::SIGNALED;
            }
            let fence_ci = vk::FenceCreateInfo {
                flags: flag,
                ..Default::default()
            };

            cmd_buf_execute_fences.push(device.device.create_fence(&fence_ci, None).unwrap());
            let semaphore_ci = vk::SemaphoreCreateInfo::builder().build();
            image_available_semaphores.push(device.device.create_semaphore(&semaphore_ci, None).unwrap());
            render_finish_semaphores.push(device.device.create_semaphore(&semaphore_ci, None).unwrap());
        }

        SwapChainMgr {
            format: surface_format.format,
            swapchain,
            present_images,
            present_image_views,
            render_pass,
            frame_buffers,
            cmd_buf_execute_fences,
            image_available_semaphores,
            render_finish_semaphores,
            image_index_to_present: 0,
            semaphore_index: 0,
            prev_semaphore_index: 0,
        }
    }

    pub fn destroy(&mut self, device: &DeviceMgr) {
        unsafe {
            self.present_image_views.iter().for_each(|&image_view| {
                device.device.destroy_image_view(image_view, None);
            });
            device.swapchain_loader.destroy_swapchain(self.swapchain, None);
        }
    }

    pub fn get_present_image_count(&self) -> u32 {
        self.present_images.len() as u32
    }

    pub fn wait_for_swap_chain(&mut self, device_mgr: &DeviceMgr) -> u32 {
        unsafe {
            let (present_index, success) = device_mgr.swapchain_loader.
                acquire_next_image(self.swapchain, std::u64::MAX,
                                   self.image_available_semaphores[self.semaphore_index], vk::Fence::null()).unwrap();
            assert!(success, "not success");
            self.prev_semaphore_index = self.semaphore_index;
            self.semaphore_index += 1;
            if self.semaphore_index >= self.present_images.len() {
                self.semaphore_index = 0;
            }

            device_mgr.device.wait_for_fences(&[self.cmd_buf_execute_fences[self.prev_semaphore_index]], true, std::u64::MAX).expect("wait fence failed");
            device_mgr.device.reset_fences(&[self.cmd_buf_execute_fences[self.prev_semaphore_index]]).expect("reset fence failed");

            present_index
        }
    }

    pub fn present(&self, device_mgr: &DeviceMgr) {
        unsafe {
            let present_ci = vk::PresentInfoKHR::builder().wait_semaphores(&[self.render_finish_semaphores[self.semaphore_index]]).
                swapchains(&[self.swapchain]).image_indices(&[self.image_index_to_present as u32]).build();
            let res = device_mgr.swapchain_loader.queue_present(device_mgr.present_queue, &present_ci).unwrap();
            assert!(res, "queue present failed");
        }
    }

    pub fn get_semaphores(&self, p_image_available_semaphore: &mut vk::Semaphore, p_render_finish_semaphore: &mut vk::Semaphore, p_cmd_execute_fence: &mut vk::Fence) {
        *p_image_available_semaphore = self.image_available_semaphores[self.prev_semaphore_index];
        *p_render_finish_semaphore = self.render_finish_semaphores[self.semaphore_index];
        *p_cmd_execute_fence = self.cmd_buf_execute_fences[self.semaphore_index];
    }

    fn create_render_pass(format: vk::Format, device_mgr: &DeviceMgr) -> vk::RenderPass {
        let renderpass_attachments = [
            vk::AttachmentDescription {
                format: format,
                samples: vk::SampleCountFlags::TYPE_1,
                load_op: vk::AttachmentLoadOp::DONT_CARE,
                store_op: vk::AttachmentStoreOp::STORE,
                stencil_load_op: vk::AttachmentLoadOp::DONT_CARE,
                stencil_store_op: vk::AttachmentStoreOp::DONT_CARE,
                initial_layout: vk::ImageLayout::UNDEFINED,
                final_layout: vk::ImageLayout::PRESENT_SRC_KHR,
                ..Default::default()
            }
        ];
        let subpasses = [
            vk::SubpassDescription::builder().color_attachments(&[vk::AttachmentReference {
                layout: vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL,
                attachment: 0,
            }]).build()
        ];

        let dependencies = [vk::SubpassDependency {
            src_subpass: vk::SUBPASS_EXTERNAL,
            src_stage_mask: vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT,
            src_access_mask: vk::AccessFlags::COLOR_ATTACHMENT_WRITE,
            dst_subpass: 0,
            dst_stage_mask: vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT,
            dst_access_mask: vk::AccessFlags::COLOR_ATTACHMENT_READ | vk::AccessFlags::COLOR_ATTACHMENT_WRITE,
            ..Default::default()
        }];

        let renderpass_create_info = vk::RenderPassCreateInfo::builder().
            attachments(&renderpass_attachments).dependencies(&dependencies).subpasses(&subpasses).build();

        unsafe {
            device_mgr.device.create_render_pass(&renderpass_create_info, None).unwrap()
        }
    }

    fn create_frame_buffers(device_mgr: &DeviceMgr, present_image_views: &Vec<vk::ImageView>, render_pass: &vk::RenderPass) -> Vec<vk::Framebuffer> {
        present_image_views.iter().map(|&view| {
            let frame_buffer_create_info = vk::FramebufferCreateInfo::builder().
                render_pass(*render_pass).attachments(&[view]).width(device_mgr.window_width).height(device_mgr.window_height).layers(1).build();
            unsafe {
                device_mgr.device.create_framebuffer(&frame_buffer_create_info, None).unwrap()
            }
        }).collect()
    }
}
