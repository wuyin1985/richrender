use std::collections::HashSet;
use std::env;
use std::ffi::c_void;
use std::os::raw::c_int;
use bevy::asset::{AssetLoader, BoxedFuture, LoadContext, LoadedAsset};
use bevy::prelude::*;
use bevy::reflect::TypeUuid;
use crate::vfx::bindings::ReleaseEffectPrefab;

#[derive(Debug, TypeUuid)]
#[uuid = "bab83cf1-78b3-4f8f-b43f-d7e0d9d75099"]
pub(super) struct VfxAsset {
    pub id: i32,
}

#[derive(Default)]
pub(super) struct VfxAssetLoader;

impl AssetLoader for VfxAssetLoader {
    fn load<'a>(&'a self, bytes: &'a [u8], load_context: &'a mut LoadContext) -> BoxedFuture<'a, anyhow::Result<(), anyhow::Error>> {
        Box::pin(async move {
            let mut p = env::current_exe().unwrap().parent().unwrap().to_path_buf();
            p.push("assets");
            let dir = p.to_str().unwrap();
            let mut dir_utf16: Vec<u16> = dir.encode_utf16().collect();
            let c = '\0' as u16;
            dir_utf16.push(c);

            info!("start parse vfx {:?} ", load_context.path());

            use crate::vfx::bindings::*;
            let id = unsafe {
                LoadEffectPrefab(bytes.as_ptr() as *const c_void, bytes.len() as c_int, dir_utf16.as_ptr() as _)
            };

            let data = VfxAsset { id };
            load_context.set_default_asset(LoadedAsset::new(data));
            info!("parse vfx complete");
            Ok(())
        })
    }

    fn extensions(&self) -> &[&str] {
        &["efk"]
    }
}

// pub(super) fn load_vfx_2_device_system(mut vfx_asset_events: EventReader<AssetEvent<VfxAsset>>,
//                                        mut ResMut<Assets<VfxAsset>>,
//                                        mut commands: Commands) {
//     for event in vfx_asset_events.iter() {
//
//         let mut changed_set: HashSet<Handle<VfxAsset>> = HashSet::default();
//         let mut destroy_set: HashSet<Handle<VfxAsset>> = HashSet::default();
//
//         match event {
//             AssetEvent::Created { ref handle } => {
//                 changed_set.insert(handle.clone_weak());
//             }
//             AssetEvent::Modified { ref handle } => {
//                 changed_set.insert(handle.clone_weak());
//                 destroy_set.insert(handle.clone_weak());
//             }
//             AssetEvent::Removed { ref handle } => {
//                 destroy_set.insert(handle.clone_weak());
//                 changed_set.remove(handle);
//             }
//         }
//
//         for destroy_handle in &destroy_set {
//             let v =
//             ReleaseEffectPrefab()
//             info!("remove gltf asset");
//         }
//
//
//         for changed_handle in changed_set.iter() {
//
//         }
//     }
// }


pub struct VfxReq
{
    pub path: &'static str,
    pub pos: Vec3,
    pub rot: Quat,
}
