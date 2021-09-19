use bevy::{
    asset::{AssetLoader, LoadContext, LoadedAsset},
    prelude::*,
    reflect::TypeUuid,
    utils::BoxedFuture,
};

#[derive(Debug)]
pub enum GltfData {
    Parsed,
    Raw {
        document: gltf::Document,
        buffers: Vec<gltf::buffer::Data>,
        images: Vec<gltf::image::Data>,
    },
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
        if let GltfData::Raw {document, buffers, images} = &self.data {
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

