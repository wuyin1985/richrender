use bevy::prelude::*;
use bevy::app::{ManualEventReader, Events};
use bevy::window::{WindowCreated, WindowResized, WindowId};
use bevy::winit::WinitWindows;
use crate::render::device_mgr::DeviceMgr;
use crate::render::swapchain_mgr::SwapChainMgr;
use crate::render::render_context::RenderContext;


struct RenderMgr {
    window_created_event_reader: ManualEventReader<WindowCreated>,
    window_resized_event_reader: ManualEventReader<WindowResized>,
    context: Option<RenderContext>,
}


impl RenderMgr {
    fn handle_window_created_event(&mut self, world: &mut World) {
        let windows = world.get_resource::<Windows>().unwrap();
        let window_created_events = world.get_resource::<Events<WindowCreated>>().unwrap();
        for window_created_event in self.window_created_event_reader.iter(&window_created_events) {
            let window = windows
                .get(window_created_event.id)
                .expect("Received window created event for non-existent window.");
            if (window.id() == WindowId::primary()) {
                assert!(self.context.is_none());
                let winit_windows = world.get_resource::<WinitWindows>().unwrap();
                let winit_window = winit_windows.get_window(window.id()).unwrap();
                self.context = Some(RenderContext::create(winit_window, window.physical_width(), window.physical_height()));
            }
        }
    }

    pub fn update(&mut self, world: &mut World) {
        self.handle_window_created_event(world);
    }
}


fn get_render_system(world: &mut World) -> impl FnMut(&mut World) {
    let mut r = RenderMgr {
        window_created_event_reader: Default::default(),
        window_resized_event_reader: Default::default(),
        context: None,
    };

    move |world| {
        r.update(world)
    }
}

pub struct RenderPlugin {}

impl Plugin for RenderPlugin {
    fn build(&self, app: &mut AppBuilder) {
        let render_system = get_render_system(app.world_mut());
        app.add_system(render_system.exclusive_system());
    }
}