use std::ffi::c_void;
use std::ops::Deref;
use std::sync::{Arc, Mutex, MutexGuard};
use bevy::ecs::schedule::ShouldRun::No;

use crate::RenderRunner;
use crate::vfx::vfx_resource::{VfxAsset, VfxReq};
use crate::vfx::bindings::*;
use crate::prelude::*;

static mut VK_QUEUE_MUTEX: Option<Arc<Mutex<i32>>> = None;
static mut VK_QUEUE_MUTEX_GUARD: Option<MutexGuard<i32>> = None;

unsafe extern "C" fn c_lock()
{
    unsafe {
        let m = VK_QUEUE_MUTEX.as_ref().unwrap();
        VK_QUEUE_MUTEX_GUARD = Some(m.lock().unwrap());
    }
}

unsafe extern "C" fn c_unlock()
{
    unsafe {
        VK_QUEUE_MUTEX_GUARD = None;
    }
}

pub(super) fn startup_vfx_system(runner: &RenderRunner) {
    use ash::vk::Handle;
    use std::ffi::c_void;

    let context = &runner.context;
    let command_list = &runner.command_buffer_list;
    let pass = &runner.forward_render_pass;

    let color = {
        let color_texture = pass.get_color_texture();
        let color_size = color_texture.get_size();
        crate::vfx::bindings::ShareTexture {
            image: color_texture.get_image().as_raw(),
            view: pass.get_color_view().as_raw(),
            format: color_texture.get_format().as_raw(),
            width: color_size.0 as _,
            height: color_size.1 as _,
        }
    };

    let depth = {
        let depth_texture = pass.get_depth_texture();
        let depth_size = depth_texture.get_size();
        crate::vfx::bindings::ShareTexture {
            image: depth_texture.get_image().as_raw(),
            view: pass.get_depth_view().as_raw(),
            format: depth_texture.get_format().as_raw(),
            width: depth_size.0 as _,
            height: depth_size.1 as _,
        }
    };

    let device = context.device.handle().as_raw();
    let phy_device = context.physical_device.as_raw();
    let queue = context.graphics_queue.as_raw();
    let command_pool = command_list.get_command_pool().as_raw();

    unsafe {
        VK_QUEUE_MUTEX = Some(Arc::clone(&runner.mutex));
    }
    unsafe {
        crate::vfx::bindings::StartupWithExternalVulkan(device, phy_device, queue, command_pool, color, depth);
        crate::vfx::bindings::SetThreadLockCall(Some(c_lock), Some(c_unlock));
    };
}

#[derive(Debug, Default)]
pub(crate) struct VfxSystemState {
    pub inited: bool,
}

pub(super) fn init_vfx_system(mut state: ResMut<VfxSystemState>, render_runner: Option<Res<RenderRunner>>, time: Res<Time>) {
    if render_runner.is_none() {
        return;
    }
    let mut render_runner = render_runner.unwrap();
    let render_runner = render_runner.deref();

    if !state.inited {
        startup_vfx_system(render_runner);
        state.inited = true;
    }
}

pub(super) fn update_vfx_system(mut state: ResMut<VfxSystemState>, time: Res<Time>, render_runner: Option<Res<RenderRunner>>) {
    use ash::vk::Handle;

    if !state.inited {
        return;
    }

    unsafe {
        let p = 0 as *mut c_void;
        let mut render_runner = render_runner.unwrap();
        let command_buffer = render_runner.get_current_command_buffer().unwrap();

        crate::vfx::bindings::UpdateFrame(p, command_buffer.as_raw());
    }
}

pub(super) fn create_vfx_by_req_system(
    mut query: Query<(Entity, &VfxReq), Without<Handle<VfxAsset>>>,
    mut asset_server: ResMut<AssetServer>,
    mut commands: Commands,
)
{
    for (entity, req) in query.iter()
    {
        let handle: Handle<VfxAsset> = asset_server.load(req.path);
        commands.entity(entity).insert(GlobalTransform::identity())
            .insert(Transform {
                translation: req.pos,
                rotation: req.rot,
                scale: Vec3::ONE,
            }).insert(handle);
    }
}

pub(super) fn update_vfx_transform(
    mut query: Query<(&Handle<VfxAsset>, &GlobalTransform), Changed<GlobalTransform>>,
    assets: Res<Assets<VfxAsset>>,
)
{
    for (vfx_handle, transform) in query.iter() {
        if let Some(vfx) = assets.get(vfx_handle) {
            let t = transform.translation;
            let r = transform.rotation;
            let (rx, ry, rz) = r.to_euler(EulerRot::ZXY);
            unsafe {
                SetEffectLocation(vfx.id, t.x, t.y, t.z);
                SetEffectRotation(vfx.id, rx, ry, rz);
            }
        }
    }
}

pub(super) struct VfxHasPlay(i32);

pub(super) fn play_effect_system(
    mut query: Query<(Entity, &Handle<VfxAsset>), Without<VfxHasPlay>>, assets: Res<Assets<VfxAsset>>, mut commands: Commands,
)
{
    for (entity, vfx_handle) in query.iter() {
        if let Some(vfx) = assets.get(vfx_handle) {
            let id = unsafe {
                PlayEffect(vfx.id)
            };
            commands.entity(entity).insert(VfxHasPlay(id));
        }
    }
}

pub(super) fn stop_effect_system(
    mut query: Query<(&VfxHasPlay), With<Destroy>>,
)
{
    for (vfx) in query.iter() {
        unsafe {
            StopEffect(vfx.0);
        }
    }
}
