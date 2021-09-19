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
use crate::render::gltf_asset_loader::{GltfAsset, GltfAssetLoader};
use std::collections::HashSet;
use crate::render::model_renderer::{ModelRenderer, ModelData};
use std::ops::DerefMut;

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


fn draw_models_system(mut runner: ResMut<RenderRunner>, model_query: Query<(&Handle<GltfAsset>, &Transform)>) {
    let runner = runner.deref_mut();
    let mut model_data = ModelData::default();
    if let Some((present_index, command_buffer)) = runner.begin_draw() {
        let context = &mut runner.device_mgr;
        for (handle, transform) in model_query.iter() {
            let model_renderer = context.get_model(handle);
            if let Some(mr) = model_renderer {
                //model_data.transform = ModelRenderer::get_center_transform(mr.get_model().aabb);
                model_data.transform = transform.compute_matrix();
                mr.draw(context, command_buffer, &model_data);
            }
        }
        runner.end_draw(present_index, command_buffer);
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
        let pos = transform.translation;
        let frame_data = PerFrameData {
            view: transform.compute_matrix().inverse(),
            proj: Mat4::perspective_rh(
                camera.fov,
                camera.aspect,
                camera.z_near,
                camera.z_far),
            light_dir: Vec3::new(1.0, -1.0, -1.0),
            camera_pos: pos,
            dummy1: 0f32,
            dummy2: 0f32,
        };
        runner.upload_per_frame_data(frame_data);
    }
}


fn load_gltf_2_device_system(mut runner: ResMut<RenderRunner>, mut assets: ResMut<Assets<GltfAsset>>,
                             mut gltf_events: EventReader<AssetEvent<GltfAsset>>) {
    let runner: &mut RenderRunner = runner.deref_mut();
    let context = &mut runner.device_mgr;
    let swap_mgr = &runner.swapchain_mgr;

    let mut changed_gltf_set: HashSet<Handle<GltfAsset>> = HashSet::default();

    for event in gltf_events.iter() {
        match event {
            AssetEvent::Created { ref handle } => {
                changed_gltf_set.insert(handle.clone_weak());
            }
            AssetEvent::Modified { ref handle } => {
                changed_gltf_set.insert(handle.clone_weak());
                //remove_current_mesh_resources(render_resource_context, handle);
            }
            AssetEvent::Removed { ref handle } => {
                //remove_current_mesh_resources(render_resource_context, handle);
                changed_gltf_set.remove(handle);
            }
        }
    }

    if changed_gltf_set.len() == 0 {
        return;
    }

    let command_buffer = runner.command_buffer_list.get_upload_command_buffer();
    unsafe {
        context.device.begin_command_buffer(command_buffer,
                                            &vk::CommandBufferBeginInfo::builder().
                                                flags(vk::CommandBufferUsageFlags::ONE_TIME_SUBMIT).build());
    }

    for changed_gltf_handle in changed_gltf_set.iter() {
        let gltf_asset = assets.get(changed_gltf_handle).expect("failed to find asset gltf");


        let model = ModelRenderer::create(context,
                                          swap_mgr,
                                          runner.forward_render_pass.get_native_render_pass(),
                                          command_buffer,
                                          gltf_asset,
                                          "simple_draw_object_vert",
                                          "simple_draw_object_frag",
        );

        context.insert_model(changed_gltf_handle.clone_weak(), model);
    }

    unsafe {
        context.device.end_command_buffer(command_buffer);
        context.device.queue_submit(context.graphics_queue, &[vk::SubmitInfo::builder().command_buffers(&[command_buffer]).build()], vk::Fence::null());
        context.device.device_wait_idle();
    }

    context.flush_staging_buffer();
}

pub struct RenderPlugin {}

impl Plugin for RenderPlugin {
    fn build(&self, app: &mut AppBuilder) {
        //init camera
        let world = app.world_mut();

        let trans = Mat4::from_scale_rotation_translation(Vec3::ONE,
                                                          Quat::IDENTITY,
                                                          Vec3::new(0.0, 0.0, 1.0));

        let ce = world.spawn().insert(Camera::default())
            .insert(FlyCamera::default())
            .insert(Transform::from_matrix(trans)).id();
        world.insert_resource(RenderCamera { camera: ce });


        let render_system = get_render_system(app.world_mut());
        app.init_asset_loader::<GltfAssetLoader>();
        app.add_asset::<GltfAsset>();
        app.add_system(render_system.exclusive_system());
        app.add_system(update_render_state_from_camera.system());
        app.add_system(load_gltf_2_device_system.system());
        app.add_system(draw_models_system.system());
        app.add_plugin(FlyCameraPlugin);
    }
}