use ash::vk;
use crate::render::render_context::RenderContext;
use crate::render::swapchain_mgr::SwapChainMgr;
use std::ffi::CString;
use std::path::Path;
use std::io::Cursor;
use ash::vk::DeviceSize;

fn read_shader_data_from_file(context: &mut RenderContext, path: &str, defines: &[&str]) -> vk::ShaderModule {
    context.shader_modules.create_shader(&context.device, path, defines)
}

pub struct PipelineVertexInputInfo {
    ci: Option<vk::PipelineVertexInputStateCreateInfo>,
    primitive: vk::PrimitiveTopology,
    cull_mode: vk::CullModeFlags,
}

impl PipelineVertexInputInfo {
    pub fn from_bap(binding: &[vk::VertexInputBindingDescription], attributes:
    &[vk::VertexInputAttributeDescription], primitive: vk::PrimitiveTopology, cull: vk::CullModeFlags) -> Self {
        PipelineVertexInputInfo {
            ci: Some(vk::PipelineVertexInputStateCreateInfo::builder().
                vertex_binding_descriptions(binding).vertex_attribute_descriptions(attributes)
                .build()),
            primitive: primitive,
            cull_mode: cull,
        }
    }

    pub fn from(binding: &[vk::VertexInputBindingDescription], attributes: &[vk::VertexInputAttributeDescription]) -> Self {
        PipelineVertexInputInfo {
            ci: Some(vk::PipelineVertexInputStateCreateInfo::builder().
                vertex_binding_descriptions(binding).vertex_attribute_descriptions(attributes)
                .build()),
            primitive: vk::PrimitiveTopology::TRIANGLE_LIST,
            cull_mode: vk::CullModeFlags::BACK,
        }
    }

    pub fn none() -> Self {
        PipelineVertexInputInfo {
            ci: None,
            primitive: vk::PrimitiveTopology::TRIANGLE_LIST,
            cull_mode: vk::CullModeFlags::BACK,
        }
    }

    pub fn get_ci(&self) -> &Option<vk::PipelineVertexInputStateCreateInfo> {
        &self.ci
    }

