use crate::render::render_context::RenderContext;
use crate::render::swapchain_mgr::SwapChainMgr;
use bevy::winit::WinitWindows;
use crate::render::forward_render::ForwardRenderPass;
use crate::render::command_buffer_list::CommandBufferList;
use ash::vk;
use std::time::SystemTime;
use crate::render::model::Model;
use crate::render::model_renderer::ModelRenderer;
use bevy::prelude::*;

pub struct RenderRunner {
    pub device_mgr: RenderContext,
    pub swapchain_mgr: SwapChainMgr,
    command_buffer_list: CommandBufferList,
    pub forward_render_pass: ForwardRenderPass,
    model_renderer: ModelRenderer,
    last_tick: SystemTime,
}

impl Drop for RenderRunner {
    fn drop(&mut self) {
        unsafe { self.device_mgr.device.device_wait_idle().unwrap(); }
        self.model_renderer.destroy(&self.device_mgr);
        self.command_buffer_list.destroy(&self.device_mgr);
        self.forward_render_pass.destroy(&self.device_mgr);
        self.swapchain_mgr.destroy(&self.device_mgr);
        self.device_mgr.destroy();
    }
}


impl RenderRunner {
    pub fn create<W: raw_window_handle::HasRawWindowHandle>(window: &W, window_width: u32, window_height: u32) -> Self {
        unsafe {
            info!("start up");
            let mut device = RenderContext::create(window, window_width, window_height);
            info!("render context create complete");
            let swapchain = SwapChainMgr::create(&device, window_width, window_height);
            let command_buffer_list = CommandBufferList::create(swapchain.get_present_image_count(), &device);
            let forward_render_pass = ForwardRenderPass::create(&mut device, &swapchain, &command_buffer_list);
            info!("forward render pass create complete");

            // unsafe {
            //     let p = device.instance.get_physical_device_image_format_properties(device.physical_device,
            //                                                                         vk::Format::R8G8B8_USCALED, vk::ImageType::TYPE_2D, vk::ImageTiling::OPTIMAL,
            //                                                                         vk::ImageUsageFlags::SAMPLED, vk::ImageCreateFlags::empty());
            // 
            //     let pk = match p {
            //         Ok(f) => {},
            //         Err(error) => {
            //             panic!("error {}", error);
            //         }
            //     };
            // }

            let model_renderer = Self::load_model(&mut device, &swapchain,
                                                  forward_render_pass.get_native_render_pass(), &command_buffer_list);
            info!("model renderer created complete");
            RenderRunner {
                device_mgr: device,
                swapchain_mgr: swapchain,
                command_buffer_list,
                forward_render_pass,
                last_tick: SystemTime::now(),
                model_renderer,
            }
        }
    }

    fn load_model(context: &mut RenderContext, swapchain_mgr: &SwapChainMgr, render_pass: vk::RenderPass, command_buffer_list: &CommandBufferList) -> ModelRenderer {
        let command_buffer = command_buffer_list.get_command_buffer(0);
        unsafe {
            context.device.begin_command_buffer(command_buffer,
                                                &vk::CommandBufferBeginInfo::builder().
                                                    flags(vk::CommandBufferUsageFlags::ONE_TIME_SUBMIT).build());
        }

        let model_renderer = ModelRenderer::create(context,
                                                   swapchain_mgr,
                                                   render_pass,
                                                   command_buffer,
                                                   "assets/gltf/DamagedHelmet/DamagedHelmet.gltf",
                                                   "spv/simple_draw_object_vert.spv",
                                                   "spv/simple_draw_object_frag.spv");


        unsafe {
            context.device.end_command_buffer(command_buffer);
            context.device.queue_submit(context.graphics_queue, &[vk::SubmitInfo::builder().command_buffers(&[command_buffer]).build()], vk::Fence::null());
            context.device.device_wait_idle();
        }

        context.flush_staging_buffer();

        model_renderer
    }


    pub fn draw(&mut self) {
        unsafe {
            let now = SystemTime::now();
            self.last_tick = now;
            let (success, present_index) = self.swapchain_mgr.wait_for_swap_chain(&mut self.device_mgr);
            if !success {
                return;
            }

            let command_buffer = self.command_buffer_list.get_command_buffer(present_index);
            {
                let command_buffer_begin_info = vk::CommandBufferBeginInfo::builder().
                    flags(vk::CommandBufferUsageFlags::ONE_TIME_SUBMIT).build();
                self.device_mgr.device.begin_command_buffer(command_buffer, &command_buffer_begin_info);
            }

            self.forward_render_pass.begin_render_pass(&mut self.device_mgr, &mut self.swapchain_mgr, command_buffer);

            self.model_renderer.draw(&self.device_mgr, command_buffer);

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

            self.device_mgr.device.cmd_copy_image(command_buffer,
                                                  self.forward_render_pass.get_final_render_image(),
                                                  vk::ImageLayout::TRANSFER_SRC_OPTIMAL,
                                                  self.swapchain_mgr.get_current_present_image(),
                                                  vk::ImageLayout::TRANSFER_DST_OPTIMAL,
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

                self.device_mgr.device.cmd_pipeline_barrier(command_buffer,
                                                            vk::PipelineStageFlags::ALL_COMMANDS,
                                                            vk::PipelineStageFlags::ALL_COMMANDS,
                                                            vk::DependencyFlags::empty(),
                                                            &[],
                                                            &[],
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
        }
    }

    fn on_window_size_changed(&mut self, window_width: u32, window_height: u32) {
        //self.swapchain.refresh(&self.device, window_width, window_height);
    }
}
