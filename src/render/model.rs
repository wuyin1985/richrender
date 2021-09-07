use gltf;
use gltf::Gltf;
use glam;
use std::error::Error;
use crate::render::aabb::Aabb;
use std::mem::size_of;
use crate::render::material::Material;

struct Node {
    parent: Option<usize>,
    children: Option<Vec<usize>>,
    translation: glam::Vec3,
    scale: glam::Vec3,
    rotation: glam::Quat,
    mesh: Option<usize>,
}


struct Model {
    // root: usize,
    // nodes: Vec<Node>,
}


fn test_load_gltf(path: &str) -> Result<Model, Box<dyn Error>> {
   

    Ok(Model {})
}