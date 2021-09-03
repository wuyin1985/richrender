use ash::vk;
use crate::render::device_mgr::DeviceMgr;
use ash::vk::ImageUsageFlags;

pub struct TextureHead {
    width: u32,
    height: u32,
    depth: u32,
    array_size: u32,
    mip_map_count: u32,
    format: vk::Format,
}

pub struct RenderTexture {
    image: vk::Image,
    device_memory: vk::DeviceMemory,
    head: TextureHead,
}


impl RenderTexture {
    pub fn destroy(&mut self, device_mgr: &DeviceMgr) {
        unsafe {
            device_mgr.device.destroy_image(self.image, None);
            device_mgr.device.free_memory(self.device_memory, None);
        }
    }

    pub fn create(device_mgr: &DeviceMgr, image_info: &vk::ImageCreateInfo, name: &str) -> Self {
        let head = TextureHead {
            format: image_info.format,
            width: image_info.extent.width,
            height: image_info.extent.height,
            depth: image_info.extent.depth,
            array_size: image_info.array_layers,
            mip_map_count: image_info.mip_levels,
        };

        unsafe {
            let image = device_mgr.device.create_image(&image_info, None).unwrap();
            let mem_req = device_mgr.device.get_image_memory_requirements(image);
            let texture_memory_index = device_mgr.find_memory_type_index(&mem_req, vk::MemoryPropertyFlags::DEVICE_LOCAL).
                expect("failed to find mem index for texture");
            let texture_allocate_info = vk::MemoryAllocateInfo {
                allocation_size: mem_req.size,
                memory_type_index: texture_memory_index,
                ..Default::default()
            };
            let device_memory = device_mgr.device.allocate_memory(&texture_allocate_info, None).unwrap();
            device_mgr.device.bind_image_memory(image, device_memory, 0).expect("unable to bind texture memory");

            Self {
                image,
                device_memory,
                head,
            }
        }
    }

    pub fn create_as_render_target(device_mgr: &'a DeviceMgr, width: u32, height: u32, format: vk::Format,
                                   msaa: vk::SampleCountFlags, usage: vk::ImageUsageFlags,
                                   name: &str, flags: vk::ImageCreateFlags) -> Self {
        let image_info = vk::ImageCreateInfo {
            format: format,
            extent: vk::Extent3D {
                width: width,
                height: height,
                depth: 1,
            },
            usage: usage,
            flags: flags,

            tiling: vk::ImageTiling::OPTIMAL,
            image_type: vk::ImageType::TYPE_2D,
            mip_levels: 1,
            array_layers: 1,
            samples: msaa,
            initial_layout: vk::ImageLayout::UNDEFINED,
            queue_family_index_count: 0,
            p_queue_family_indices: std::ptr::null(),
            sharing_mode: vk::SharingMode::EXCLUSIVE,
            ..Default::default()
        };

        RenderTexture::create(device_mgr, &image_info, name)
    }

    pub fn create_as_depth_stencil(device_mgr: &DeviceMgr, width: u32, height: u32,
                                   format: vk::Format, msaa: vk::SampleCountFlags, name: &str) -> RenderTexture {
        let image_info = vk::ImageCreateInfo {
            format: format,
            extent: vk::Extent3D {
                width: width,
                height: height,
                depth: 1,
            },
            usage: vk::ImageUsageFlags::DEPTH_STENCIL_ATTACHMENT | vk::ImageUsageFlags::SAMPLED,
            flags: flags,

            tiling: vk::ImageTiling::OPTIMAL,
            image_type: vk::ImageType::TYPE_2D,
            mip_levels: 1,
            array_layers: 1,
            samples: msaa,
            initial_layout: vk::ImageLayout::UNDEFINED,
            queue_family_index_count: 0,
            p_queue_family_indices: std::ptr::null(),
            sharing_mode: vk::SharingMode::EXCLUSIVE,
            ..Default::default()
        };

        RenderTexture::create(device_mgr, &image_info, name)
    }


    pub fn get_format(&self) -> vk::Format {
        self.head.format
    }

    pub fn create_color_view(&self, device_mgr: &DeviceMgr) -> vk::ImageView {
        let view_ci = vk::ImageViewCreateInfo::builder().image(self.image).
            format(self.head.format).subresource_range(vk::ImageSubresourceRange {
            aspect_mask: vk::ImageAspectFlags::COLOR,
            base_mip_level: 0,
            level_count: self.head.mip_map_count,
            base_array_layer: 0,
            layer_count: 1,
        }).view_type(vk::ImageViewType::TYPE_2D).build();

        unsafe {
            device_mgr.device.create_image_view(&view_ci, None).unwrap()
        }
    }

    pub fn create_depth_view(&self, device_mgr: &DeviceMgr) -> vk::ImageView {
        let view_ci = vk::ImageViewCreateInfo::builder().image(self.image).
            format(self.head.format).subresource_range(vk::ImageSubresourceRange {
            aspect_mask: vk::ImageAspectFlags::DEPTH,
            base_mip_level: 0,
            level_count: 1,
            base_array_layer: 0,
            layer_count: 1,
        }).view_type(vk::ImageViewType::TYPE_2D).build();

        unsafe {
            device_mgr.device.create_image_view(&view_ci, None).unwrap()
        }
    }
}