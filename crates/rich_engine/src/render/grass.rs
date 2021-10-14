use std::ffi::CString;
use std::mem::size_of;
use bevy::prelude::*;
use ash::vk;
use ash::vk::PipelineStageFlags;
use crate::render::graphic_pipeline::{GraphicPipeline, PipelineVertexInputInfo, ShaderStages};
use crate::{Buffer, ForwardRenderPass, RenderContext};
use crate::render::swapchain_mgr::SwapChainMgr;
use crate::render::util;

#[repr(C)]
#[derive(Clone, Debug, Copy)]
pub struct GrassGridData {
    grid_size: Vec2,
    slot_size: Vec2,
    slot_count: UVec2,
    grass_y: f32,
    grass_count: u32,
    dispatch_size: u32,
}

pub struct GrassGenerateCompute {
    descriptor_set_layout: vk::DescriptorSetLayout,
    descriptor_set: vk::DescriptorSet,
    pipeline: vk::Pipeline,
    pipeline_layout: vk::PipelineLayout,
    working_semaphore: vk::Semaphore,
}

impl GrassGenerateCompute {
    pub fn destroy(&mut self, context: &RenderContext) {
        unsafe {
            let device = &context.device;
            device.destroy_semaphore(self.working_semaphore, None);
            device.destroy_pipeline_layout(self.pipeline_layout, None);
            device.destroy_pipeline(self.pipeline, None);
            device.free_descriptor_sets(context.descriptor_pool, &[self.descriptor_set]);
            device.destroy_descriptor_set_layout(self.descriptor_set_layout, None);
        }
    }

    pub fn create(context: &mut RenderContext, swap_chain_mgr: &SwapChainMgr, grass_blade_buffer: &Buffer, grid: &GrassGridData) -> Self {
        let descriptor_layout = {
            let descriptor_set_bindings = [
                vk::DescriptorSetLayoutBinding::builder()
                    .binding(0)
                    .stage_flags(vk::ShaderStageFlags::COMPUTE)
                    .descriptor_type(vk::DescriptorType::STORAGE_BUFFER)
                    .descriptor_count(1)
                    .build()
            ];

            unsafe {
                let ci = vk::DescriptorSetLayoutCreateInfo::builder().bindings(&descriptor_set_bindings).build();
                context.device.create_descriptor_set_layout(&ci, None).expect("failed to create set layout")
            }
        };

        let descriptor_set = {
            let ai = vk::DescriptorSetAllocateInfo::builder().descriptor_pool(context.descriptor_pool).set_layouts(&[descriptor_layout])
                .build();
            unsafe {
                context.device.allocate_descriptor_sets(&ai).expect("failed to create descriptor sets")[0]
            }
        };

        //write descriptor set
        {
            let di = [vk::DescriptorBufferInfo::builder().buffer(grass_blade_buffer.buffer).offset(0).range(vk::WHOLE_SIZE).build()];
            let writes = [
                vk::WriteDescriptorSet::builder()
                    .dst_set(descriptor_set)
                    .descriptor_type(vk::DescriptorType::STORAGE_BUFFER)
                    .dst_binding(0).buffer_info(&di)
                    .build()
            ];

            unsafe {
                context.device.update_descriptor_sets(&writes, &[]);
            }
        }

        let constant_ranges = [
            vk::PushConstantRange::builder().offset(0).size(size_of::<GrassGridData>() as _).stage_flags(vk::ShaderStageFlags::COMPUTE).build()
        ];

        let pipeline_layout = {
            let descriptor_layouts = [descriptor_layout];
            let pipeline_layout_ci = vk::PipelineLayoutCreateInfo::builder().set_layouts(&descriptor_layouts)
                .push_constant_ranges(&constant_ranges).build();
            unsafe {
                context.device.create_pipeline_layout(&pipeline_layout_ci, None).expect("failed to create pipeline layout")
            }
        };

        let pipeline = {
            let stage = context.shader_modules.create_shader_stage(&context.device, "grass_generate_comp", &[],
                                                                   vk::ShaderStageFlags::COMPUTE);

            let ci = vk::ComputePipelineCreateInfo::builder().stage(stage).layout(pipeline_layout).build();
            unsafe {
                context.device.create_compute_pipelines(vk::PipelineCache::null(), &[ci], None).expect("create compute pipeline failed")[0]
            }
        };

        let working_semaphore = {
            let ci = vk::SemaphoreCreateInfo::builder().build();
            unsafe {
                context.device.create_semaphore(&ci, None).expect("create semaphore failed")
            }
        };

        GrassGenerateCompute {
            pipeline_layout,
            pipeline,
            descriptor_set,
            descriptor_set_layout: descriptor_layout,
            working_semaphore,
        }
    }

