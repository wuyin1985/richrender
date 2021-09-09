use std::mem::size_of;
use ash::vk;
use crate::render::material::Material;
use crate::render::aabb::Aabb;
use crate::render::buffer::Buffer;
use crate::render::render_context::RenderContext;
use crate::render::vertex::*;


pub struct Mesh {
    primitives: Vec<Primitive>,
    aabb: Aabb,
}

impl Mesh {
    fn new(primitives: Vec<Primitive>) -> Self {
        let aabbs = primitives.iter().map(|p| p.aabb()).collect::<Vec<_>>();
        let aabb = Aabb::union(&aabbs).unwrap();
        Mesh { primitives, aabb }
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
    meshes: Vec<Mesh>,
    vertices_buffer: Buffer,
    indices_buffer: Buffer,
}

impl Meshes {
    pub fn destroy(&mut self, context: &RenderContext) {
        unsafe {
            self.vertices_buffer.destroy(context);
            self.indices_buffer.destroy(context);
        }
    }

    pub fn from_gltf(context: &mut RenderContext, upload_command_buffer: vk::CommandBuffer,
                     document: &gltf::Document, buffers: &Vec<gltf::buffer::Data>) -> Meshes {
        load_meshes(context, upload_command_buffer, document, buffers)
    }
}

pub struct Primitive {
    index: usize,
    vertices: VertexBufferPart,
    indices: Option<IndexBufferPart>,
    material: Material,
    aabb: Aabb,
}

impl Primitive {
    pub fn index(&self) -> usize {
        self.index
    }

    pub fn vertices(&self) -> &VertexBufferPart {
        &self.vertices
    }

    pub fn indices(&self) -> &Option<IndexBufferPart> {
        &self.indices
    }

    pub fn material(&self) -> Material {
        self.material
    }