    pub fn get_primitive(&self) -> vk::PrimitiveTopology {
        self.primitive
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

pub struct ShaderStages {
    pub vert: Option<&'static str>,
    pub frag: Option<&'static str>,
    pub tesc: Option<&'static str>,
    pub tese: Option<&'static str>,
}

impl ShaderStages {
    fn file_to_shader_stage(context: &mut RenderContext, file_path: Option<&'static str>,
                            stage_flags: vk::ShaderStageFlags,
                            defines: &[&str],
                            res: &mut Vec<vk::PipelineShaderStageCreateInfo>,
                            entry_point_name: &CString)
    {
        if let Some(vt) = file_path {
            let module = read_shader_data_from_file(context, vt, defines);
            let shader_state_info = vk::PipelineShaderStageCreateInfo::builder()
                .stage(stage_flags)
                .module(module)
                .name(&entry_point_name)
                .build();

            res.push(shader_state_info);
        }
    }


    pub fn to_shader_stage_create_info_array(&self, context: &mut RenderContext, defines: &[&str], entry_point_name: &CString) ->
    Vec<vk::PipelineShaderStageCreateInfo>
    {
        let mut res = Vec::new();
        Self::file_to_shader_stage(context, self.vert, vk::ShaderStageFlags::VERTEX, defines, &mut res, entry_point_name);
        Self::file_to_shader_stage(context, self.frag, vk::ShaderStageFlags::FRAGMENT, defines, &mut res, entry_point_name);
        Self::file_to_shader_stage(context, self.tesc, vk::ShaderStageFlags::TESSELLATION_CONTROL, defines, &mut res, entry_point_name);
        Self::file_to_shader_stage(context, self.tese, vk::ShaderStageFlags::TESSELLATION_EVALUATION, defines, &mut res, entry_point_name);
        res
    }
}

pub struct GraphicPipeline {
    pipeline: vk::Pipeline,
    pipeline_layout: vk::PipelineLayout,
}

impl GraphicPipeline {
    pub fn destroy(&mut self, device_mgr: &RenderContext) {
        unsafe {
            device_mgr.device.destroy_pipeline_layout(self.pipeline_layout, None);
            device_mgr.device.destroy_pipeline(self.pipeline, None);
        }
    }

    pub fn create_with_info(device_mgr: &mut RenderContext,
                            swapchain_mgr: &SwapChainMgr,
                            render_pass: vk::RenderPass,
                            vertex_input: &PipelineVertexInputInfo,
                            pipeline_layout_ci: &vk::PipelineLayoutCreateInfo,
                            msaa: vk::SampleCountFlags,
                            pipeline_stage_shader_create_info_array: &[vk::PipelineShaderStageCreateInfo]) -> Self {
        let device = &mut device_mgr.device;
        let input_assembly_info = vk::PipelineInputAssemblyStateCreateInfo::builder()
            .topology(vertex_input.primitive)
            .primitive_restart_enable(false)
            .build();

        let surface_resolution = swapchain_mgr.surface_resolution;

        let viewport = vk::Viewport {
            x: 0.0,
            y: surface_resolution.height as _,
            width: surface_resolution.width as _,
            height: -(surface_resolution.height as f32),
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
            .cull_mode(vertex_input.cull_mode)
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


        let pipeline_info = {
            let ti = vk::PipelineTessellationStateCreateInfo::builder()
                .patch_control_points(1).flags(vk::PipelineTessellationStateCreateFlags::empty()).build();

            let mut ci = vk::GraphicsPipelineCreateInfo::builder()
                .stages(pipeline_stage_shader_create_info_array)
                .input_assembly_state(&input_assembly_info)
                .viewport_state(&viewport_info)
                .rasterization_state(&rasterizer_info)
                .multisample_state(&multisampling_info)
                .depth_stencil_state(&depth_stencil_info)
                .color_blend_state(&color_blending_info)
                // .dynamic_state() null since don't have any dynamic states
                .layout(layout)
                .render_pass(render_pass)
                .subpass(0);

            if let Some(info) = vertex_input.get_ci().as_ref() {
                ci = ci.vertex_input_state(info);
            }

            let has_tesselation =
                pipeline_stage_shader_create_info_array.iter().any(|ci| ci.stage.contains(vk::ShaderStageFlags::TESSELLATION_CONTROL));

            if has_tesselation {
                ci = ci.tessellation_state(&ti);
            }

            // .base_pipeline_handle() null since it is not derived from another
            // .base_pipeline_index(-1) same
            ci.build()
        };
        let pipeline_infos = [pipeline_info];

        let pipeline = unsafe {
            device
                .create_graphics_pipelines(vk::PipelineCache::null(), &pipeline_infos, None)
                .unwrap()[0]
        };

        GraphicPipeline {
            pipeline,
            pipeline_layout: layout,
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
        let vertex_shader_module = read_shader_data_from_file(device_mgr, vert_spv_path, defines);
        let fragment_shader_module = read_shader_data_from_file(device_mgr, frag_spv_path, defines);

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

        Self::create_with_info(device_mgr, swapchain_mgr, render_pass, vertex_input,
                               pipeline_layout_ci, msaa, &shader_states_infos)
    }

    pub fn create_vert_only(device_mgr: &mut RenderContext,
                            swapchain_mgr: &SwapChainMgr,
                            render_pass: vk::RenderPass,
                            vertex_input: &PipelineVertexInputInfo,
                            pipeline_layout_ci: &vk::PipelineLayoutCreateInfo,
                            msaa: vk::SampleCountFlags,
                            vert_spv_path: &str,
                            defines: &[&str]) -> Self {
        let vertex_shader_module = read_shader_data_from_file(device_mgr, vert_spv_path, defines);

        let device = &device_mgr.device;
        let entry_point_name = CString::new("main").unwrap();
        let vertex_shader_state_info = vk::PipelineShaderStageCreateInfo::builder()
            .stage(vk::ShaderStageFlags::VERTEX)
            .module(vertex_shader_module)
            .name(&entry_point_name)
            .build();

        let shader_states_infos = [vertex_shader_state_info];

        let input_assembly_info = vk::PipelineInputAssemblyStateCreateInfo::builder()
            .topology(vk::PrimitiveTopology::TRIANGLE_LIST)
            .primitive_restart_enable(false)
            .build();


        let sd = device_mgr.render_config.shadow_map_dim;

        let mut surface_resolution = swapchain_mgr.surface_resolution;
        surface_resolution.width = sd as _;
        surface_resolution.height = sd as _;
        let viewport = vk::Viewport {
            x: 0.0,
            y: sd,
            width: sd,
            height: -sd,
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
            .cull_mode(vk::CullModeFlags::NONE)
            .front_face(vk::FrontFace::COUNTER_CLOCKWISE)
            .depth_bias_enable(true)
            .depth_bias_constant_factor(1.25)
            .depth_bias_clamp(0.0)
            .depth_bias_slope_factor(1.75)
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

        let layout = {
            unsafe { device.create_pipeline_layout(pipeline_layout_ci, None).unwrap() }
        };

        let pipeline_info = {
            let mut ci = vk::GraphicsPipelineCreateInfo::builder()
                .stages(&shader_states_infos)
                .input_assembly_state(&input_assembly_info)
                .viewport_state(&viewport_info)
                .rasterization_state(&rasterizer_info)
                .multisample_state(&multisampling_info)
                .depth_stencil_state(&depth_stencil_info)
                .layout(layout)
                .render_pass(render_pass)
                .subpass(0);

            if let Some(info) = vertex_input.get_ci().as_ref() {
                ci = ci.vertex_input_state(info);
            }

            ci.build()
        };
        let pipeline_infos = [pipeline_info];

        let pipeline = unsafe {
            device
                .create_graphics_pipelines(vk::PipelineCache::null(), &pipeline_infos, None)
                .unwrap()[0]
        };

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