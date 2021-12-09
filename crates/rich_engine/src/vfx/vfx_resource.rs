use std::collections::HashSet;
use std::env;
use std::ffi::c_void;
use std::os::raw::c_int;
use std::path::PathBuf;
use bevy::asset::{AssetLoader, BoxedFuture, LoadContext, LoadedAsset};
use bevy::prelude::*;
use bevy::reflect::TypeUuid;
use crate::vfx::bindings::ReleaseEffectPrefab;

#[derive(Debug, TypeUuid)]
#[uuid = "bab83cf1-78b3-4f8f-b43f-d7e0d9d75099"]
pub(super) struct VfxAsset {
    pub id: Option<i32>,
    pub bytes: Vec<u8>,
    pub path: PathBuf,
}

#[derive(Default)]
pub(super) struct VfxAssetLoader;

impl AssetLoader for VfxAssetLoader {
    fn load<'a>(&'a self, bytes: &'a [u8], load_context: &'a mut LoadContext) -> BoxedFuture<'a, anyhow::Result<(), anyhow::Error>> {
        Box::pin(async move {
            let file_path = format!("./assets/{}", load_context.path().as_os_str().to_str().unwrap());
            let full_path = std::fs::canonicalize(&file_path)
                .expect(format!("failed to find full path of {:?}", file_path).as_str());

            let data = VfxAsset { id: None, bytes: bytes.to_vec(), path: full_path };
            load_context.set_default_asset(LoadedAsset::new(data));
            Ok(())
        })
    }

    fn extensions(&self) -> &[&str] {
        &["efk"]
    }
}

pub(super) fn load_vfx_2_device_system(mut vfx_asset_events: EventReader<AssetEvent<VfxAsset>>,
                                       mut assets: ResMut<Assets<VfxAsset>>) {
    for event in vfx_asset_events.iter() {
        let mut changed_set: HashSet<Handle<VfxAsset>> = HashSet::default();
        let mut destroy_set: HashSet<Handle<VfxAsset>> = HashSet::default();

        match event {
            AssetEvent::Created { ref handle } => {
                changed_set.insert(handle.clone_weak());
            }
            AssetEvent::Modified { ref handle } => {
                // changed_set.insert(handle.clone_weak());
                // destroy_set.insert(handle.clone_weak());
            }
            AssetEvent::Removed { ref handle } => {
                destroy_set.insert(handle.clone_weak());
                changed_set.remove(handle);
            }
        }

        for destroy_handle in &destroy_set {
            if let Some(v) = assets.get(destroy_handle) {
                if let Some(vid) = v.id {
                    unsafe {
                        ReleaseEffectPrefab(vid);
                        info!("remove vfx asset");
                    }
                }
            }
        }

        for handle in &changed_set {
            if let Some(v) = assets.get_mut(handle) {
                use crate::vfx::bindings::*;

                info!("start parse vfx");
                let mut p = v.path.parent().unwrap();
                let dir = p.to_str().unwrap();
                let mut dir_utf16: Vec<u16> = dir.encode_utf16().collect();
                let c = '\0' as u16;
                dir_utf16.push(c);

                let id = unsafe {
                    LoadEffectPrefab(v.bytes.as_ptr() as *const c_void, v.bytes.len() as c_int, dir_utf16.as_ptr() as _)
                };
                v.id = Some(id);
                info!("parse vfx complete");
            }
        }
    }
}


pub struct VfxReq
{
    pub path: &'static str,
    pub pos: Vec3,
    pub rot: Quat,
}
