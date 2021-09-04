use crate::render::device_mgr::DeviceMgr;
use crate::render::swapchain_mgr::SwapChainMgr;
use bevy::winit::WinitWindows;
use crate::render::forward_render::ForwardRenderPass;
use crate::render::simple_draw_object::SimpleDrawObject;
use crate::render::command_buffer_list::CommandBufferList;
use ash::vk;

pub struct RenderContext {
    device_mgr: DeviceMgr,
    swapchain_mgr: SwapChainMgr,
    command_buffer_list: CommandBufferList,
    forward_render_pass: ForwardRenderPass,
    simple_draw_object: SimpleDrawObject,
}

impl Drop for RenderContext {
    fn drop(&mut self) {
        unsafe { self.device_mgr.device.device_wait_idle().unwrap(); }
        self.simple_draw_object.destroy(&self.device_mgr);
        self.command_buffer_list.destroy(&self.device_mgr);
        self.forward_render_pass.destroy(&self.device_mgr);
        self.swapchain_mgr.destroy(&self.device_mgr);
        self.device_mgr.destroy();
    }
}

impl RenderContext {
    pub fn create<W: raw_window_handle::HasRawWindowHandle>(window: &W, window_width: u32, window_height: u32) -> Self {
        unsafe {
            let device = DeviceMgr::create(window, window_width, window_height);
            let swapchain = SwapChainMgr::create(&device, window_width, window_height);
            let command_buffer_list = CommandBufferList::create(swapchain.get_present_image_count(), &device);
            let forward_render_pass = ForwardRenderPass::create(&device, &swapchain);
            let simple_draw_object = SimpleDrawObject::create(&device,
                                                              &swapchain, forward_render_pass.get_native_render_pass());
            RenderContext {
                device_mgr: device,
                swapchain_mgr: swapchain,
                command_buffer_list,
                forward_render_pass,
                simple_draw_object,
            }
        }
    }

