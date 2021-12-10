use std::collections::{HashMap, HashSet};
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
    pub bytes: Vec<u8>,
    pub path: PathBuf,
}

#[derive(Debug, Default)]
pub(super) struct VfxPrefab {
    pub id: i32,
    pub duration: i32,
    pub is_loop: bool,
}

#[derive(Debug, Default)]
pub(crate) struct VfxSystemState {
    inited: bool,
    map: HashMap<Handle<VfxAsset>, VfxPrefab>,
}

impl VfxSystemState {
    pub(super) fn create() -> Self {
        VfxSystemState {
            map: HashMap::<Handle<VfxAsset>, VfxPrefab>::new(),
            inited: false,
        }
    }

    pub(super) fn try_get_prefab(&self, handle: &Handle<VfxAsset>) -> Option<&VfxPrefab> {
        self.map.get(handle)
    }

    pub(super) fn remove_prefab(&mut self, handle: &Handle<VfxAsset>) -> Option<VfxPrefab> {
        self.map.remove(handle)
    }

    pub(super) fn insert_prefab(&mut self, handle: &Handle<VfxAsset>, prefab: VfxPrefab) {
        self.map.insert(handle.clone_weak(), prefab);
    }

    pub(super) fn set_inited(&mut self) {
        self.inited = true;
    }

    pub(super) fn is_inited(&self) -> bool {
        self.inited
    }
}

#[derive(Default)]
pub(super) struct VfxAssetLoader;

impl AssetLoader for VfxAssetLoader {
    fn load<'a>(&'a self, bytes: &'a [u8], load_context: &'a mut LoadContext) -> BoxedFuture<'a, anyhow::Result<(), anyhow::Error>> {
        Box::pin(async move {
            let file_path = format!("./assets/{}", load_context.path().as_os_str().to_str().unwrap());
            let full_path = std::fs::canonicalize(&file_path)
                .expect(format!("failed to find full path of {:?}", file_path).as_str());

            let data = VfxAsset { bytes: bytes.to_vec(), path: full_path };
            load_context.set_default_asset(LoadedAsset::new(data));
            Ok(())
        })
    }

    fn extensions(&self) -> &[&str] {
        &["efk"]
    }
}

pub(super) fn load_vfx_2_device_system(mut vfx_asset_events: EventReader<AssetEvent<VfxAsset>>,
                                       mut assets: Res<Assets<VfxAsset>>,
                                       mut state: ResMut<VfxSystemState>) {
    for event in vfx_asset_events.iter() {
        let mut changed_set: HashSet<Handle<VfxAsset>> = HashSet::default();
        let mut destroy_set: HashSet<Handle<VfxAsset>> = HashSet::default();

        match event {
            AssetEvent::Created { ref handle } => {
                changed_set.insert(handle.clone_weak());
            }
            AssetEvent::Modified { ref handle } => {
                changed_set.insert(handle.clone_weak());
                destroy_set.insert(handle.clone_weak());
            }
            AssetEvent::Removed { ref handle } => {
                destroy_set.insert(handle.clone_weak());
                changed_set.remove(handle);
            }
        }

        for destroy_handle in &destroy_set {
            if let Some(v) = state.remove_prefab(destroy_handle) {
                unsafe {
                    ReleaseEffectPrefab(v.id);
                    info!("remove vfx asset");
                }
            }
        }

        for handle in &changed_set {
            if let Some(v) = assets.get(handle) {
                use crate::vfx::bindings::*;

                info!("start parse vfx");
                let mut p = v.path.parent().unwrap();
                let dir = p.to_str().unwrap();
                let mut dir_utf16: Vec<u16> = dir.encode_utf16().collect();
                let c = '\0' as u16;
                dir_utf16.push(c);

                let mut info = EffectInfo { duration: 0, prefabId: 0 };
                unsafe {
                    let ptr = &mut info as *mut EffectInfo;
                    LoadEffectPrefab(v.bytes.as_ptr() as *const c_void, v.bytes.len() as c_int, dir_utf16.as_ptr() as _, ptr);
                };

                let prefab = VfxPrefab { id: info.prefabId, duration: info.duration, is_loop: info.duration == i32::MAX };
                info!("parse vfx complete {}:{}f loop:{}", prefab.id, prefab.duration, prefab.is_loop);
                state.insert_prefab(handle, prefab);
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
