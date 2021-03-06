use std::mem::size_of;
use ash::vk;
use crate::render::material::Material;
use crate::render::aabb::Aabb;
use crate::render::buffer::Buffer;
use crate::render::render_context::RenderContext;
use crate::render::vertex::*;
use core::mem;
use crate::render::vertex_layout::VertexLayout;
use std::io::Write;
use bevy::prelude::*;
use crate::render::shader_const::*;


pub struct Mesh {
    primitives: Vec<Primitive>,
    aabb: Aabb,
}

impl Mesh {
    fn new(primitives: Vec<Primitive>) -> Self {
        let aabbs = primitives.iter().map(|p| p.aabb()).collect::<Vec<_>>();
        let aabb = Aabb::union(&aabbs).unwrap_or(Aabb::new(Vec3::ZERO, Vec3::ZERO));
        Mesh { primitives, aabb }
    }

    pub fn get_primitives(&self) -> &Vec<Primitive> {
        &self.primitives
    }
}

impl Mesh {
    pub fn primitives(&self) -> &[Primitive] {
        &self.primitives
    }

    pub fn primitive_count(&self) -> usize {
        self.primitives.len()
    }

    pub fn aabb(&self) -> Aabb {
        self.aabb
    }
}

pub struct Meshes {
    pub meshes: Vec<Mesh>,
    pub buffer: Buffer,
}

impl Meshes {
    pub fn destroy(&mut self, context: &RenderContext) {
        unsafe {
            self.buffer.destroy(context);
        }
    }

    pub fn from_gltf(context: &mut RenderContext,
                     upload_command_buffer: vk::CommandBuffer,
                     document: &gltf::Document,
                     buffers: &Vec<gltf::buffer::Data>) -> Meshes {
        load_meshes(context, upload_command_buffer, document, buffers)
    }
}

pub struct Primitive {
    index: usize,
    material: Material,
    aabb: Aabb,
    vertex_layout: VertexLayout,
}

impl Primitive {
    pub fn index(&self) -> usize {
        self.index
    }

    pub fn indices(&self) -> &BufferPart {
        &self.vertex_layout.indices
    }

    pub fn material(&self) -> Material {
        self.material
    }

    pub fn aabb(&self) -> Aabb {
        self.aabb
    }

    pub fn get_material(&self) -> &Material {
        &self.material
    }

    pub fn get_vertex_layout(&self) -> &VertexLayout {
        &self.vertex_layout
    }
}


fn get_aabb(bounds: &gltf::mesh::Bounds<[f32; 3]>) -> Aabb {
    let min = bounds.min;
    let min = Vec3::new(min[0], min[1], min[2]);

    let max = bounds.max;
    let max = Vec3::new(max[0], max[1], max[2]);

    Aabb::new(min, max)
}

fn buffer_view_slice<'a, 's>(
    view: gltf::buffer::View<'a>,
    buffers: &'s Vec<gltf::buffer::Data>,
) -> &'s [u8] {
    let start = view.offset();
    let end = start + view.length();
    let buffer = &buffers[view.buffer().index()];
    &buffer[start..end]
}

fn read_no_sparse_buffer(accessor: &gltf::accessor::Accessor, datas: &Vec<gltf::buffer::Data>, element_count: u32) -> (Vec<u8>, usize, usize) {
    assert!(accessor.sparse().is_none(), "load sparse data is not supported");
    let view = accessor.view().expect("not view found");
    let stride = VertexLayout::calculate_stride(accessor.data_type(), element_count) as usize;
    let v_stride = if let Some(v) = view.stride() {
        //assert_eq!(stride, v_stride as u32, "error stride");
        v
    } else {
        stride
    };

    let view_slice = buffer_view_slice(view, datas);
    let start = accessor.offset();
    let count = accessor.count();

    if stride == v_stride {
        let end = start + (stride as usize) * count;
        return ((&view_slice[start..end]).to_vec(), count, v_stride);
    }

    let mut ret = vec![];
    let data_len = view_slice.len();
    for i in 0..count {
        let si = start + i * v_stride;
        let mut ei = si + v_stride;
        if ei > data_len {
            ei = data_len;
            assert_eq!(i, count - 1, "error data len");
        }

        ret.extend_from_slice(&view_slice[si..ei]);
    }

    (ret, count, v_stride)
}