    pub fn draw(&mut self) {
        unsafe {
            let (success, present_index) = self.swapchain_mgr.wait_for_swap_chain(&mut self.device_mgr);
            if !success {
                return;
            }
            println!("require image {}", present_index);

            let command_buffer = self.command_buffer_list.get_command_buffer(present_index);
            {
                let command_buffer_begin_info = vk::CommandBufferBeginInfo::builder().
                    flags(vk::CommandBufferUsageFlags::ONE_TIME_SUBMIT).build();
                self.device_mgr.device.begin_command_buffer(command_buffer, &command_buffer_begin_info);
            }

            self.forward_render_pass.begin_render_pass(&mut self.device_mgr, &mut self.swapchain_mgr, command_buffer);
            self.simple_draw_object.draw(&mut self.device_mgr, command_buffer);
            self.forward_render_pass.end_pass(&mut self.device_mgr, command_buffer);


            let mut present_image_available_semaphore: vk::Semaphore = vk::Semaphore::null();
            let mut render_finish_semaphore: vk::Semaphore = vk::Semaphore::null();
            let mut cmd_buf_execute_fence: vk::Fence = vk::Fence::null();

            self.swapchain_mgr.get_semaphores(&mut present_image_available_semaphore,
                                              &mut render_finish_semaphore, &mut cmd_buf_execute_fence);


            {
                let image_barriers = [
                    vk::ImageMemoryBarrier::builder().image(self.forward_render_pass.get_final_render_image())
                        .src_access_mask(vk::AccessFlags::MEMORY_WRITE).dst_access_mask(vk::AccessFlags::MEMORY_READ)
                        .old_layout(vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL)
                        .new_layout(vk::ImageLayout::TRANSFER_SRC_OPTIMAL)
                        .subresource_range(vk::ImageSubresourceRange {
                            aspect_mask: vk::ImageAspectFlags::COLOR,
                            base_mip_level: 0,
                            level_count: 1,
                            base_array_layer: 0,
                            layer_count: 1,
                        }).build(),
                    vk::ImageMemoryBarrier::builder().image(self.swapchain_mgr.get_current_present_image())
                        .src_access_mask(vk::AccessFlags::MEMORY_WRITE).dst_access_mask(vk::AccessFlags::MEMORY_READ)
                        .old_layout(vk::ImageLayout::UNDEFINED)
                        .new_layout(vk::ImageLayout::TRANSFER_DST_OPTIMAL)
                        .subresource_range(vk::ImageSubresourceRange {
                            aspect_mask: vk::ImageAspectFlags::COLOR,
                            base_mip_level: 0,
                            level_count: 1,
                            base_array_layer: 0,
                            layer_count: 1,
                        }).build(),
                ];

                self.device_mgr.device.cmd_pipeline_barrier(command_buffer, vk::PipelineStageFlags::ALL_COMMANDS,
                                                            vk::PipelineStageFlags::ALL_COMMANDS,
                                                            vk::DependencyFlags::empty(), &[], &[],
                                                            &image_barriers);
            }


            let surface_resolution = self.swapchain_mgr.surface_resolution;

            self.device_mgr.device.cmd_copy_image(command_buffer, self.forward_render_pass.get_final_render_image(), vk::ImageLayout::TRANSFER_SRC_OPTIMAL,
                                                  self.swapchain_mgr.get_current_present_image(), vk::ImageLayout::TRANSFER_DST_OPTIMAL,
                                                  &[vk::ImageCopy {
                                                      src_subresource: vk::ImageSubresourceLayers {
                                                          aspect_mask: vk::ImageAspectFlags::COLOR,
                                                          layer_count: 1,
                                                          ..Default::default()
                                                      },
                                                      dst_subresource: vk::ImageSubresourceLayers {
                                                          aspect_mask: vk::ImageAspectFlags::COLOR,
                                                          layer_count: 1,
                                                          ..Default::default()
                                                      },
                                                      src_offset: Default::default(),
                                                      dst_offset: Default::default(),
                                                      extent: vk::Extent3D {
                                                          width: surface_resolution.width,
                                                          height: surface_resolution.height,
                                                          depth: 1,
                                                      },
                                                  }]);

            {
                let image_barriers = [
                    vk::ImageMemoryBarrier::builder().image(self.swapchain_mgr.get_current_present_image())
                        .src_access_mask(vk::AccessFlags::MEMORY_WRITE).dst_access_mask(vk::AccessFlags::MEMORY_READ)
                        .old_layout(vk::ImageLayout::TRANSFER_DST_OPTIMAL)
                        .new_layout(vk::ImageLayout::PRESENT_SRC_KHR)
                        .subresource_range(vk::ImageSubresourceRange {
                            aspect_mask: vk::ImageAspectFlags::COLOR,
                            base_mip_level: 0,
                            level_count: 1,
                            base_array_layer: 0,
                            layer_count: 1,
                        }).build(),
                ];

                self.device_mgr.device.cmd_pipeline_barrier(command_buffer, vk::PipelineStageFlags::ALL_COMMANDS,
                                                            vk::PipelineStageFlags::ALL_COMMANDS,
                                                            vk::DependencyFlags::empty(), &[], &[],
                                                            &image_barriers);
            }

            unsafe {
                self.device_mgr.device.end_command_buffer(command_buffer);
            }

            let submit_info = vk::SubmitInfo::builder()
                .wait_semaphores(&[present_image_available_semaphore]).wait_dst_stage_mask(&[vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT])
                .command_buffers(&[command_buffer]).signal_semaphores(&[render_finish_semaphore]).build();

            self.device_mgr.device.queue_submit(self.device_mgr.graphics_queue, &[submit_info], cmd_buf_execute_fence);

            self.swapchain_mgr.present(&self.device_mgr);
            println!("present success!")
        }
    }

    fn on_window_size_changed(&mut self, window_width: u32, window_height: u32) {
        //self.swapchain.refresh(&self.device, window_width, window_height);
    }
}
