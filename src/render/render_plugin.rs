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
use crate::render::render_context::PerFrameData;

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
        let frame_data = PerFrameData {
            view: transform.compute_matrix().inverse(),
            proj: Mat4::perspective_rh(
                camera.fov,
                camera.aspect,
                camera.z_near,
                camera.z_far),
            light_dir: Vec3::new(1.0, -1.0, -1.0),
            camera_pos: transform.translation,
        };
        runner.upload_per_frame_data(frame_data);
    }
}

pub struct RenderPlugin {}

impl Plugin for RenderPlugin {
    fn build(&self, app: &mut AppBuilder) {
        //init camera
        let world = app.world_mut();

        let transM = Mat4::from_scale_rotation_translation(Vec3::ONE,
                                                           Quat::from_axis_angle(Vec3::Y, 0f32.to_radians()),
                                                           Vec3::new(0.0, 0.0, 1.0));

        let ce = world.spawn().insert(Camera::default())
            .insert(FlyCamera::default())
            .insert(Transform::from_matrix(transM)).id();
        world.insert_resource(RenderCamera { camera: ce });


        let render_system = get_render_system(app.world_mut());
        app.add_system(render_system.exclusive_system());
        app.add_system(update_render_state_from_camera.system());
        app.add_plugin(FlyCameraPlugin);
    }
}