    pub fn compute(&mut self, context: &RenderContext, command_buffer: vk::CommandBuffer, grid: &GrassGridData) {
        unsafe {
            context.device.begin_command_buffer(command_buffer,
                                                &vk::CommandBufferBeginInfo::builder().flags(vk::CommandBufferUsageFlags::ONE_TIME_SUBMIT).build());

            context.device.cmd_bind_pipeline(command_buffer, vk::PipelineBindPoint::COMPUTE, self.pipeline);
            context.device.cmd_bind_descriptor_sets(command_buffer, vk::PipelineBindPoint::COMPUTE, self.pipeline_layout, 0, &[self
                .descriptor_set], &[]);
            let grid_data_bytes: &[u8] = unsafe { util::any_as_u8_slice(grid) };
            context.device.cmd_push_constants(command_buffer, self.pipeline_layout, vk::ShaderStageFlags::COMPUTE, 0, grid_data_bytes);
            context.device.cmd_dispatch(command_buffer, grid.dispatch_size, 1, 1);
            context.device.end_command_buffer(command_buffer);

            let mut ci = vk::SubmitInfo::builder().command_buffers(&[command_buffer])
                .signal_semaphores(&[self.working_semaphore])
                .wait_dst_stage_mask(&[PipelineStageFlags::TOP_OF_PIPE])
                .wait_semaphores(&[])
                .build();
            context.device.queue_submit(context.compute_queue, &[ci], vk::Fence::null());
        }
    }
}

#[repr(C)]
#[derive(Clone, Debug, Copy)]
struct NumBlades {
    vertex_count: u32,
    instance_count: u32,
    first_vertex: u32,
    first_instance: u32,
}

struct GrassUpdateCompute {
    descriptor_set_layout: vk::DescriptorSetLayout,
    descriptor_set: vk::DescriptorSet,
    pipeline: vk::Pipeline,
    pipeline_layout: vk::PipelineLayout,
    working_semaphore: vk::Semaphore,
    pub num_blades_buffer: Buffer,
}

impl GrassUpdateCompute {
    pub fn destroy(&mut self, context: &RenderContext) {
        unsafe {
            let device = &context.device;
            device.destroy_semaphore(self.working_semaphore, None);
            device.destroy_pipeline_layout(self.pipeline_layout, None);
            device.destroy_pipeline(self.pipeline, None);
            device.free_descriptor_sets(context.descriptor_pool, &[self.descriptor_set]);
            device.destroy_descriptor_set_layout(self.descriptor_set_layout, None);
        }
    }

