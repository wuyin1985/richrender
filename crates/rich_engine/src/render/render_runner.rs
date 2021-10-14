use crate::render::render_context::{RenderContext, PerFrameData, DummyResources};
use crate::render::swapchain_mgr::SwapChainMgr;
use bevy::winit::WinitWindows;
use crate::render::forward_render::ForwardRenderPass;
use crate::render::command_buffer_list::CommandBufferList;
use ash::vk;
use std::time::SystemTime;
use crate::render::model::Model;
use crate::render::model_renderer::ModelRenderer;
use bevy::prelude::*;
use crate::render::grass::GrassMgr;
use crate::render::uniform::UniformObject;

pub struct RenderRunner {
    pub context: RenderContext,
    pub swapchain_mgr: SwapChainMgr,
    pub command_buffer_list: CommandBufferList,
    pub forward_render_pass: ForwardRenderPass,
    pub grass: GrassMgr,
    last_tick: SystemTime,
    pub current_present_index: i32,
}

impl Drop for RenderRunner {
    fn drop(&mut self) {
        unsafe { self.context.device.device_wait_idle().unwrap(); }
        self.grass.destroy(&self.context);
        self.command_buffer_list.destroy(&self.context);
        self.forward_render_pass.destroy(&self.context);
        self.swapchain_mgr.destroy(&self.context);
        self.context.destroy();
    }
}


impl RenderRunner {
    pub fn create<W: raw_window_handle::HasRawWindowHandle>(window: &W, window_width: u32, window_height: u32) -> Self {
        unsafe {
            info!("start up");
            let mut context = RenderContext::create(window, window_width, window_height);
            let per_frame_data = UniformObject::<PerFrameData>::create(&mut context,
                                                                       PerFrameData::create(),
                                                                       vk::DescriptorType::UNIFORM_BUFFER,
                                                                       vk::ShaderStageFlags::VERTEX |
                                                                           vk::ShaderStageFlags::FRAGMENT |
                                                                           vk::ShaderStageFlags::TESSELLATION_EVALUATION|
                                                                           vk::ShaderStageFlags::COMPUTE);
            context.per_frame_uniform = Some(per_frame_data);
            //context.push_resource(per_frame_data);

            info!("render context create complete");
            let swapchain = SwapChainMgr::create(&context, window_width, window_height);
            let command_buffer_list = CommandBufferList::create(swapchain.get_present_image_count(), &context);
            let forward_render_pass = ForwardRenderPass::create(&mut context, &swapchain, &command_buffer_list);

            let command_buffer = command_buffer_list.get_upload_command_buffer();
            unsafe {
                context.device.begin_command_buffer(command_buffer,
                                                    &vk::CommandBufferBeginInfo::builder().
                                                        flags(vk::CommandBufferUsageFlags::ONE_TIME_SUBMIT).build());
            }

            let grass = GrassMgr::create(&mut context, &swapchain, &forward_render_pass, command_buffer);

            let dummy_res = DummyResources::create(&mut context, command_buffer);
            context.insert_resource(dummy_res);

            unsafe {
                context.device.end_command_buffer(command_buffer);
                context.device.queue_submit(context.graphics_queue, &[vk::SubmitInfo::builder().command_buffers(&[command_buffer]).build()], vk::Fence::null());
                context.device.device_wait_idle();
            }

            context.flush_staging_buffer();

            info!("forward render pass create complete");
            // unsafe {
            //     let p = context.instance.get_physical_context_image_format_properties(context.physical_context,
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

            info!("model renderer created complete");

            RenderRunner {
                context,
                swapchain_mgr: swapchain,
                command_buffer_list,
                forward_render_pass,
                last_tick: SystemTime::now(),
                current_present_index: -1,
                grass,
            }
        }
    }

    pub fn upload_per_frame_data(&mut self, data: PerFrameData) {
        let context = &mut self.context;
        let mut pf = std::mem::take(&mut context.per_frame_uniform);
        let uo = pf.as_mut().unwrap();
        uo.upload_data_2_device(context, data);
        context.per_frame_uniform = pf;
    }

    pub fn begin_draw(&mut self) -> Option<(usize, vk::CommandBuffer)> {
        let now = SystemTime::now();
        self.last_tick = now;
        let (success, present_index) = self.swapchain_mgr.wait_for_swap_chain(&mut self.context);
        if !success {
            return None;
        }

        let command_buffer = self.command_buffer_list.get_command_buffer(present_index);
        {
            let command_buffer_begin_info = vk::CommandBufferBeginInfo::builder().
                flags(vk::CommandBufferUsageFlags::ONE_TIME_SUBMIT).build();
            unsafe {
                self.context.device.begin_command_buffer(command_buffer, &command_buffer_begin_info);
            }
        }

        self.current_present_index = present_index as _;
        return Some((present_index, command_buffer));
    }

    pub fn get_current_command_buffer(&self) -> Option<vk::CommandBuffer> {
        if self.current_present_index >= 0 {
            return Some(self.command_buffer_list.get_command_buffer(self.current_present_index as _));
        }

        return None;
    }

    pub fn get_upload_command_buffer(&self) -> vk::CommandBuffer {
        self.command_buffer_list.get_upload_command_buffer()
    }

    pub fn end_draw(&mut self, command_buffer: vk::CommandBuffer) {
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

            unsafe {
                self.context.device.cmd_pipeline_barrier(command_buffer, vk::PipelineStageFlags::ALL_COMMANDS,
                                                         vk::PipelineStageFlags::ALL_COMMANDS,
                                                         vk::DependencyFlags::empty(), &[], &[],
                                                         &image_barriers);
            }
        }


        let surface_resolution = self.swapchain_mgr.surface_resolution;

        unsafe {
            self.context.device.cmd_copy_image(command_buffer,
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
        }

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

            unsafe {
                self.context.device.cmd_pipeline_barrier(command_buffer,
                                                         vk::PipelineStageFlags::ALL_COMMANDS,
                                                         vk::PipelineStageFlags::ALL_COMMANDS,
                                                         vk::DependencyFlags::empty(),
                                                         &[],
                                                         &[],
                                                         &image_barriers);
            }
        }

        unsafe {
            self.context.device.end_command_buffer(command_buffer);
        }

        let submit_info = vk::SubmitInfo::builder()
            .wait_semaphores(&[present_image_available_semaphore]).wait_dst_stage_mask(&[vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT])
            .command_buffers(&[command_buffer]).signal_semaphores(&[render_finish_semaphore]).build();

        unsafe {
            self.context.device.queue_submit(self.context.graphics_queue, &[submit_info], cmd_buf_execute_fence);
        }

        self.swapchain_mgr.present(&self.context);
    }

    fn on_window_size_changed(&mut self, window_width: u32, window_height: u32) {
        //self.swapchain.refresh(&self.device, window_width, window_height);
    }
}
