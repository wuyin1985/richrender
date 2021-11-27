use std::collections::HashMap;
use std::ffi::c_void;
use std::ops::{Deref, DerefMut};
use ash::vk::Handle;
use bevy::ecs::system::LocalState;

use bevy::prelude::{AppBuilder, Local, Plugin, Res, ResMut, Time};
use crate::{ForwardRenderPass, RenderContext, RenderRunner};
use crate::render::CommandBufferList;
use crate::prelude::*;

mod bindings;

struct Vfx
{
    id: i32,
}

fn startup_vfx_system(runner: & RenderRunner) {
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
        crate::vfx::bindings::StartupWithExternalVulkan(device, phy_device, queue, command_pool, color, depth)
    };
}

#[derive(Debug, Default)]
struct VfxSystemState {
    inited: bool,
}

fn init_vfx(mut state: ResMut<VfxSystemState>, render_runner: Option<Res<RenderRunner>>, time: Res<Time>)
{
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

fn update_vfx(mut state: ResMut<VfxSystemState>, time: Res<Time>, render_runner: Option<Res<RenderRunner>>)
{
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

pub struct VfxPlugin {}

impl Plugin for VfxPlugin {
    fn build(&self, app: &mut AppBuilder) {
        app.insert_resource(VfxSystemState { inited: false });

        app.add_system_to_stage(
            RenderStage::Upload,
            init_vfx.system(),
        );

        app.add_system_to_stage(
            RenderStage::PostDraw,
            update_vfx.system(),
        );
    }
}