    pub fn create(context: &mut RenderContext, swap_chain_mgr: &SwapChainMgr, upload_command_buffer: vk::CommandBuffer
                  , grass_blade_buffer: &Buffer,
                  visible_grass: &Buffer) -> Self {
        let num_blades = NumBlades { first_vertex: 0, first_instance: 0, instance_count: 1, vertex_count: 0 };
        let num_blades_buffer = Buffer::create_device_local_buffer(context, upload_command_buffer,
                                                                   vk::BufferUsageFlags::UNIFORM_BUFFER |
                                                                       vk::BufferUsageFlags::VERTEX_BUFFER |
                                                                       vk::BufferUsageFlags::STORAGE_BUFFER |
                                                                       vk::BufferUsageFlags::INDIRECT_BUFFER,
                                                                   &[num_blades]);

        let descriptor_layout = {
            let descriptor_set_bindings = [
                //all blades
                vk::DescriptorSetLayoutBinding::builder()
                    .binding(0)
                    .stage_flags(vk::ShaderStageFlags::COMPUTE)
                    .descriptor_type(vk::DescriptorType::STORAGE_BUFFER)
                    .descriptor_count(1)
                    .build(),
                //cull blades
                vk::DescriptorSetLayoutBinding::builder()
                    .binding(1)
                    .stage_flags(vk::ShaderStageFlags::COMPUTE)
                    .descriptor_type(vk::DescriptorType::STORAGE_BUFFER)
                    .descriptor_count(1)
                    .build(),
                //num blades
                vk::DescriptorSetLayoutBinding::builder()
                    .binding(2)
                    .stage_flags(vk::ShaderStageFlags::COMPUTE)
                    .descriptor_type(vk::DescriptorType::STORAGE_BUFFER)
                    .descriptor_count(1)
                    .build(),
            ];

            unsafe {
                let ci = vk::DescriptorSetLayoutCreateInfo::builder().bindings(&descriptor_set_bindings).build();
                context.device.create_descriptor_set_layout(&ci, None).expect("failed to create set layout")
            }
        };

        let descriptor_set = util::create_descriptor_set(context, descriptor_layout);

        //write descriptor set
        {
            let all_grass_info = [vk::DescriptorBufferInfo::builder()
                .buffer(grass_blade_buffer.buffer).offset(0).range(vk::WHOLE_SIZE)
                .build()];

            let visible_grass_info = [vk::DescriptorBufferInfo::builder().buffer(visible_grass.buffer).offset(0).range(vk::WHOLE_SIZE)
                .build()];

            let num_blades_info = [vk::DescriptorBufferInfo::builder().buffer(num_blades_buffer.buffer).offset(0).range(vk::WHOLE_SIZE)
                .build()];

            let writes = [
                vk::WriteDescriptorSet::builder().descriptor_type(vk::DescriptorType::STORAGE_BUFFER)
                    .dst_binding(0)
                    .dst_set(descriptor_set)
                    .buffer_info(&all_grass_info)
                    .build(),
                vk::WriteDescriptorSet::builder()
                    .dst_set(descriptor_set)
                    .descriptor_type(vk::DescriptorType::STORAGE_BUFFER)
                    .dst_binding(1).buffer_info(&visible_grass_info)
                    .build(),
                vk::WriteDescriptorSet::builder()
                    .dst_set(descriptor_set)
                    .descriptor_type(vk::DescriptorType::STORAGE_BUFFER)
                    .dst_binding(2).buffer_info(&num_blades_info)
                    .build(),
            ];
            unsafe {
                context.device.update_descriptor_sets(&writes, &[]);
            }
        }

        let uni = context.per_frame_uniform.as_ref().unwrap();

        let constant_ranges = [
            vk::PushConstantRange::builder().offset(0).size(size_of::<GrassGridData>() as _).stage_flags(vk::ShaderStageFlags::COMPUTE).build()
        ];

        let pipeline_layout = {
            let descriptor_layouts = [uni.descriptor_set_layout, descriptor_layout];
            let pipeline_layout_ci = vk::PipelineLayoutCreateInfo::builder().set_layouts(&descriptor_layouts).push_constant_ranges
            (&constant_ranges)
                .build();
            unsafe {
                context.device.create_pipeline_layout(&pipeline_layout_ci, None).expect("failed to create pipeline layout")
            }
        };

        let pipeline = {
            let stage = context.shader_modules.create_shader_stage(&context.device, "grass_update_comp", &[],
                                                                   vk::ShaderStageFlags::COMPUTE);

            let ci = vk::ComputePipelineCreateInfo::builder().stage(stage).layout(pipeline_layout).build();
            unsafe {
                context.device.create_compute_pipelines(vk::PipelineCache::null(), &[ci], None).expect("create compute pipeline failed")[0]
            }
        };

        let working_semaphore = {
            let ci = vk::SemaphoreCreateInfo::builder().build();
            unsafe {
                context.device.create_semaphore(&ci, None).expect("create semaphore failed")
            }
        };

        GrassUpdateCompute {
            pipeline_layout,
            pipeline,
            descriptor_set,
            descriptor_set_layout: descriptor_layout,
            working_semaphore,
            num_blades_buffer,
        }
    }

