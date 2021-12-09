use bevy::prelude::*;
use bevy::app::{ManualEventReader, Events};
use bevy::window::{WindowCreated, WindowResized, WindowId};
use bevy::winit::WinitWindows;
use crate::render::swapchain_mgr::SwapChainMgr;
use crate::render::render_runner::RenderRunner;
use crate::render::graphic_pipeline::{PipelineVertexInputInfo, GraphicPipeline};
use ash::vk;
use crate::render::{CameraOpEvent, RenderStage, vertex};
use crate::render::camera::Camera;
use crate::render::fly_camera::{FlyCamera, FlyCameraPlugin};
use crate::render::render_context::PerFrameData;
use crate::render::gltf_asset_loader::{GltfAsset, GltfAssetLoader};
use std::collections::HashSet;
use crate::render::model_renderer::{ModelRenderer, ModelData, ShadeNames};
use std::ops::DerefMut;
use std::sync::Mutex;
use bevy::math::Vec4Swizzles;
use bevy::tasks::ComputeTaskPool;
use bevy::transform::TransformSystem;
use crate::core::destroy::{Destroy, DestroyStage};
use crate::DisplayName;
use super::animation_system;
use crate::render::model_runtime;
use crate::render::model_runtime::{ModelRuntime, ModelSkins};

pub struct RenderInitEvent {}

pub struct MainLight {}


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

            let mut fire = world.get_resource_mut::<Events<RenderInitEvent>>().unwrap();
            fire.send(RenderInitEvent {});
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


fn draw_models_system(mut runner: Option<ResMut<RenderRunner>>,
                      mut transform_query: Query<&GlobalTransform>,
                      mut model_query: Query<(&ModelRuntime, Option<&ModelSkins>, &Handle<GltfAsset>, &GlobalTransform),
                          Without<Destroy>>) {
    if let Some(runner) = &mut runner {
        let runner = runner.deref_mut();
        let mut model_data = ModelData::default();
        if let Some((present_index, command_buffer)) = runner.begin_draw() {
            let context = &mut runner.context;

            #[cfg(feature = "statistic")]
                context.statistic.begin_query(&context.device, command_buffer);

            let forward_render_pass = &runner.forward_render_pass;

            if runner.grass.enable_draw {
                runner.grass.compute_grass_data(context);

                runner.grass.cmd_barrier(context, command_buffer);
            }

            //shadow
            forward_render_pass.begin_shadow_pass(context, command_buffer);

            let mut list = Vec::new();
            for (runtime, skins, handle, transform) in model_query.iter_mut() {

                let model_renderer = context.get_model(handle);
                if let Some(mr) = model_renderer {
                    mr.draw_shadow(context, command_buffer, &runtime, skins, &transform_query);
                    list.push((handle, skins, transform, runtime));
                }
            }
            forward_render_pass.end_shadow_pass(context, command_buffer);

            //draw
            forward_render_pass.begin_render_pass(context, command_buffer);

            for (handle, skins, _, runtime) in list {
                let mr = context.get_model(handle).unwrap();
                mr.draw(context, command_buffer, &runtime, skins, &transform_query);
            }

            runner.grass.draw(context, command_buffer);

            forward_render_pass.end_render_pass(context, command_buffer);

            #[cfg(feature = "statistic")]
                context.statistic.end_query(&context.device, command_buffer);
        } else {
            runner.current_present_index = -1;
        }
    }
}

fn end_draw_system(mut runner: Option<ResMut<RenderRunner>>) {
    if let Some(runner) = &mut runner {
        if let Some(cb) = runner.get_current_command_buffer() {
            runner.end_draw(cb);
        }
    }
}

pub struct RenderCamera {
    pub camera: Entity,
}

