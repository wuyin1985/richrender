use ash::vk;
use crate::render::render_context::RenderContext;
use crate::render::swapchain_mgr::SwapChainMgr;
use std::ffi::CString;
use std::path::Path;
use std::io::Cursor;

pub struct PipelineVertexInputInfo {
    ci: vk::PipelineVertexInputStateCreateInfo,
}

impl PipelineVertexInputInfo {
    pub fn from(binding: &[vk::VertexInputBindingDescription], attributes: &[vk::VertexInputAttributeDescription]) -> Self {
        PipelineVertexInputInfo {
            ci: vk::PipelineVertexInputStateCreateInfo::builder().
                vertex_binding_descriptions(binding).vertex_attribute_descriptions(attributes).build()
        }
    }

    pub fn get_ci(&self) -> &vk::PipelineVertexInputStateCreateInfo {
        &self.ci
    }
}

pub struct PipelineLayoutInfo {
    ci: vk::PipelineLayoutCreateInfo,
}

impl PipelineLayoutInfo {
    pub fn from(ci: vk::PipelineLayoutCreateInfo) -> Self {
        PipelineLayoutInfo {
            ci
        }
    }
    pub fn get_ci(&self) -> &vk::PipelineLayoutCreateInfo {
        &self.ci
    }
}

pub struct GraphicPipeline {
    pipeline: vk::Pipeline,
    pipeline_layout: vk::PipelineLayout,
}

impl GraphicPipeline {
    fn read_shader_data_from_file(context: &mut RenderContext, path: &str, defines: &[&str]) -> vk::ShaderModule {
        context.shader_modules.create_shader(&context.device, path, defines)
    }

    pub fn destroy(&mut self, device_mgr: &RenderContext) {
        unsafe {
            device_mgr.device.destroy_pipeline_layout(self.pipeline_layout, None);
            device_mgr.device.destroy_pipeline(self.pipeline, None);
        }
    }