    pub fn compute(&mut self, context: &RenderContext, command_buffer: vk::CommandBuffer, grid: &GrassGridData,
                   wait_semaphore: &[vk::Semaphore]) {
        unsafe {
            context.device.begin_command_buffer(command_buffer,
                                                &vk::CommandBufferBeginInfo::builder().flags(vk::CommandBufferUsageFlags::ONE_TIME_SUBMIT).build());

            context.device.cmd_bind_pipeline(command_buffer, vk::PipelineBindPoint::COMPUTE, self.pipeline);

            let uni = context.per_frame_uniform.as_ref().unwrap();
            context.device.cmd_bind_descriptor_sets(command_buffer, vk::PipelineBindPoint::COMPUTE, self.pipeline_layout,
                                                    0, &[uni.descriptor_set, self.descriptor_set], &[]);

            let grid_data_bytes: &[u8] = unsafe { util::any_as_u8_slice(grid) };
            context.device.cmd_push_constants(command_buffer, self.pipeline_layout, vk::ShaderStageFlags::COMPUTE, 0, grid_data_bytes);

            context.device.cmd_dispatch(command_buffer, grid.dispatch_size, 1, 1);
            context.device.end_command_buffer(command_buffer);

            let ci = vk::SubmitInfo::builder().command_buffers(&[command_buffer])
                .wait_dst_stage_mask(&[vk::PipelineStageFlags::TOP_OF_PIPE])
                .wait_semaphores(wait_semaphore)
                .build();
            context.device.queue_submit(context.compute_queue, &[ci], vk::Fence::null());
        }
    }
}

#[repr(C)]
#[derive(Clone, Debug, Copy)]
struct GrassBlade {
    //position and direction (y angle)
    v0: Vec4,
    //bezier control point and height
    v1: Vec4,
    //physical model guide and width
    v2: Vec4,
    //update vector and stiffness
    up: Vec4,
}


pub struct GrassMgr {
    all_grass_blade_buffer: Buffer,
    visible_grass_blade_buffer: Buffer,
    pipeline: GraphicPipeline,
    compute_command_pool: vk::CommandPool,
    generate_command_buffer: vk::CommandBuffer,
    update_command_buffer: vk::CommandBuffer,
    has_gen_grass: bool,
    gen_compute: GrassGenerateCompute,
    update_compute: GrassUpdateCompute,
    grid: GrassGridData,

    draw_descriptor_layout: vk::DescriptorSetLayout,
    draw_descriptor_set: vk::DescriptorSet,
}

impl GrassMgr {
    pub fn destroy(&mut self, context: &RenderContext) {
        self.pipeline.destroy(context);
        self.all_grass_blade_buffer.destroy(context);
        self.visible_grass_blade_buffer.destroy(context);
        self.gen_compute.destroy(context);
        self.update_compute.destroy(context);
        unsafe {
            context.device.free_command_buffers(self.compute_command_pool, &[self.generate_command_buffer, self.update_command_buffer]);
            context.device.destroy_command_pool(self.compute_command_pool, None);
        }
    }