fn update_render_state_from_camera(mut commands: Commands,
                                   render_camera: Res<RenderCamera>,
                                   mut runner: Option<ResMut<RenderRunner>>,
                                   time: Res<Time>,
                                   camera_query: Query<(&Camera, &Transform)>,
                                   main_light_query: Query<(&MainLight, &Transform)>,
)
{
    if let Some(runner) = &mut runner {
        if let Ok((light, light_transform)) = main_light_query.single() {
            if let Ok((camera, transform)) = camera_query.get(render_camera.camera) {
                let light_view = light_transform.compute_matrix().inverse();
                let light_dir = light_view.transform_vector3(Vec3::Z);

                let light_project = Mat4::orthographic_rh(-10.0, 10.0, -10.0, 10.0, 1.0, 20.0);
                let light_matrix = light_project * light_view;
                let pos = transform.translation;

                let proj = Mat4::perspective_rh(
                    camera.fov,
                    camera.aspect,
                    camera.z_near,
                    camera.z_far);

                let view = transform.compute_matrix().inverse();

                let frame_data = PerFrameData {
                    view: view,
                    proj: proj,
                    light_matrix,
                    light_dir: Vec4::from((light_dir, 0.0)),
                    camera_pos: Vec4::from((pos, 1.0)),
                    camera_dir: Vec4::from((transform.rotation.mul_vec3(Vec3::Z), 1.0)),
                    delta_time: 0.016,
                    total_time: time.seconds_since_startup() as _,
                };

                runner.upload_per_frame_data(frame_data);
            }
        }
    }
}

fn begin_upload(mut runner: Option<ResMut<RenderRunner>>) {
    if let Some(runner) = &mut runner {
        let runner: &mut RenderRunner = runner.deref_mut();
        let command_buffer = runner.get_upload_command_buffer();
        let context = &mut runner.context;
        unsafe {
            context.device.begin_command_buffer(command_buffer,
                                                &vk::CommandBufferBeginInfo::builder().
                                                    flags(vk::CommandBufferUsageFlags::ONE_TIME_SUBMIT).build());
        }
    }
}

fn end_upload(mut runner: Option<ResMut<RenderRunner>>) {
    if let Some(runner) = &mut runner {
        let runner: &mut RenderRunner = runner.deref_mut();
        let command_buffer = runner.get_upload_command_buffer();
        let context = &mut runner.context;
        unsafe {
            context.device.end_command_buffer(command_buffer);
            let mut guard = runner.mutex.lock().unwrap();
            context.device.queue_submit(context.transfer_queue, &[vk::SubmitInfo::builder().command_buffers(&[command_buffer]).build()],
                                        vk::Fence::null());
            drop(guard);
            context.device.device_wait_idle();
        }

        context.flush_staging_buffer();
    }
}

fn it_works() {
    let num1: usize = 1;
    let mut num2: usize = num1;
    num2 += num1;
}

fn load_gltf_2_device_system(mut runner: Option<ResMut<RenderRunner>>,
                             mut assets: ResMut<Assets<GltfAsset>>,
                             mut gltf_events: EventReader<AssetEvent<GltfAsset>>) {
    if let Some(runner) = &mut runner {
        let runner: &mut RenderRunner = runner.deref_mut();
        let command_buffer = runner.get_upload_command_buffer();
        let context = &mut runner.context;
        let swap_mgr = &runner.swapchain_mgr;

        let mut changed_gltf_set: HashSet<Handle<GltfAsset>> = HashSet::default();
        let mut destroy_gltf_set: HashSet<Handle<GltfAsset>> = HashSet::default();

        for event in gltf_events.iter() {
            match event {
                AssetEvent::Created { ref handle } => {
                    changed_gltf_set.insert(handle.clone_weak());
                }
                AssetEvent::Modified { ref handle } => {
                    changed_gltf_set.insert(handle.clone_weak());
                    destroy_gltf_set.insert(handle.clone_weak());
                }
                AssetEvent::Removed { ref handle } => {
                    destroy_gltf_set.insert(handle.clone_weak());
                    changed_gltf_set.remove(handle);
                }
            }
        }

        for destroy_handle in &destroy_gltf_set {
            context.remove_model(destroy_handle);
            info!("remove gltf asset");
        }

        if changed_gltf_set.len() == 0 {
            return;
        }


        for changed_gltf_handle in changed_gltf_set.iter() {
            let gltf_asset = assets.get(changed_gltf_handle).expect("failed to find asset gltf");


            let model = ModelRenderer::create(context,
                                              swap_mgr,
                                              &runner.forward_render_pass,
                                              command_buffer,
                                              gltf_asset,
                                              &ShadeNames {
                                                  vertex: "pbr_vert",
                                                  frag: "pbr_frag",
                                                  shadow_vertex: "pbr_shadow_vert",
                                                  shadow_frag: "pbr_shadow_frag",
                                              });

            context.insert_model(changed_gltf_handle.clone_weak(), model);
        }
    }
}

