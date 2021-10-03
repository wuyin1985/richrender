/*use crate::render::graphic_pipeline::{GraphicPipeline, PipelineVertexInputInfo};
use ash::vk;
use crate::render::render_context::RenderContext;
use crate::render::swapchain_mgr::SwapChainMgr;
use crate::render::forward_render::ForwardRenderPass;

pub struct DebugDraw
{
    descriptor_set_layout: vk::DescriptorSetLayout,
    descriptor_set: vk::DescriptorSet,
    pipeline: GraphicPipeline,
}

impl DebugDraw {
    pub fn create(context: &mut RenderContext, swap_mgr: &SwapChainMgr, forward_render: &ForwardRenderPass) -> Self {
        let (descriptor_set_layout, descriptor_set) = {
            let bindings = [
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


            let shadow_info = {
                let shadow = forward_render.get_shadow();
                let (view, sampler) = (shadow.shadow_view, shadow.sampler);

                [vk::DescriptorImageInfo::builder()
                    .image_layout(vk::ImageLayout::DEPTH_ATTACHMENT_STENCIL_READ_ONLY_OPTIMAL)
                    .image_view(view)
                    .sampler(sampler)
                    .build()]
            };

            let descriptor_writes = [
                vk::WriteDescriptorSet::builder()
                    .dst_set(set)
                    .dst_binding(0)
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
        };

        let all_layout = [descriptor_set_layout];
        let pipeline_layout_ci = vk::PipelineLayoutCreateInfo::builder().set_layouts(&all_layout).build();


        let vi = PipelineVertexInputInfo::none();
        let pipeline = GraphicPipeline::create(context,
                                               swap_mgr,
                                               forward_render.get_native_render_pass(),
                                               &vi,
                                               &pipeline_layout_ci,
                                               vk::SampleCountFlags::TYPE_1,
                                               "", "", &[]);

        Self {
            descriptor_set_layout,
            descriptor_set,
            pipeline,
        }
    }

    pub fn draw(&self, context: &RenderContext, command_buffer: vk::CommandBuffer) {
        let device = context.device;
        unsafe {
            device.cmd_bind_pipeline(command_buffer, vk::PipelineBindPoint::GRAPHICS, self.pipeline.get_pipeline());
            device.cmd_bind_descriptor_sets(command_buffer,
                                            vk::PipelineBindPoint::GRAPHICS, self.pipeline.get_layout(),
                                            0, &[self.descriptor_set], &[0]);
            device.cmd_draw(command_buffer, 3, 1, 0, 0);
        }
    }
}*/