    pub fn create(device_mgr: &mut RenderContext,
                  swapchain_mgr: &SwapChainMgr,
                  render_pass: vk::RenderPass,
                  vertex_input: &PipelineVertexInputInfo,
                  pipeline_layout_ci: &vk::PipelineLayoutCreateInfo,
                  msaa: vk::SampleCountFlags,
                  vert_spv_path: &str,
                  frag_spv_path: &str,
                  defines: &[&str]) -> Self {

        let vertex_shader_module = Self::read_shader_data_from_file(device_mgr, vert_spv_path, defines);
        let fragment_shader_module = Self::read_shader_data_from_file(device_mgr, frag_spv_path, defines);

        let device = &device_mgr.device;
        let entry_point_name = CString::new("main").unwrap();
        let vertex_shader_state_info = vk::PipelineShaderStageCreateInfo::builder()
            .stage(vk::ShaderStageFlags::VERTEX)
            .module(vertex_shader_module)
            .name(&entry_point_name)
            .build();
        let fragment_shader_state_info = vk::PipelineShaderStageCreateInfo::builder()
            .stage(vk::ShaderStageFlags::FRAGMENT)
            .module(fragment_shader_module)
            .name(&entry_point_name)
            .build();
        let shader_states_infos = [vertex_shader_state_info, fragment_shader_state_info];


        let input_assembly_info = vk::PipelineInputAssemblyStateCreateInfo::builder()
            .topology(vk::PrimitiveTopology::TRIANGLE_LIST)
            .primitive_restart_enable(false)
            .build();

        let surface_resolution = swapchain_mgr.surface_resolution;

        let viewport = vk::Viewport {
            x: 0.0,
            y: 0.0,
            width: surface_resolution.width as _,
            height: surface_resolution.height as _,
            min_depth: 0.0,
            max_depth: 1.0,
        };
        let viewports = [viewport];
        let scissor = vk::Rect2D {
            offset: vk::Offset2D { x: 0, y: 0 },
            extent: surface_resolution,
        };
        let scissors = [scissor];
        let viewport_info = vk::PipelineViewportStateCreateInfo::builder()
            .viewports(&viewports)
            .scissors(&scissors)
            .build();

        let rasterizer_info = vk::PipelineRasterizationStateCreateInfo::builder()
            .depth_clamp_enable(false)
            .rasterizer_discard_enable(false)
            .polygon_mode(vk::PolygonMode::FILL)
            .line_width(1.0)
            .cull_mode(vk::CullModeFlags::BACK)
            .front_face(vk::FrontFace::COUNTER_CLOCKWISE)
            .depth_bias_enable(false)
            .depth_bias_constant_factor(0.0)
            .depth_bias_clamp(0.0)
            .depth_bias_slope_factor(0.0)
            .build();

        let multisampling_info = vk::PipelineMultisampleStateCreateInfo::builder()
            .sample_shading_enable(false)
            .rasterization_samples(msaa)
            //.min_sample_shading(1.0)
            // .sample_mask() // null
            .alpha_to_coverage_enable(false)
            .alpha_to_one_enable(false)
            .build();

        let noop_stencil_state = vk::StencilOpState {
            fail_op: vk::StencilOp::KEEP,
            pass_op: vk::StencilOp::KEEP,
            depth_fail_op: vk::StencilOp::KEEP,
            compare_op: vk::CompareOp::ALWAYS,
            ..Default::default()
        };
        let depth_stencil_info = vk::PipelineDepthStencilStateCreateInfo {
            depth_test_enable: 1,
            depth_write_enable: 1,
            depth_compare_op: vk::CompareOp::LESS_OR_EQUAL,
            front: noop_stencil_state,
            back: noop_stencil_state,
            max_depth_bounds: 1.0,
            ..Default::default()
        };

        let color_blend_attachment = vk::PipelineColorBlendAttachmentState::builder()
            .color_write_mask(vk::ColorComponentFlags::all())
            .blend_enable(false)
            .src_color_blend_factor(vk::BlendFactor::ONE)
            .dst_color_blend_factor(vk::BlendFactor::ZERO)
            .color_blend_op(vk::BlendOp::ADD)
            .src_alpha_blend_factor(vk::BlendFactor::ONE)
            .dst_alpha_blend_factor(vk::BlendFactor::ZERO)
            .alpha_blend_op(vk::BlendOp::ADD)
            .build();
        let color_blend_attachments = [color_blend_attachment];

        let color_blending_info = vk::PipelineColorBlendStateCreateInfo::builder()
            .logic_op_enable(false)
            .logic_op(vk::LogicOp::COPY)
            .attachments(&color_blend_attachments)
            .blend_constants([0.0, 0.0, 0.0, 0.0])
            .build();

        let layout = {
            unsafe { device.create_pipeline_layout(pipeline_layout_ci, None).unwrap() }
        };

        let pipeline_info = vk::GraphicsPipelineCreateInfo::builder()
            .stages(&shader_states_infos)
            .vertex_input_state(vertex_input.get_ci())
            .input_assembly_state(&input_assembly_info)
            .viewport_state(&viewport_info)
            .rasterization_state(&rasterizer_info)
            .multisample_state(&multisampling_info)
            .depth_stencil_state(&depth_stencil_info)
            .color_blend_state(&color_blending_info)
            // .dynamic_state() null since don't have any dynamic states
            .layout(layout)
            .render_pass(render_pass)
            .subpass(0)
            // .base_pipeline_handle() null since it is not derived from another
            // .base_pipeline_index(-1) same
            .build();
        let pipeline_infos = [pipeline_info];

        let pipeline = unsafe {
            device
                .create_graphics_pipelines(vk::PipelineCache::null(), &pipeline_infos, None)
                .unwrap()[0]
        };

        // unsafe {
        //     device.destroy_shader_module(vertex_shader_module, None);
        //     device.destroy_shader_module(fragment_shader_module, None);
        // };

        GraphicPipeline {
            pipeline,
            pipeline_layout: layout,
        }
    }

    pub fn get_pipeline(&self) -> vk::Pipeline {
        self.pipeline
    }

    pub fn get_layout(&self) -> vk::PipelineLayout {
        self.pipeline_layout
    }
}