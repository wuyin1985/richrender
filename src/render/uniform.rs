use crate::render::render_context::{RenderContext, RenderResource};
use ash::vk;
use crate::render::buffer::Buffer;
use std::any::Any;

#[derive(Default)]
pub struct UniformObject<T> {
    buffer: Buffer,
    pub data: T,
    pub descriptor_set_layout: vk::DescriptorSetLayout,
    pub descriptor_set: vk::DescriptorSet,
}

impl<T: 'static + Send + Sync + Copy> RenderResource for UniformObject<T> {
    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }

    fn destroy_res(&mut self, rc: &RenderContext) {
        self.destroy(rc);
    }
}

impl<T> UniformObject<T> where T: Copy {
    pub fn destroy(&mut self, context: &RenderContext) {
        unsafe {
            context.device.free_descriptor_sets(context.descriptor_pool, &[self.descriptor_set]);
            context.device.destroy_descriptor_set_layout(self.descriptor_set_layout, None);
        }
        self.buffer.destroy(context);
    }

    pub fn create(context: &mut RenderContext, data: T, descriptor_type: vk::DescriptorType, shader_stages: vk::ShaderStageFlags) -> UniformObject<T>
    {
        let mut uniform_buffer = Buffer::create_host_visible_buffer(context, vk::BufferUsageFlags::UNIFORM_BUFFER, &[data]);

        let bindings = [vk::DescriptorSetLayoutBinding::builder().
            binding(0).descriptor_count(1).descriptor_type(descriptor_type)
            .stage_flags(shader_stages).build()];
        let descriptor_set_layout = unsafe {
            context.
                device.create_descriptor_set_layout(
                &vk::DescriptorSetLayoutCreateInfo::builder().bindings(&bindings).build(), None)
                .expect("create descriptor layout failed")
        };

        let descriptor_sets = [descriptor_set_layout];

        let descriptor_alloc_ci = vk::DescriptorSetAllocateInfo::builder()
            .descriptor_pool(context.descriptor_pool)
            .set_layouts(&descriptor_sets)
            .build();

        let descriptor_sets = unsafe
            { context.device.allocate_descriptor_sets(&descriptor_alloc_ci).expect("failed to allocate descriptor sets") };

        let descriptor_set = descriptor_sets[0];

        let descriptor_buffer_info = [vk::DescriptorBufferInfo::builder()
            .buffer(uniform_buffer.buffer)
            .offset(0)
            .range(vk::WHOLE_SIZE)
            .build()];

        let descriptor_write_info = vk::WriteDescriptorSet::builder()
            .descriptor_type(vk::DescriptorType::UNIFORM_BUFFER)
            .dst_set(descriptor_set)
            .dst_binding(0)
            .buffer_info(&descriptor_buffer_info)
            .build();

        unsafe {
            context.device.update_descriptor_sets(&[descriptor_write_info], &[]);
        }

        UniformObject {
            buffer: uniform_buffer,
            data,
            descriptor_set,
            descriptor_set_layout,
        }
    }

    pub fn upload_data_2_device(&mut self, context: &RenderContext, data: T) {
        self.data = data;
        self.buffer.upload_data_align(context, &[self.data]);
    }
}
