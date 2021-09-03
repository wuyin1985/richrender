use ash::vk;
use std::ffi::CString;
use crate::render::device_mgr::DeviceMgr;
use crate::render::swapchain_mgr::SwapChainMgr;
use std::mem::size_of;
use std::io::Cursor;
use std::fs::File;
use std::path::Path;

fn load_from_assets<P: AsRef<Path>>(path: P) -> Cursor<Vec<u8>> {
    use std::fs::File;
    use std::io::Read;

    let mut buf = Vec::new();
    let fullpath = &Path::new("assets").join(&path);
    let mut file = File::open(&fullpath).unwrap();
    file.read_to_end(&mut buf).unwrap();
    Cursor::new(buf)
}


#[derive(Clone, Copy)]
#[allow(dead_code)]
struct Vertex {
    pos: [f32; 3],
    color: [f32; 3],
    coords: [f32; 2],
}

impl Vertex {
    fn get_binding_description() -> vk::VertexInputBindingDescription {
        vk::VertexInputBindingDescription::builder()
            .binding(0)
            .stride(size_of::<Vertex>() as _)
            .input_rate(vk::VertexInputRate::VERTEX)
            .build()
    }

    fn get_attribute_descriptions() -> [vk::VertexInputAttributeDescription; 3] {
        let position_desc = vk::VertexInputAttributeDescription::builder()
            .binding(0)
            .location(0)
            .format(vk::Format::R32G32B32_SFLOAT)
            .offset(0)
            .build();
        let color_desc = vk::VertexInputAttributeDescription::builder()
            .binding(0)
            .location(1)
            .format(vk::Format::R32G32B32_SFLOAT)
            .offset(12)
            .build();
        let coords_desc = vk::VertexInputAttributeDescription::builder()
            .binding(0)
            .location(2)
            .format(vk::Format::R32G32_SFLOAT)
            .offset(24)
            .build();
        [position_desc, color_desc, coords_desc]
    }
}


pub struct SimpleDrawObject {
    graphic_pipeline: vk::Pipeline,
}

impl SimpleDrawObject {
    pub fn create(device_mgr: &DeviceMgr, swapchain_mgr: &SwapChainMgr, render_pass: vk::RenderPass) -> Self {
        SimpleDrawObject {
            graphic_pipeline: Self::create_graphic_pipeline(device_mgr,
                                                            swapchain_mgr,
                                                            render_pass,
                                                            device_mgr.render_config.msaa,
                                                            "spv/simple_draw_object_vert.spv",
                                                            "spv/simple_draw_object_frag.spv")
        }
    }

    fn read_shader_data_from_file(device_mgr: &DeviceMgr, path: &str) -> vk::ShaderModule {
        let mut cursor = load_from_assets(path);
        let res = ash::util::read_spv(&mut cursor).expect(format!("failed to read spv {}", path).as_str());
        let create_info = vk::ShaderModuleCreateInfo::builder().code(res.as_slice()).build();
        unsafe { device_mgr.device.create_shader_module(&create_info, None).unwrap() }
    }

    fn create_graphic_pipeline(device_mgr: &DeviceMgr, swapchain_mgr: &SwapChainMgr, render_pass: vk::RenderPass,
                               msaa: vk::SampleCountFlags, vert_spv_path: &str, frag_spv_path: &str) -> vk::Pipeline {
        let device = &device_mgr.device;

        let vertex_shader_module = Self::read_shader_data_from_file(device_mgr, vert_spv_path);
        let fragment_shader_module = Self::read_shader_data_from_file(device_mgr, frag_spv_path);

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

        // let vertex_binding_descs = [Vertex::get_binding_description()];
        // let vertex_attribute_descs = Vertex::get_attribute_descriptions();
        // let vertex_input_info = vk::PipelineVertexInputStateCreateInfo::builder()
        //     .vertex_binding_descriptions(&vertex_binding_descs)
        //     .vertex_attribute_descriptions(&vertex_attribute_descs)
        //     .build();

        let vertex_input_info = vk::PipelineVertexInputStateCreateInfo {
            vertex_attribute_description_count: 0,
            vertex_binding_description_count: 0,
            ..Default::default()
        };

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
            .min_sample_shading(1.0)
            // .sample_mask() // null
            .alpha_to_coverage_enable(false)
            .alpha_to_one_enable(false)
            .build();

        let depth_stencil_info = vk::PipelineDepthStencilStateCreateInfo::builder()
            .depth_test_enable(true)
            .depth_write_enable(true)
            .depth_compare_op(vk::CompareOp::LESS)
            .depth_bounds_test_enable(false)
            .min_depth_bounds(0.0)
            .max_depth_bounds(1.0)
            .stencil_test_enable(false)
            .front(Default::default())
            .back(Default::default())
            .build();

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
            //let layouts = [descriptor_set_layout];
            let layout_info = vk::PipelineLayoutCreateInfo::builder()
                //.set_layouts(&layouts)
                // .push_constant_ranges
                .build();

            unsafe { device.create_pipeline_layout(&layout_info, None).unwrap() }
        };

        let pipeline_info = vk::GraphicsPipelineCreateInfo::builder()
            .stages(&shader_states_infos)
            .vertex_input_state(&vertex_input_info)
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

        unsafe {
            device.destroy_shader_module(vertex_shader_module, None);
            device.destroy_shader_module(fragment_shader_module, None);
        };

        pipeline
    }

    pub fn draw(&self, device_mgr: &DeviceMgr, command_buffer: vk::CommandBuffer) {
        unsafe {
            device_mgr.device.cmd_bind_pipeline(command_buffer, vk::PipelineBindPoint::GRAPHICS, self.graphic_pipeline);
            device_mgr.device.cmd_draw(command_buffer, 3, 1, 0, 0);
        }
    }
}