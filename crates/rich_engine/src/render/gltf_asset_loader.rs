use bevy::{
    prelude::*,
    asset::{AssetLoader, LoadContext, LoadedAsset},
    reflect::TypeUuid,
    utils::BoxedFuture,
};
use crate::render::animation::Animations;
use crate::render::node::Nodes;
use crate::render::skin::Skin;

#[derive(Debug)]
pub enum GltfData {
    Parsed,
    Raw {
        document: gltf::Document,
        buffers: Vec<gltf::buffer::Data>,
        images: Vec<gltf::image::Data>,
    },
}


pub struct AnimationWithNodes {
    pub animations: Animations,
    pub nodes: Nodes,
    pub skins: Vec<Skin>,
}

impl AnimationWithNodes {
    pub fn create(animations: Animations, nodes: Nodes, skins: Vec<Skin>) -> Self {
        AnimationWithNodes { animations, nodes, skins }
    }
}

pub struct GltfAnimationRuntime {
    pub data: Option<AnimationWithNodes>,
    pub init: bool,
}

impl Default for GltfAnimationRuntime {
    fn default() -> Self {
        Self {
            data: None,
            init: false,
        }
    }
}

impl GltfAnimationRuntime {
    pub fn update_animation_nodes(&mut self, delta_time: f32) -> bool {
        if let Some(anim_nodes) = self.data.as_mut() {
            if anim_nodes.animations.update(&mut anim_nodes.nodes, delta_time) {
                //anim_nodes.nodes.transform(None);
                anim_nodes.nodes
                    .get_skins_transform()
                    .iter()
                    .for_each(|(index, transform)| {
                        let skin = &mut anim_nodes.skins[*index];
                        skin.compute_joints_matrices(*transform, &anim_nodes.nodes.nodes());
                    });

                return true;
            }
        }

        return false;
    }
}


#[derive(Debug, TypeUuid)]
#[uuid = "f779f9ea-41cd-48ad-a553-0894d84a4be7"]
pub struct GltfAsset {
    data: GltfData,
}

impl GltfAsset {
    pub fn export(&self) -> (
        &gltf::Document,
        &Vec<gltf::buffer::Data>,
        &Vec<gltf::image::Data>,
    )
    {
        if let GltfData::Raw { document, buffers, images } = &self.data {
            return (&document, &buffers, &images);
        }

        panic!("not raw")
    }

    pub fn set_parsed(&mut self) {
        self.data = GltfData::Parsed {}
    }
}

#[derive(Default)]
pub struct GltfAssetLoader;

impl AssetLoader for GltfAssetLoader {
    fn load<'a>(&'a self, bytes: &'a [u8], load_context: &'a mut LoadContext) -> BoxedFuture<'a, anyhow::Result<(), anyhow::Error>> {
        Box::pin(async move {
            let (document, buffers, images) = gltf::import_slice(bytes)?;
            let data = GltfAsset { data: GltfData::Raw { document, buffers, images } };
            load_context.set_default_asset(LoadedAsset::new(data));
            println!("load !! assets");
            Ok(())
        })
    }

    fn extensions(&self) -> &[&str] {
        &["gltf", "glb"]
    }
}

