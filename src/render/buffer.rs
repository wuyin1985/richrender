use ash::vk;
use crate::render::render_context::RenderContext;
use std::mem::size_of;
use std::ffi::c_void;

struct MemoryMapPointer(*mut c_void);

unsafe impl Send for MemoryMapPointer {}

unsafe impl Sync for MemoryMapPointer {}

pub struct Buffer {
    pub buffer: vk::Buffer,
    pub memory: vk::DeviceMemory,
    pub size: vk::DeviceSize,
    mapped_ptr: Option<MemoryMapPointer>,
}

impl Default for Buffer {
    fn default() -> Self {
        Self {
            buffer: Default::default(),
            memory: Default::default(),
            size: Default::default(),
            mapped_ptr: None,
        }
    }
}

impl Buffer {
    pub fn destroy(&mut self, context: &RenderContext) {
        self.unmap_memory(context);
        unsafe {
            context.device.destroy_buffer(self.buffer, None);
            context.device.free_memory(self.memory, None);
        }
    }

    pub fn create(context: &RenderContext, size: vk::DeviceSize, usage: vk::BufferUsageFlags, mem_properties: vk::MemoryPropertyFlags) -> Self {
        let device = &context.device;
        let buffer = {
            let buffer_ci = vk::BufferCreateInfo::builder().size(size).usage(usage).sharing_mode(vk::SharingMode::EXCLUSIVE);
            unsafe {
                device.create_buffer(&buffer_ci, None).expect("failed to create buffer")
            }
        };

        let mem_requirements = unsafe { device.get_buffer_memory_requirements(buffer) };
        let memory = {
            let mem_type = context.find_memory_type_index(&mem_requirements, mem_properties).unwrap();
            let alloc_info = vk::MemoryAllocateInfo::builder().allocation_size(mem_requirements.size).memory_type_index(mem_type);
            unsafe {
                device.allocate_memory(&alloc_info, None).expect("failed to allocate memory")
            }
        };

        unsafe {
            device
                .bind_buffer_memory(buffer, memory, 0)
                .expect("Failed to bind buffer memory")
        };

        Buffer {
            buffer,
            size,
            memory,
            mapped_ptr: None,
        }
    }

    pub unsafe fn mem_copy<T: Copy>(ptr: *mut c_void, data: &[T]) {
        let elem_size = size_of::<T>() as vk::DeviceSize;
        let size = data.len() as vk::DeviceSize * elem_size;
        let mut align = ash::util::Align::new(ptr, elem_size, size);
        align.copy_from_slice(data);
    }

    pub fn create_host_visible_buffer<T: Copy>(context: &RenderContext, usage: vk::BufferUsageFlags, data: &[T]) -> Self {
        let size = (data.len() * size_of::<T>()) as vk::DeviceSize;
        let mut buffer = Buffer::create(context, size, usage,
                                        vk::MemoryPropertyFlags::HOST_VISIBLE | vk::MemoryPropertyFlags::HOST_COHERENT);
        unsafe {
            let ptr = buffer.map_memory(context);
            Self::mem_copy(ptr, data);
        }

        buffer
    }

    pub fn upload_data<T: Copy>(&mut self, context: &RenderContext, data: &[T]) {
        unsafe {
            let ptr = self.map_memory(context);
            Self::mem_copy(ptr, data);
        }
    }

    pub fn create_device_local_buffer<T: Copy>(context: &mut RenderContext, upload_command_buffer: vk::CommandBuffer, usage: vk::BufferUsageFlags, data: &[T]) -> Buffer {
        let size = (data.len() * size_of::<T>()) as vk::DeviceSize;
        let mut staging_buffer = Self::create_host_visible_buffer(context, vk::BufferUsageFlags::TRANSFER_SRC, data);
        let mut device_buffer = Self::create(context, size, vk::BufferUsageFlags::TRANSFER_DST | usage, vk::MemoryPropertyFlags::DEVICE_LOCAL);
        device_buffer.cmd_copy(context, upload_command_buffer, &staging_buffer, size);
        context.push_staging_buffer(staging_buffer);
        device_buffer
    }


    pub fn map_memory(&mut self, context: &RenderContext) -> *mut c_void {
        if let Some(ptr) = &self.mapped_ptr {
            ptr.0
        } else {
            unsafe {
                let ptr = context.device.map_memory(self.memory, 0, self.size, vk::MemoryMapFlags::empty())
                    .expect("failed to map memory");
                self.mapped_ptr = Some(MemoryMapPointer(ptr));
                ptr
            }
        }
    }

    fn unmap_memory(&mut self, context: &RenderContext) {
        if self.mapped_ptr.take().is_some() {
            unsafe {
                context.device.unmap_memory(self.memory);
            }
        }
    }

    pub fn cmd_copy(&self, context: &RenderContext, command_buffer: vk::CommandBuffer, src: &Buffer, size: vk::DeviceSize) {
        let region = vk::BufferCopy {
            src_offset: 0,
            dst_offset: 0,
            size,
        };
        let regions = [region];

        unsafe {
            context.device
                .cmd_copy_buffer(command_buffer, src.buffer, self.buffer, &regions)
        };
    }
}

