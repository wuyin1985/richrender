use ash::vk;
use gltf::accessor::DataType;
use crate::render::vertex::BufferPart;
use crate::render::shader_const;

pub struct VertexMeta {
    format: vk::Format,
    size: u32,
    location: u32,
}

pub struct VertexLayout {
    pub metas: Vec<VertexMeta>,
    pub offsets: Vec<usize>,
    pub indices: BufferPart,
    pub indices_type: vk::IndexType,
    pub buffers_ref_offsets: Vec<vk::DeviceSize>,
}


impl VertexLayout {
    pub fn gltf_data_type_vk_index(data_type: DataType) -> vk::IndexType {
        match data_type {
            DataType::U16 => vk::IndexType::UINT16,
            DataType::U32 => vk::IndexType::UINT32,
            _ => { panic!("unsupported data type {:?}", data_type) }
        }
    }

    pub fn gltf_data_type_2_vk_format(data_type: DataType, data_count: u32) -> vk::Format {
        match (data_type, data_count) {

            //indices
            (DataType::U16, 1) => vk::Format::R16_UINT,
            (DataType::U32, 1) => vk::Format::R32_UINT,

            //tex_coord
            (DataType::U8, 2) => vk::Format::R8G8_UINT,
            (DataType::U32, 2) => vk::Format::R32G32_UINT,
            (DataType::F32, 2) => vk::Format::R32G32_SFLOAT,

            //position 
            (DataType::F32, 3) => vk::Format::R32G32B32_SFLOAT,

            //weights
            (DataType::F32, 4) => vk::Format::R32G32B32A32_SFLOAT,

            //joints
            (DataType::U8, 4) => vk::Format::R8G8B8A8_UINT,
            (DataType::U32, 4) => vk::Format::R32G32B32A32_UINT,

            _ => { panic!("unsupported gltf data type {:?} {}", data_type, data_count) }
        }
    }

    pub fn vk_format_size(format: vk::Format) -> u32 {
        match format {
            vk::Format::R16_UINT => 2,
            vk::Format::R32_UINT => 4,
            vk::Format::R8G8_UINT => 2,
            vk::Format::R32G32_UINT => 8,
            vk::Format::R32G32_SFLOAT => 8,
            vk::Format::R32G32B32_SFLOAT => 12,
            vk::Format::R32G32B32A32_SFLOAT => 16,
            vk::Format::R8G8B8A8_UINT => 4,
            vk::Format::R32G32B32A32_UINT => 16,
            _ => { panic!("unsupported vk format {:?}", format) }
        }
    }

    pub fn calculate_stride(data_type: DataType, element_count: u32) -> u32 {
        let format = Self::gltf_data_type_2_vk_format(data_type, element_count);
        return Self::vk_format_size(format);
    }

    pub fn create() -> Self {
        VertexLayout {
            metas: Vec::new(),
            offsets: Vec::new(),
            indices: BufferPart { count: 0, index: 0 },
            indices_type: vk::IndexType::UINT32,
            buffers_ref_offsets: Vec::new(),
        }
    }

    pub fn set_indices(&mut self, indices_offset: usize, indices_count: usize, data_type: DataType) {
        self.indices = BufferPart { index: indices_offset, count: indices_count };
        self.indices_type = Self::gltf_data_type_vk_index(data_type);
    }

    pub fn push_meta(&mut self, data_type: DataType, element_count: u32, offset_in_buffer: usize, location: u32) {
        let format = Self::gltf_data_type_2_vk_format(data_type, element_count);
        let size = Self::vk_format_size(format);
        self.metas.push(VertexMeta {
            format,
            size,
            location,
        });
        self.offsets.push(offset_in_buffer);
    }

    pub fn refresh_buffer_offsets(&mut self) {
        self.buffers_ref_offsets = self.offsets.iter().map(|offset| *offset as vk::DeviceSize).collect::<Vec<_>>();
    }

    pub fn build_vk_bindings(&self) -> Vec<vk::VertexInputBindingDescription> {
        self.metas.iter().enumerate().map(|(index, meta)| {
            vk::VertexInputBindingDescription::builder()
                .binding(index as _)
                .stride(meta.size)
                .input_rate(vk::VertexInputRate::VERTEX)
                .build()
        }).collect()
    }

    pub fn build_vk_attributes(&self) -> Vec<vk::VertexInputAttributeDescription> {
        self.metas.iter().enumerate().map(|(index, meta)| {
            vk::VertexInputAttributeDescription::builder()
                .binding(index as _)
                .location(meta.location)
                .offset(0)
                .format(meta.format)
                .build()
        }).collect()
    }

    pub fn get_shader_defines(&self) -> Vec<&str> {
        self.metas.iter().filter_map(|meta| {
            shader_const::get_shader_define_name(meta.location)
        }).collect::<Vec<&str>>()
    }
}