pub struct RenderPlugin {}

#[derive(Debug, Hash, PartialEq, Eq, Clone, SystemLabel)]
enum UploadLabel {
    Model,
    Skin,
}

impl Plugin for RenderPlugin {
    fn build(&self, app: &mut AppBuilder) {
        //init camera
        let world = app.world_mut();

        let trans = Mat4::from_scale_rotation_translation(Vec3::ONE,
                                                          //Quat::from_axis_angle(Vec3::X, 180f32.to_radians()),
                                                          Quat::IDENTITY,
                                                          Vec3::new(0.0, 0.0, 1.0));

        let ce = world.spawn().insert(Camera::default())
            .insert(FlyCamera::default())
            .insert(DisplayName::from_str("camera"))
            .insert(Transform::from_matrix(trans)).id();
        world.insert_resource(RenderCamera { camera: ce });

        let light_pos = Vec3::new(-12.0, 3.5, -2.0);
        let light_look_at = Vec3::ZERO;
        let light_mat = Mat4::look_at_rh(light_pos, light_look_at, Vec3::Y).inverse();

        let light = world.spawn().insert(MainLight {})
            .insert(Transform::from_matrix(light_mat))
            .insert(DisplayName::from_str("main_light"));


        let render_system = get_render_system(app.world_mut());
        app.init_asset_loader::<GltfAssetLoader>();
        app.add_asset::<GltfAsset>();

        app.add_event::<RenderInitEvent>();
        app.add_event::<CameraOpEvent>();

        //upload
        app.add_stage_after(CoreStage::PreUpdate, RenderStage::BeginUpload, SystemStage::parallel());
        app.add_stage_after(RenderStage::BeginUpload, RenderStage::Upload, SystemStage::parallel());
        app.add_stage_after(RenderStage::Upload, RenderStage::EndUpload, SystemStage::parallel());
        app.add_stage_after(RenderStage::EndUpload, RenderStage::ThirdPartyUpload, SystemStage::single_threaded());

        app.add_system_to_stage(RenderStage::BeginUpload, begin_upload.system());
        app.add_system_to_stage(RenderStage::Upload, load_gltf_2_device_system.system().label(UploadLabel::Model));
        app.add_system_to_stage(RenderStage::Upload, model_runtime::init_model_runtime_system.system().after(UploadLabel::Model));
        app.add_system_to_stage(RenderStage::EndUpload, end_upload.system());

        //draw
        app.add_stage_after(CoreStage::PostUpdate, RenderStage::PrepareDraw, SystemStage::parallel());
        app.add_stage_after(RenderStage::PrepareDraw, RenderStage::BeginDraw, SystemStage::parallel());
        app.add_stage_after(RenderStage::BeginDraw, RenderStage::Draw, SystemStage::parallel());
        app.add_stage_after(RenderStage::Draw, RenderStage::PostDraw, SystemStage::single_threaded());
        app.add_stage_after(RenderStage::PostDraw, RenderStage::EndDraw, SystemStage::parallel());

        app.add_system_to_stage(RenderStage::PrepareDraw, Camera::update_camera_op_event_system.system());
        app.add_system_to_stage(RenderStage::PrepareDraw, render_system.exclusive_system());
        app.add_system_to_stage(RenderStage::PrepareDraw, update_render_state_from_camera.system());

        app.add_system_to_stage(RenderStage::BeginDraw, draw_models_system.system());
        app.add_system_to_stage(RenderStage::EndDraw, end_draw_system.system());

        app.add_system(model_runtime::update_model_runtime_animation.system());

        app.add_system_to_stage(CoreStage::PostUpdate, model_runtime::update_skin_joint_matrix.system().after(TransformSystem::TransformPropagate));

        app.add_system_to_stage(DestroyStage::Prepare, model_runtime::destroy_model_skins_system.system());

        app.add_plugin(FlyCameraPlugin);
    }
}