    pub fn aabb(&self) -> Aabb {
        self.aabb
    }
}

struct PrimitiveData {
    index: usize,
    indices: Option<IndexBufferPart>,
    vertices: VertexBufferPart,
    material: Material,
    aabb: Aabb,
}


fn get_aabb(bounds: &gltf::mesh::Bounds<[f32; 3]>) -> Aabb {
    let min = bounds.min;
    let min = glam::Vec3::new(min[0], min[1], min[2]);

    let max = bounds.max;
    let max = glam::Vec3::new(max[0], max[1], max[2]);

    Aabb::new(min, max)
}

fn read_positions<'a, 's, F>(reader: &gltf::mesh::Reader<'a, 's, F>) -> Vec<[f32; 3]>
    where
        F: Clone + Fn(gltf::Buffer<'a>) -> Option<&'s [u8]>,
{
    reader
        .read_positions()
        .expect("Position primitives should be present")
        .collect()
}

fn read_normals<'a, 's, F>(reader: &gltf::mesh::Reader<'a, 's, F>) -> Vec<[f32; 3]>
    where
        F: Clone + Fn(gltf::Buffer<'a>) -> Option<&'s [u8]>,
{
    reader
        .read_normals()
        .map_or(vec![], |normals| normals.collect())
}

fn read_tex_coords<'a, 's, F>(reader: &gltf::mesh::Reader<'a, 's, F>, channel: u32) -> Vec<[f32; 2]>
    where
        F: Clone + Fn(gltf::Buffer<'a>) -> Option<&'s [u8]>,
{
    reader
        .read_tex_coords(channel)
        .map_or(vec![], |coords| coords.into_f32().collect())
}

fn read_weights<'a, 's, F>(reader: &gltf::mesh::Reader<'a, 's, F>) -> Vec<[f32; 4]>
    where
        F: Clone + Fn(gltf::Buffer<'a>) -> Option<&'s [u8]>,
{
    reader
        .read_weights(0)
        .map_or(vec![], |weights| weights.into_f32().collect())
}

fn read_joints<'a, 's, F>(reader: &gltf::mesh::Reader<'a, 's, F>) -> Vec<[u32; 4]>
    where
        F: Clone + Fn(gltf::Buffer<'a>) -> Option<&'s [u8]>,
{
    reader.read_joints(0).map_or(vec![], |joints| {
        joints
            .into_u16()
            .map(|[x, y, z, w]| [u32::from(x), u32::from(y), u32::from(z), u32::from(w)])
            .collect()
    })
}

fn read_indices<'a, 's, F>(reader: &gltf::mesh::Reader<'a, 's, F>) -> Option<Vec<u32>>
    where
        F: Clone + Fn(gltf::Buffer<'a>) -> Option<&'s [u8]>,
{
    reader
        .read_indices()
        .map(|indices| indices.into_u32().collect::<Vec<_>>())
}

fn load_meshes(context: &mut RenderContext, upload_command_buffer: vk::CommandBuffer,
               document: &gltf::Document, buffers: &Vec<gltf::buffer::Data>) -> Meshes {
    let mut meshes_data = Vec::<Vec<PrimitiveData>>::new();
    let mut all_vertices = Vec::<Vertex>::new();
    let mut all_indices = Vec::<u32>::new();

    let mut primitive_count = 0;

    for mesh in document.meshes() {
        let mut primitives_buffers = Vec::<PrimitiveData>::new();
        for primitive in mesh.primitives() {
            let buffer_reader = primitive.reader(|buffer| Some(&buffers[buffer.index()]));
            if let Some(accessor) = primitive.get(&gltf::Semantic::Positions) {
                let aabb = get_aabb(&primitive.bounding_box());
                let positions = read_positions(&buffer_reader);
                if accessor.count() != positions.len() {
                    panic!("the count not same {} {}", accessor.count(), positions.len());
                }
                let normals = read_normals(&buffer_reader);
                let tex_coords = read_tex_coords(&buffer_reader, 0);
                let weights = read_weights(&buffer_reader);
                let joints = read_joints(&buffer_reader);

                let mut vertices = positions.iter().enumerate().map(|(index, position)| {
                    let position = *position;
                    let normal = *normals.get(index).unwrap_or(&[0.0, 0.0, 0.0]);
                    let tex_coord = *tex_coords.get(index).unwrap_or(&[0.0f32, 0.0f32]);
                    let weight = *weights.get(index).unwrap_or(&[0.0f32, 0.0f32, 0.0f32, 0.0f32]);
                    let joint = *joints.get(index).unwrap_or(&[0u32, 0u32, 0u32, 0u32]);

                    Vertex {
                        position,
                        normal,
                        tex_coord,
                        weight,
                        joint,
                    }
                }).collect::<Vec<_>>();

                let indices = read_indices(&buffer_reader).map(|indices| {
                    let offset = all_indices.len() * size_of::<u32>();
                    all_indices.extend_from_slice(&indices);
                    (offset, indices.len())
                });

                let vertex_offset = all_vertices.len() * size_of::<Vertex>();
                all_vertices.extend_from_slice(&vertices);

                let material = primitive.material().into();
                let primitive_index = primitive_count;
                primitive_count += 1;

                primitives_buffers.push(PrimitiveData {
                    index: primitive_index,
                    indices,
                    vertices: (vertex_offset, accessor.count()),
                    material,
                    aabb,
                })
            }
        }

        meshes_data.push(primitives_buffers);
    }

    if meshes_data.is_empty() {
        panic!("the mesh data is empty");
    }

    if all_indices.is_empty() {
        panic!("the indices is empty");
    }

    let indices_buffer =
        Buffer::create_device_local_buffer(context, upload_command_buffer, vk::BufferUsageFlags::INDEX_BUFFER, &all_indices);

    let vertices_buffer = Buffer::create_device_local_buffer(context, upload_command_buffer,
                                                             vk::BufferUsageFlags::VERTEX_BUFFER, &all_vertices);

    let meshes = meshes_data
        .iter()
        .map(|primitive_datas| {
            let primitives = primitive_datas
                .iter().map(|primitive_data| {
                Primitive {
                    index: primitive_data.index,
                    vertices: primitive_data.vertices,
                    indices: primitive_data.indices,
                    material: primitive_data.material,
                    aabb: primitive_data.aabb,
                }
            }).collect();

            Mesh::new(primitives)
        }).collect::<Vec<_>>();

    Meshes {
        meshes,
        vertices_buffer,
        indices_buffer,
    }
}