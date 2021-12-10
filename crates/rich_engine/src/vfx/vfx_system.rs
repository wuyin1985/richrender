use std::ffi::c_void;
use std::ops::Deref;
use std::sync::{Arc, Mutex, MutexGuard};
use bevy::ecs::schedule::ShouldRun::No;

use crate::{RenderCamera, RenderRunner};
use crate::vfx::vfx_resource::{VfxAsset, VfxReq, VfxSystemState};
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


pub(super) fn init_vfx_system(mut state: ResMut<VfxSystemState>, render_runner: Option<Res<RenderRunner>>, time: Res<Time>) {
    if render_runner.is_none() {
        return;
    }
    let mut render_runner = render_runner.unwrap();
    let render_runner = render_runner.deref();

    if !state.is_inited() {
        startup_vfx_system(render_runner);
        state.set_inited();
    }
}

fn matrix_convert(value: &Mat4) -> super::bindings::Matrix {
    super::bindings::Matrix { Values: value.to_cols_array_2d() }
}

pub(super) fn draw_vfx_system(mut state: ResMut<VfxSystemState>,
                              time: Res<Time>,
                              render_runner: Option<Res<RenderRunner>>) {
    use ash::vk::Handle;

    if !state.is_inited() {
        return;
    }

    unsafe {
        let p = 0 as *mut c_void;
        let mut render_runner = render_runner.unwrap();
        let context = &render_runner.context;
        let data = context.per_frame_uniform.as_ref().unwrap();
        let proj = matrix_convert(&data.data.proj);
        let view = matrix_convert(&data.data.view);
        super::bindings::SyncProjectionMatrix(proj);
        super::bindings::SyncViewMatrix(view);

        let command_buffer = render_runner.get_current_command_buffer().unwrap();
        super::bindings::UpdateFrame(p, command_buffer.as_raw());
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

pub(super) fn update_vfx_system(
    mut query: Query<(Entity, &mut VfxHasPlay, &GlobalTransform)>,
    mut commands: Commands,
)
{
    for (entity, mut vfx, transform) in query.iter_mut() {
        let t = transform.translation;
        let r = transform.rotation;
        let (rx, ry, rz) = r.to_euler(EulerRot::ZXY);
        unsafe {
            SetEffectLocation(vfx.instance_id, t.x, t.y, t.z);
            SetEffectRotation(vfx.instance_id, rx, ry, rz);
        }

        if !vfx.is_loop {
            vfx.left_frame -= 1;
            if vfx.left_frame <= 0 {
                commands.entity(entity).insert(Destroy {});
            }
        }
    }
}

pub(super) struct VfxHasPlay {
    pub left_frame: i32,
    pub is_loop: bool,
    pub instance_id: i32,
}

pub(super) fn play_effect_system(
    mut query: Query<(Entity, &Handle<VfxAsset>), Without<VfxHasPlay>>,
    state: Res<VfxSystemState>,
    mut commands: Commands,
)
{
    for (entity, vfx_handle) in query.iter() {
        if let Some(vfx) = state.try_get_prefab(vfx_handle) {
            let id = unsafe {
                PlayEffect(vfx.id)
            };

            commands.entity(entity).insert(VfxHasPlay { instance_id: id, left_frame: vfx.duration, is_loop: vfx.is_loop });
        }
    }
}

pub(super) fn stop_effect_system(
    mut query: Query<(&VfxHasPlay), With<Destroy>>,
)
{
    for (vfx) in query.iter() {
        unsafe {
            StopEffect(vfx.instance_id);
        }
    }
}
