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

            unsafe {
                self.device_mgr.device.end_command_buffer(command_buffer);
            }

            let mut present_image_available_semaphore: vk::Semaphore = vk::Semaphore::null();
            let mut render_finish_semaphore: vk::Semaphore = vk::Semaphore::null();
            let mut cmd_buf_execute_fence: vk::Fence = vk::Fence::null();

            self.swapchain_mgr.get_semaphores(&mut present_image_available_semaphore,
                                              &mut render_finish_semaphore, &mut cmd_buf_execute_fence);

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
