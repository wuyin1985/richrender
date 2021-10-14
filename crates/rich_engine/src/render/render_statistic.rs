use ash::vk;

pub struct RenderStatistic {
    stats_values: Vec<u64>,
    stats_names: Vec<&'static str>,
    query_pool: vk::QueryPool,
}

impl RenderStatistic {
    pub fn destroy(&mut self, device: &ash::Device) {
        unsafe {
            device.destroy_query_pool(self.query_pool, None);
        }
    }

    pub fn create(device: &ash::Device) -> Self {
        let stats_names = vec![
            "Input assembly vertex count",
            "Input assembly primitives count",
            "Vertex shader invocations",
            "Clipping stage primitives processed",
            "Clipping stage primitives output",
            "Fragment shader invocations",
            "Tess. control shader patches",
            "Tess. eval. shader invocations",
        ];

        let stats_values = vec![0u64; stats_names.len() as _];

        let qci = vk::QueryPoolCreateInfo::builder()
            .query_type(vk::QueryType::PIPELINE_STATISTICS)
            .pipeline_statistics(
                vk::QueryPipelineStatisticFlags::INPUT_ASSEMBLY_VERTICES |
                    vk::QueryPipelineStatisticFlags::INPUT_ASSEMBLY_PRIMITIVES |
                    vk::QueryPipelineStatisticFlags::VERTEX_SHADER_INVOCATIONS |
                    vk::QueryPipelineStatisticFlags::CLIPPING_INVOCATIONS |
                    vk::QueryPipelineStatisticFlags::CLIPPING_PRIMITIVES |
                    vk::QueryPipelineStatisticFlags::FRAGMENT_SHADER_INVOCATIONS |
                    vk::QueryPipelineStatisticFlags::TESSELLATION_CONTROL_SHADER_PATCHES |
                    vk::QueryPipelineStatisticFlags::TESSELLATION_EVALUATION_SHADER_INVOCATIONS
            )
            .query_count(1)
            .build();

        let query_pool = unsafe {
            device.create_query_pool(&qci, None).expect("failed to create query pool")
        };

        Self {
            stats_names,
            stats_values,
            query_pool,
        }
    }

    pub fn require_results(&mut self, device: &ash::Device) {
        unsafe {
            // device.get_query_pool_results(self.query_pool, 0, 1, &mut self.stats_values,
            //                               vk::QueryResultFlags::TYPE_64);
        }
    }

    pub fn begin_query(&self, device: &ash::Device, command_buffer: vk::CommandBuffer) {
        unsafe {
            // device.cmd_reset_query_pool(command_buffer, self.query_pool, 0, 1);
            // device.cmd_begin_query(command_buffer, self.query_pool, 0, vk::QueryControlFlags::empty());
        }
    }

    pub fn end_query(&self, device: &ash::Device, command_buffer: vk::CommandBuffer) {
        unsafe {
            //device.cmd_end_query(command_buffer, self.query_pool, 0);
        }
    }
}