    pub fn create(context: &mut RenderContext, swap_mgr: &SwapChainMgr, render_pass: &ForwardRenderPass,
                  upload_command_buffer: vk::CommandBuffer) -> Self {
        let vb = [vk::VertexInputBindingDescription::builder()
            .binding(0)
            .stride(std::mem::size_of::<GrassBlade>() as _)
            .input_rate(vk::VertexInputRate::VERTEX).build()];

        let va = [
            //v0
            vk::VertexInputAttributeDescription::builder()
                .binding(0)
                .location(0)
                .format(vk::Format::R32G32B32A32_SFLOAT)
                .offset(0).build(),

            //v1
            vk::VertexInputAttributeDescription::builder()
                .binding(0)
                .location(1)
                .format(vk::Format::R32G32B32A32_SFLOAT)
                .offset(4 * 4).build(),

            //v2
            vk::VertexInputAttributeDescription::builder()
                .binding(0)
                .location(2)
                .format(vk::Format::R32G32B32A32_SFLOAT)
                .offset(4 * 4 * 2).build(),

            //up
            vk::VertexInputAttributeDescription::builder()
                .binding(0)
                .location(3)
                .format(vk::Format::R32G32B32A32_SFLOAT)
                .offset(4 * 4 * 3).build(),
        ];

        let (draw_descriptor_layout, draw_descriptor_set) = Self::create_descriptors(context, render_pass);

        let vi = PipelineVertexInputInfo::from_bap(&vb, &va, vk::PrimitiveTopology::PATCH_LIST);
        let uni = context.per_frame_uniform.as_ref().unwrap();
        let pipe_ci = vk::PipelineLayoutCreateInfo::builder().set_layouts(&[uni.descriptor_set_layout, draw_descriptor_layout])
            .build();

        let entry_point_name = CString::new("main").unwrap();
        let shaders = ShaderStages {
            vert: Some("grass_vert"),
            frag: Some("grass_frag"),
            tesc: Some("grass_tesc"),
            tese: Some("grass_tese"),
        }.to_shader_stage_create_info_array(context, &[], &entry_point_name);

        let pipeline = GraphicPipeline::create_with_info(context, swap_mgr, render_pass.get_native_render_pass(),
                                                         &vi, &pipe_ci, vk::SampleCountFlags::TYPE_1, &shaders);

        let compute_command_pool = {
            let pool_ci = vk::CommandPoolCreateInfo {
                queue_family_index: context.compute_queue_family_index,
                flags: vk::CommandPoolCreateFlags::RESET_COMMAND_BUFFER,
                ..Default::default()
            };
            unsafe {
                context.device.create_command_pool(&pool_ci, None).unwrap()
            }
        };

        let (generate_command_buffer, update_command_buffer) = {
            let command_ci = vk::CommandBufferAllocateInfo {
                command_pool: compute_command_pool,
                command_buffer_count: 2,
                level: vk::CommandBufferLevel::PRIMARY,
                ..Default::default()
            };
            unsafe {
                let commands = context.device.allocate_command_buffers(&command_ci).unwrap();
                (commands[0], commands[1])
            }
        };

        let mut grid_data = GrassGridData {
            grid_size: Vec2::new(100.0, 100.0),
            slot_size: Vec2::new(0.15, 0.15),
            slot_count: UVec2::default(),
            grass_y: 0.0,
            grass_count: 0,
            dispatch_size: 0,
        };

        let slot_count: UVec2 = (grid_data.grid_size / grid_data.slot_size).floor().as_u32();
        grid_data.slot_count = slot_count;
        let blade_count = slot_count.x * slot_count.y;
        grid_data.grass_count = blade_count;
        const COMPUTE_GROUP_LOCAL_SIZE: u32 = 32;
        grid_data.dispatch_size = (blade_count + COMPUTE_GROUP_LOCAL_SIZE - 1) / COMPUTE_GROUP_LOCAL_SIZE;

        let all_blade_size = blade_count * (size_of::<GrassBlade>() as u32);
        let all_grass_blade_buffer = Buffer::create_device_local_buffer_with_size(context,
                                                                                  upload_command_buffer,
                                                                                  vk::BufferUsageFlags::UNIFORM_BUFFER |
                                                                                      vk::BufferUsageFlags::VERTEX_BUFFER |
                                                                                      vk::BufferUsageFlags::STORAGE_BUFFER,
                                                                                  all_blade_size);

        let visible_grass_blade_buffer = Buffer::create_device_local_buffer_with_size(context,
                                                                                      upload_command_buffer,
                                                                                      vk::BufferUsageFlags::UNIFORM_BUFFER |
                                                                                          vk::BufferUsageFlags::VERTEX_BUFFER |
                                                                                          vk::BufferUsageFlags::STORAGE_BUFFER,
                                                                                      all_blade_size);

        let gen_compute = GrassGenerateCompute::create(context, swap_mgr, &all_grass_blade_buffer, &grid_data);
        let update_compute = GrassUpdateCompute::create(context, swap_mgr,
                                                        upload_command_buffer, &all_grass_blade_buffer, &visible_grass_blade_buffer);

        Self {
            compute_command_pool,
            generate_command_buffer,
            update_command_buffer,
            pipeline,
            all_grass_blade_buffer,
            visible_grass_blade_buffer,
            has_gen_grass: false,
            gen_compute,
            update_compute,
            grid: grid_data,

            draw_descriptor_layout,
            draw_descriptor_set,
        }
    }

