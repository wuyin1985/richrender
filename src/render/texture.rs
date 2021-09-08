use ash::vk;
use crate::render::render_context::RenderContext;
use ash::vk::ImageUsageFlags;
use gltf::image::Format;
use crate::render::buffer::Buffer;
use std::mem::size_of;
use crate::render::util;

pub struct TextureHead {
    width: u32,
    height: u32,
    depth: u32,
    array_size: u32,
    mip_map_count: u32,
    format: vk::Format,
}

pub struct Texture {
    image: vk::Image,
    device_memory: vk::DeviceMemory,
    head: TextureHead,
}


impl Texture {
    pub fn destroy(&mut self, device_mgr: &RenderContext) {
        unsafe {
            device_mgr.device.destroy_image(self.image, None);
            device_mgr.device.free_memory(self.device_memory, None);
        }
    }

    pub fn create(device_mgr: &RenderContext, image_info: &vk::ImageCreateInfo, name: &str) -> Self {
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

    pub fn create_from_data(context: &mut RenderContext, upload_command_buffer: vk::CommandBuffer, image_ci: &vk::ImageCreateInfo, data: &[u8]) -> Self {
        let texture = Self::create(context, image_ci, "image");
        let image_size = (data.len() * size_of::<u8>()) as vk::DeviceSize;
        let mut buffer = Buffer::create(
            context,
            image_size,
            vk::BufferUsageFlags::TRANSFER_SRC,
            vk::MemoryPropertyFlags::HOST_VISIBLE | vk::MemoryPropertyFlags::HOST_COHERENT,
        );

        unsafe {
            let ptr = buffer.map_memory(context);
            util::mem_copy(ptr, &data);
        }

        //copy data to image
        {
            let extent = vk::Extent2D { width: texture.head.width, height: texture.head.height };

            texture.cmd_transition_image_layout(context, upload_command_buffer,
                                                vk::ImageLayout::UNDEFINED, vk::ImageLayout::TRANSFER_DST_OPTIMAL);
            texture.cmd_copy_buffer(context, upload_command_buffer, &buffer, extent);

            texture.cmd_generate_mipmaps(context, upload_command_buffer, extent);
        }
        
        context.push_staging_buffer(buffer);
        
        texture
    }

    pub fn create_as_render_target(device_mgr: &RenderContext, width: u32, height: u32, format: vk::Format,
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

        Texture::create(device_mgr, &image_info, name)
    }

    pub fn create_as_depth_stencil(device_mgr: &RenderContext, width: u32, height: u32,
                                   format: vk::Format, msaa: vk::SampleCountFlags, name: &str) -> Texture {
        let image_info = vk::ImageCreateInfo {
            format: format,
            extent: vk::Extent3D {
                width: width,
                height: height,
                depth: 1,
            },
            usage: vk::ImageUsageFlags::DEPTH_STENCIL_ATTACHMENT | vk::ImageUsageFlags::SAMPLED,
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

        Texture::create(device_mgr, &image_info, name)
    }


    pub fn get_format(&self) -> vk::Format {
        self.head.format
    }

    pub fn create_color_view(&self, device_mgr: &RenderContext) -> vk::ImageView {
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

    pub fn create_sample(&self, context: &RenderContext, max_mip_levels: u32) -> vk::Sampler {
        let sampler = {
            let sampler_info = vk::SamplerCreateInfo::builder()
                .mag_filter(vk::Filter::LINEAR)
                .min_filter(vk::Filter::LINEAR)
                .address_mode_u(vk::SamplerAddressMode::REPEAT)
                .address_mode_v(vk::SamplerAddressMode::REPEAT)
                .address_mode_w(vk::SamplerAddressMode::REPEAT)
                .anisotropy_enable(true)
                .max_anisotropy(16.0)
                .border_color(vk::BorderColor::INT_OPAQUE_BLACK)
                .unnormalized_coordinates(false)
                .compare_enable(false)
                .compare_op(vk::CompareOp::ALWAYS)
                .mipmap_mode(vk::SamplerMipmapMode::LINEAR)
                .mip_lod_bias(0.0)
                .min_lod(0.0)
                .max_lod(max_mip_levels as _);

            unsafe {
                context.device
                    .create_sampler(&sampler_info, None)
                    .expect("Failed to create sampler")
            }
        };

        sampler
    }

    pub fn create_depth_view(&self, device_mgr: &RenderContext) -> vk::ImageView {
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

    pub fn cmd_copy_buffer(
        &self,
        context: &RenderContext,
        command_buffer: vk::CommandBuffer,
        buffer: &Buffer,
        extent: vk::Extent2D,
    ) {
        let region = vk::BufferImageCopy::builder()
            .buffer_offset(0)
            .buffer_row_length(0)
            .buffer_image_height(0)
            .image_subresource(vk::ImageSubresourceLayers {
                aspect_mask: vk::ImageAspectFlags::COLOR,
                mip_level: 0,
                base_array_layer: 0,
                layer_count: self.head.array_size,
            })
            .image_offset(vk::Offset3D { x: 0, y: 0, z: 0 })
            .image_extent(vk::Extent3D {
                width: extent.width,
                height: extent.height,
                depth: 1,
            })
            .build();
        let regions = [region];
        unsafe {
            context.device.cmd_copy_buffer_to_image(
                command_buffer,
                buffer.buffer,
                self.image,
                vk::ImageLayout::TRANSFER_DST_OPTIMAL,
                &regions,
            )
        }
    }

    pub fn cmd_generate_mipmaps(&self, context: &RenderContext, command_buffer: vk::CommandBuffer, extent: vk::Extent2D) {
        let format_properties = unsafe {
            context.instance
                .get_physical_device_format_properties(context.physical_device, self.get_format())
        };
        if !format_properties
            .optimal_tiling_features
            .contains(vk::FormatFeatureFlags::SAMPLED_IMAGE_FILTER_LINEAR)
        {
            panic!(
                "Linear blitting is not supported for format {:?}.",
                self.get_format()
            )
        }

        let mut barrier = vk::ImageMemoryBarrier::builder()
            .image(self.image)
            .src_queue_family_index(vk::QUEUE_FAMILY_IGNORED)
            .dst_queue_family_index(vk::QUEUE_FAMILY_IGNORED)
            .subresource_range(vk::ImageSubresourceRange {
                aspect_mask: vk::ImageAspectFlags::COLOR,
                base_array_layer: 0,
                layer_count: self.head.array_size,
                level_count: 1,
                ..Default::default()
            })
            .build();

        let mut mip_width = extent.width as i32;
        let mut mip_height = extent.height as i32;
        for level in 1..self.head.mip_map_count {
            let next_mip_width = if mip_width > 1 {
                mip_width / 2
            } else {
                mip_width
            };
            let next_mip_height = if mip_height > 1 {
                mip_height / 2
            } else {
                mip_height
            };

            barrier.subresource_range.base_mip_level = level - 1;
            barrier.old_layout = vk::ImageLayout::TRANSFER_DST_OPTIMAL;
            barrier.new_layout = vk::ImageLayout::TRANSFER_SRC_OPTIMAL;
            barrier.src_access_mask = vk::AccessFlags::TRANSFER_WRITE;
            barrier.dst_access_mask = vk::AccessFlags::TRANSFER_READ;
            let barriers = [barrier];

            unsafe {
                context.device.cmd_pipeline_barrier(
                    command_buffer,
                    vk::PipelineStageFlags::TRANSFER,
                    vk::PipelineStageFlags::TRANSFER,
                    vk::DependencyFlags::empty(),
                    &[],
                    &[],
                    &barriers,
                )
            };

            let blit = vk::ImageBlit::builder()
                .src_offsets([
                    vk::Offset3D { x: 0, y: 0, z: 0 },
                    vk::Offset3D {
                        x: mip_width,
                        y: mip_height,
                        z: 1,
                    },
                ])
                .src_subresource(vk::ImageSubresourceLayers {
                    aspect_mask: vk::ImageAspectFlags::COLOR,
                    mip_level: level - 1,
                    base_array_layer: 0,
                    layer_count: self.head.array_size,
                })
                .dst_offsets([
                    vk::Offset3D { x: 0, y: 0, z: 0 },
                    vk::Offset3D {
                        x: next_mip_width,
                        y: next_mip_height,
                        z: 1,
                    },
                ])
                .dst_subresource(vk::ImageSubresourceLayers {
                    aspect_mask: vk::ImageAspectFlags::COLOR,
                    mip_level: level,
                    base_array_layer: 0,
                    layer_count: self.head.array_size,
                })
                .build();
            let blits = [blit];

            unsafe {
                context.device.cmd_blit_image(
                    command_buffer,
                    self.image,
                    vk::ImageLayout::TRANSFER_SRC_OPTIMAL,
                    self.image,
                    vk::ImageLayout::TRANSFER_DST_OPTIMAL,
                    &blits,
                    vk::Filter::LINEAR,
                )
            };

            barrier.old_layout = vk::ImageLayout::TRANSFER_SRC_OPTIMAL;
            barrier.new_layout = vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL;
            barrier.src_access_mask = vk::AccessFlags::TRANSFER_READ;
            barrier.dst_access_mask = vk::AccessFlags::SHADER_READ;
            let barriers = [barrier];

            unsafe {
                context.device.cmd_pipeline_barrier(
                    command_buffer,
                    vk::PipelineStageFlags::TRANSFER,
                    vk::PipelineStageFlags::FRAGMENT_SHADER,
                    vk::DependencyFlags::empty(),
                    &[],
                    &[],
                    &barriers,
                )
            };

            mip_width = next_mip_width;
            mip_height = next_mip_height;
        }

        barrier.subresource_range.base_mip_level = self.head.mip_map_count - 1;
        barrier.old_layout = vk::ImageLayout::TRANSFER_DST_OPTIMAL;
        barrier.new_layout = vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL;
        barrier.src_access_mask = vk::AccessFlags::TRANSFER_WRITE;
        barrier.dst_access_mask = vk::AccessFlags::SHADER_READ;
        let barriers = [barrier];

        unsafe {
            context.device.cmd_pipeline_barrier(
                command_buffer,
                vk::PipelineStageFlags::TRANSFER,
                vk::PipelineStageFlags::FRAGMENT_SHADER,
                vk::DependencyFlags::empty(),
                &[],
                &[],
                &barriers,
            )
        };
    }


    pub fn cmd_transition_image_layout(
        &self,
        context: &RenderContext,
        command_buffer: vk::CommandBuffer,
        old_layout: vk::ImageLayout,
        new_layout: vk::ImageLayout,
    ) {
        let (src_access_mask, dst_access_mask, src_stage, dst_stage) =
            match (old_layout, new_layout) {
                (vk::ImageLayout::UNDEFINED, vk::ImageLayout::TRANSFER_DST_OPTIMAL) => (
                    vk::AccessFlags::empty(),
                    vk::AccessFlags::TRANSFER_WRITE,
                    vk::PipelineStageFlags::TOP_OF_PIPE,
                    vk::PipelineStageFlags::TRANSFER,
                ),
                (
                    vk::ImageLayout::TRANSFER_DST_OPTIMAL,
                    vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL,
                ) => (
                    vk::AccessFlags::TRANSFER_WRITE,
                    vk::AccessFlags::SHADER_READ,
                    vk::PipelineStageFlags::TRANSFER,
                    vk::PipelineStageFlags::FRAGMENT_SHADER,
                ),
                (vk::ImageLayout::UNDEFINED, vk::ImageLayout::DEPTH_STENCIL_ATTACHMENT_OPTIMAL) => {
                    (
                        vk::AccessFlags::empty(),
                        vk::AccessFlags::DEPTH_STENCIL_ATTACHMENT_READ
                            | vk::AccessFlags::DEPTH_STENCIL_ATTACHMENT_WRITE,
                        vk::PipelineStageFlags::TOP_OF_PIPE,
                        vk::PipelineStageFlags::EARLY_FRAGMENT_TESTS,
                    )
                }
                (vk::ImageLayout::UNDEFINED, vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL) => (
                    vk::AccessFlags::empty(),
                    vk::AccessFlags::COLOR_ATTACHMENT_READ
                        | vk::AccessFlags::COLOR_ATTACHMENT_WRITE,
                    vk::PipelineStageFlags::TOP_OF_PIPE,
                    vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT,
                ),
                (
                    vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL,
                    vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL,
                ) => (
                    vk::AccessFlags::COLOR_ATTACHMENT_WRITE,
                    vk::AccessFlags::SHADER_READ,
                    vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT,
                    vk::PipelineStageFlags::FRAGMENT_SHADER,
                ),
                (
                    vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL,
                    vk::ImageLayout::TRANSFER_DST_OPTIMAL,
                ) => (
                    vk::AccessFlags::COLOR_ATTACHMENT_WRITE,
                    vk::AccessFlags::TRANSFER_WRITE,
                    vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT,
                    vk::PipelineStageFlags::TRANSFER,
                ),
                _ => (
                    vk::AccessFlags::empty(),
                    vk::AccessFlags::empty(),
                    vk::PipelineStageFlags::TOP_OF_PIPE,
                    vk::PipelineStageFlags::TOP_OF_PIPE,
                ),
            };

        let aspect_mask = if new_layout == vk::ImageLayout::DEPTH_STENCIL_ATTACHMENT_OPTIMAL {
            let mut mask = vk::ImageAspectFlags::DEPTH;
            if util::has_stencil_component(self.head.format) {
                mask |= vk::ImageAspectFlags::STENCIL;
            }
            mask
        } else {
            vk::ImageAspectFlags::COLOR
        };

        let barrier = vk::ImageMemoryBarrier::builder()
            .old_layout(old_layout)
            .new_layout(new_layout)
            .src_queue_family_index(vk::QUEUE_FAMILY_IGNORED)
            .dst_queue_family_index(vk::QUEUE_FAMILY_IGNORED)
            .image(self.image)
            .subresource_range(vk::ImageSubresourceRange {
                aspect_mask,
                base_mip_level: 0,
                level_count: self.head.mip_map_count,
                base_array_layer: 0,
                layer_count: self.head.array_size,
            })
            .src_access_mask(src_access_mask)
            .dst_access_mask(dst_access_mask)
            .build();
        let barriers = [barrier];

        unsafe {
            context.device.cmd_pipeline_barrier(
                command_buffer,
                src_stage,
                dst_stage,
                vk::DependencyFlags::empty(),
                &[],
                &[],
                &barriers,
            )
        };
    }

    pub fn get_image(&self) -> vk::Image {
        self.image
    }
}