fn read_no_sparse_vertex_data(primitive: &gltf::mesh::Primitive,
                              data_type: &gltf::Semantic,
                              datas: &Vec<gltf::buffer::Data>,
                              element_count: u32,
                              output_data: &mut Vec<u8>,
                              layout: &mut VertexLayout,
                              location: u32) {
    if let Some(accessor) = &primitive.get(data_type) {
        let (data, count, stride) = read_no_sparse_buffer(accessor, datas, element_count);
        let mut offset = output_data.len();
        output_data.extend(data);
        const MOD: usize = 4;
        let m = output_data.len() % MOD;
        if m != 0 {
            let left = MOD - m;
            let ar = vec![0; left];
            output_data.extend(ar);
            offset += left;
        }

        layout.push_meta(accessor.data_type(), element_count, offset, location, stride as _);
    }
}


fn load_meshes(context: &mut RenderContext, upload_command_buffer: vk::CommandBuffer,
               document: &gltf::Document, buffers: &Vec<gltf::buffer::Data>) -> Meshes {
    let mut primitive_count = 0;
    let mut all_data = Vec::<u8>::new();
    let mut meshes = Vec::<Mesh>::new();

    for mesh in document.meshes() {
        let mut primitives_buffers = Vec::<Primitive>::new();
        for primitive in mesh.primitives() {
            assert_eq!(primitive.mode(), gltf::mesh::Mode::Triangles, "error mode");

            let mut vertex_layout = VertexLayout::create();
            let indices_offset = all_data.len();

            let indices_accessor = &primitive.indices().expect("no indices");
            let (indices, indices_count, stride) = read_no_sparse_buffer(indices_accessor, buffers, 1);
            all_data.extend(indices);


            vertex_layout.set_indices(indices_offset, indices_count, indices_accessor.data_type());
            read_no_sparse_vertex_data(&primitive, &gltf::Semantic::Positions, buffers, 3, &mut all_data, &mut vertex_layout, LOCATION_IN_POS);
            read_no_sparse_vertex_data(&primitive, &gltf::Semantic::Normals, buffers, 3, &mut all_data, &mut vertex_layout, LOCATION_IN_NORMAL);
            read_no_sparse_vertex_data(&primitive, &gltf::Semantic::TexCoords(0), buffers, 2, &mut all_data, &mut vertex_layout, LOCATION_IN_TEX_COORD);
            read_no_sparse_vertex_data(&primitive, &gltf::Semantic::Weights(0), buffers, 4, &mut all_data, &mut vertex_layout, LOCATION_IN_WEIGHTS);
            read_no_sparse_vertex_data(&primitive, &gltf::Semantic::Joints(0), buffers, 4, &mut all_data, &mut vertex_layout, LOCATION_IN_JOINTS);

            vertex_layout.refresh_buffer_offsets();

            let material = primitive.material().into();
            let primitive_index = primitive_count;
            primitive_count += 1;

            let aabb = get_aabb(&primitive.bounding_box());

            primitives_buffers.push(Primitive {
                index: primitive_index,
                vertex_layout,
                material,
                aabb,
            });
        }

        meshes.push(Mesh::new(primitives_buffers))
    }


    let buffer =
        Buffer::create_device_local_buffer(context, upload_command_buffer, vk::BufferUsageFlags::INDEX_BUFFER | vk::BufferUsageFlags::VERTEX_BUFFER, &all_data);


    Meshes {
        meshes,
        buffer,
    }
}