    fn create_descriptors(context: &mut RenderContext,
                          render_pass: &ForwardRenderPass) -> (vk::DescriptorSetLayout, vk::DescriptorSet) {
        let bindings = [
            vk::DescriptorSetLayoutBinding::builder()
                .binding(0)
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
            let shadow = render_pass.get_shadow();
            let (view, sampler) = (shadow.shadow_view, shadow.sampler);

            [vk::DescriptorImageInfo::builder()
                .image_layout(vk::ImageLayout::DEPTH_STENCIL_READ_ONLY_OPTIMAL)
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
    }

    pub fn compute_grass_data(&mut self, context: &RenderContext) {
        if !self.has_gen_grass {
            self.gen_compute.compute(context, self.generate_command_buffer, &self.grid);
            self.update_compute.compute(context, self.update_command_buffer, &self.grid, &[self.gen_compute.working_semaphore]);
            self.has_gen_grass = true;
        } else {
            self.update_compute.compute(context, self.update_command_buffer, &self.grid, &[]);
        }
    }

    pub fn cmd_barrier(&self, context: &RenderContext, command_buffer: vk::CommandBuffer) {
        //barrier
        let barriers = [
            vk::BufferMemoryBarrier::builder()
                .buffer(self.visible_grass_blade_buffer.buffer)
                .offset(0)
                .size(self.visible_grass_blade_buffer.size as _)
                .src_access_mask(vk::AccessFlags::SHADER_WRITE)
                .dst_access_mask(vk::AccessFlags::INDIRECT_COMMAND_READ)
                .src_queue_family_index(context.compute_queue_family_index)
                .dst_queue_family_index(context.graphics_queue_family_index)
                .build(),
            // vk::BufferMemoryBarrier::builder()
            //     .buffer(self.update_compute.num_blades_buffer.buffer)
            //     .offset(0)
            //     .size(self.update_compute.num_blades_buffer.size as _)
            //     .src_access_mask(vk::AccessFlags::SHADER_WRITE)
            //     .dst_access_mask(vk::AccessFlags::INDIRECT_COMMAND_READ)
            //     .src_queue_family_index(context.compute_queue_family_index)
            //     .dst_queue_family_index(context.graphics_queue_family_index)
            //     .build(),
        ];

        unsafe {
            context.device.cmd_pipeline_barrier(command_buffer,
                                                vk::PipelineStageFlags::COMPUTE_SHADER,
                                                vk::PipelineStageFlags::DRAW_INDIRECT,
                                                vk::DependencyFlags::empty(), &[], &barriers, &[]);
        }
    }

    pub fn draw(&self, context: &RenderContext, command_buffer: vk::CommandBuffer) {
        let uni = context.per_frame_uniform.as_ref().unwrap();
        let pipe = self.pipeline.get_pipeline();
        unsafe {
            context.device.cmd_bind_pipeline(command_buffer, vk::PipelineBindPoint::GRAPHICS, pipe);
            context.device.cmd_bind_vertex_buffers(command_buffer, 0, &[self.visible_grass_blade_buffer.buffer], &[0]);
            context.device.cmd_bind_descriptor_sets(command_buffer, vk::PipelineBindPoint::GRAPHICS,
                                                    self.pipeline.get_layout(), 0, &[uni.descriptor_set, self.draw_descriptor_set], &[]);
            context.device.cmd_draw_indirect(command_buffer, self.update_compute.num_blades_buffer.buffer, 0, 1, 0);
        }
    }
}