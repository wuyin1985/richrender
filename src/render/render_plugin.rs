use bevy::prelude::*;
use bevy::app::{ManualEventReader, Events};
use bevy::window::{WindowCreated, WindowResized, WindowId};
use bevy::winit::WinitWindows;
use crate::render::swapchain_mgr::SwapChainMgr;
use crate::render::render_runner::RenderRunner;
use crate::render::graphic_pipeline::{PipelineVertexInputInfo, GraphicPipeline};
use ash::vk;
use crate::render::vertex;
use crate::render::camera::Camera;
use crate::render::fly_camera::{FlyCamera, FlyCameraPlugin};

struct RenderMgr {
    window_created_event_reader: ManualEventReader<WindowCreated>,
    window_resized_event_reader: ManualEventReader<WindowResized>,
}


impl RenderMgr {
    fn handle_window_created_event(&mut self, world: &mut World) {
        let windows = world.get_resource::<Windows>().unwrap();
        let window_created_events = world.get_resource::<Events<WindowCreated>>().unwrap();

        let ww = {
            self.window_created_event_reader.iter(&window_created_events).find_map(|window_created_event| {
                let window = windows
                    .get(window_created_event.id)
                    .expect("Received window created event for non-existent window.");
                if window.id() == WindowId::primary() {
                    let winit_windows = world.get_resource::<WinitWindows>().unwrap();
                    let winit_window = winit_windows.get_window(window.id()).unwrap();
                    Some((window, winit_window))
                } else {
                    None
                }
            })
        };

        if let Some((window, winit_window)) = ww {
            let render_runner = RenderRunner::create(winit_window, window.physical_width(), window.physical_height());
            world.insert_resource(render_runner);
        };
    }

    pub fn update(&mut self, world: &mut World) {
        self.handle_window_created_event(world);
        let mut rr = world.get_resource_mut::<RenderRunner>();
        if let Some(render_runner) = rr.as_mut() {
            render_runner.draw();
        }
    }
}


fn get_render_system(world: &mut World) -> impl FnMut(&mut World) {
    let mut r = RenderMgr {
        window_created_event_reader: Default::default(),
        window_resized_event_reader: Default::default(),
    };

    move |pworld| {
        r.update(pworld)
    }
}

struct RenderCamera {
    camera: Entity,
}

fn update_render_state_from_camera(mut commands: Commands,
                                   render_camera: Res<RenderCamera>,
                                   mut runner: ResMut<RenderRunner>,
                                   camera_query: Query<(&Camera, &Transform)>,
)
{
    if let Ok((camera, transform)) = camera_query.get(render_camera.camera) {
        let data = runner.get_per_frame_data_mut();
        data.view = transform.compute_matrix();
        data.proj = Mat4::perspective_rh(
            camera.fov,
            camera.aspect,
            camera.z_near,
            camera.z_far,
        );
        //println!("camera {:?} {:?}", camera, transform);
    }
}

pub struct RenderPlugin {}

impl Plugin for RenderPlugin {
    fn build(&self, app: &mut AppBuilder) {
        //init camera
        let world = app.world_mut();
        let default_pos = Vec3::new(2f32, 2f32, 2f32);
        let default_scale = Vec3::new(1f32, 1f32, 1f32);
        let default_rot = Quat::from_rotation_mat4(&Mat4::look_at_rh(default_pos, Vec3::ZERO, Vec3::new(0f32, 0f32, 1f32)));
        let default_camera_transform = Mat4::from_scale_rotation_translation(
            default_scale, default_rot, default_pos);
        let ce = world.spawn().insert(Camera::default())
            .insert(FlyCamera::default())
            .insert(Transform::from_matrix(default_camera_transform)).id();
        world.insert_resource(RenderCamera { camera: ce });


        let render_system = get_render_system(app.world_mut());
        app.add_system(render_system.exclusive_system());
        app.add_system(update_render_state_from_camera.system());
        app.add_plugin(FlyCameraPlugin);
    }
}