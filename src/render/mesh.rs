use std::mem::size_of;
use crate::render::material::Material;
use crate::render::aabb::Aabb;

#[derive(Clone, Copy, Debug)]
struct Vertex {
    position: [f32; 3],
    normal: [f32; 3],
    tex_coord: [f32; 2],
    weight: [f32; 4],
    joint: [u32; 4],
}

/// Vertex buffer byte offset / element count
type VertexBufferPart = (usize, usize);

/// Index buffer byte offset / element count
type IndexBufferPart = (usize, usize);

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

fn load_meshes(document: &gltf::Document, buffers: &Vec<gltf::buffer::Data>) {
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
}