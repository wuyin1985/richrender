use std::ffi::{CString, CStr};
use ash::vk;
use std::borrow::{Cow, Borrow};
use raw_window_handle::HasRawWindowHandle;
use crate::render::swapchain_mgr::SwapchainSupportDetails;
use crate::render::buffer::Buffer;
use std::mem;
use bevy::prelude::*;
use crate::render::uniform::UniformObject;
use bevy::asset::AssetIoError::PathWatchError;
use std::collections::HashMap;
use std::any::{TypeId, Any};
use std::cell::RefCell;
use std::sync::{Arc, RwLock, RwLockReadGuard};
use crate::render::gltf_asset_loader::GltfAsset;
use crate::render::model_renderer::ModelRenderer;
use std::mem::size_of;
use crate::render::shader_collection::ShaderCollection;
use crate::render::texture::Texture;
use crate::render::model::ModelTexture;
use crate::render::render_statistic::RenderStatistic;

pub struct RenderConfig {
    pub msaa: vk::SampleCountFlags,
    pub apply_post_effect: bool,
    pub apply_shadow: bool,
    pub color_format: vk::Format,
    pub depth_format: vk::Format,
    pub shadow_map_dim: f32,
}

#[repr(C)]
#[derive(Clone, Debug, Copy)]
pub struct PerFrameData {
    pub view: Mat4,
    pub proj: Mat4,
    pub light_matrix: Mat4,
    pub light_dir: Vec4,
    pub camera_pos: Vec4,
    pub camera_dir: Vec4,
    pub delta_time: f32,
    pub total_time: f32,
}

impl PerFrameData {
    pub fn create() -> PerFrameData {
        PerFrameData {
            view: Mat4::IDENTITY,
            proj: Mat4::IDENTITY,
            light_matrix: Mat4::IDENTITY,
            light_dir: Vec4::Z,
            camera_pos: Vec4::ZERO,
            camera_dir: Vec4::Z,
            delta_time: 0f32,
            total_time: 0f32,
        }
    }
}

pub trait RenderResource: 'static + Send + Sync {
    fn as_any(&self) -> &dyn Any;
    fn as_any_mut(&mut self) -> &mut dyn Any;
    fn destroy_res(&mut self, rc: &mut RenderContext);
}

pub struct DummyResources {
    pub white_texture: ModelTexture,
}

impl RenderResource for DummyResources {
    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }

    fn destroy_res(&mut self, rc: &mut RenderContext) {
        self.destroy(rc);
    }
}

impl DummyResources {
    pub fn destroy(&mut self, context: &mut RenderContext) {
        self.white_texture.destroy(context);
    }

    pub fn create(context: &mut RenderContext, command_buffer: vk::CommandBuffer) -> Self {
        let t = Texture::create_from_rgba(context, command_buffer, 1, 1, &[std::u8::MAX; 4]);
        DummyResources {
            white_texture: ModelTexture::from(context, t)
        }
    }
}

pub struct RenderContext {
    pub window_width: u32,
    pub window_height: u32,
    pub entry: ash::Entry,
    pub surface_loader: ash::extensions::khr::Surface,
    pub surface: vk::SurfaceKHR,
    pub debug_call_back: vk::DebugUtilsMessengerEXT,
    pub debug_utils_loader: ash::extensions::ext::DebugUtils,
    pub physical_device: vk::PhysicalDevice,
    pub device: ash::Device,
    pub instance: ash::Instance,
    pub graphics_queue: vk::Queue,
    pub present_queue: vk::Queue,
    pub compute_queue: vk::Queue,
    pub device_memory_properties: vk::PhysicalDeviceMemoryProperties,
    pub swapchain_loader: ash::extensions::khr::Swapchain,
    pub graphics_queue_family_index: u32,
    pub compute_queue_family_index: u32,
    pub render_config: RenderConfig,
    pub descriptor_pool: vk::DescriptorPool,
    staging_buffers: Vec<Buffer>,
    resources: HashMap<TypeId, Box<dyn RenderResource>>,
    models: HashMap<Handle<GltfAsset>, ModelRenderer>,
    pub per_frame_uniform: Option<UniformObject<PerFrameData>>,
    pub min_uniform_buffer_offset_alignment: u32,
    pub shader_modules: ShaderCollection,
    #[cfg(feature = "statistic")]
    pub statistic: RenderStatistic,
}


unsafe extern "system" fn vulkan_debug_callback(
    message_severity: vk::DebugUtilsMessageSeverityFlagsEXT,
    message_type: vk::DebugUtilsMessageTypeFlagsEXT,
    p_callback_data: *const vk::DebugUtilsMessengerCallbackDataEXT,
    _user_data: *mut std::os::raw::c_void,
) -> vk::Bool32 {
    let callback_data = *p_callback_data;
    let message_id_number: i32 = callback_data.message_id_number as i32;

    let message_id_name = if callback_data.p_message_id_name.is_null() {
        Cow::from("")
    } else {
        CStr::from_ptr(callback_data.p_message_id_name).to_string_lossy()
    };

    let message = if callback_data.p_message.is_null() {
        Cow::from("")
    } else {
        CStr::from_ptr(callback_data.p_message).to_string_lossy()
    };

    let output_string = format!(
        "{:?} [{} ({})] : {}\n",
        message_type,
        message_id_name,
        &message_id_number.to_string(),
        message,
    );

    let output = output_string.as_str();

    match message_severity {
        vk::DebugUtilsMessageSeverityFlagsEXT::INFO | vk::DebugUtilsMessageSeverityFlagsEXT::VERBOSE
        => {
            info!(output)
        }

        vk::DebugUtilsMessageSeverityFlagsEXT::WARNING => {
            warn!(output)
        }

        vk::DebugUtilsMessageSeverityFlagsEXT::ERROR => {
            error!(output);
            panic!("graphic error")
        }

        _ => {
            panic!("unsupported message type")
        }
    }

    vk::FALSE
}


impl RenderContext {
    pub fn destroy(&mut self) {
        unsafe {
            let mut sm = std::mem::take(&mut self.shader_modules);
            sm.destroy(self);

            #[cfg(feature = "statistic")]
                self.statistic.destroy(&self.device);

            let mut res = std::mem::take(&mut self.resources);
            for (_, r) in res.iter_mut() {
                (*r).destroy_res(self);
            }

            let mut models = std::mem::take(&mut self.models);
            for (_, res) in models.iter_mut() {
                (*res).destroy(self);
            }
            let mut pf = std::mem::take(&mut self.per_frame_uniform);
            let uo = pf.as_mut().unwrap();
            uo.destroy(self);

            self.device.destroy_descriptor_pool(self.descriptor_pool, None);
            self.device.destroy_device(None);
            self.surface_loader.destroy_surface(self.surface, None);
            self.debug_utils_loader
                .destroy_debug_utils_messenger(self.debug_call_back, None);
            self.instance.destroy_instance(None);
        }
    }

    fn is_device_suitable(
        instance: &ash::Instance,
        surface: &ash::extensions::khr::Surface,
        surface_khr: vk::SurfaceKHR,
        device: vk::PhysicalDevice,
    ) -> bool {
        let (graphics, present, compute) = Self::find_queue_families(instance, surface, surface_khr, device);
        let extention_support = Self::check_device_extension_support(instance, device);
        let is_swapchain_adequate = {
            let details = SwapchainSupportDetails::new(device, surface, surface_khr);
            !details.formats.is_empty() && !details.present_modes.is_empty()
        };
        let features = unsafe { instance.get_physical_device_features(device) };
        graphics.is_some()
            && present.is_some()
            && extention_support
            && is_swapchain_adequate
            && features.sampler_anisotropy == vk::TRUE
    }

    fn get_required_device_extensions() -> [&'static CStr; 2] {
        [vk::KhrSwapchainFn::name(), ash::extensions::khr::Maintenance1::name()]
    }

    fn check_device_extension_support(instance: &ash::Instance, device: vk::PhysicalDevice) -> bool {
        let required_extentions = Self::get_required_device_extensions();

        let extension_props = unsafe {
            instance
                .enumerate_device_extension_properties(device)
                .unwrap()
        };

        for required in required_extentions.iter() {
            let found = extension_props.iter().any(|ext| {
                let name = unsafe { CStr::from_ptr(ext.extension_name.as_ptr()) };
                required == &name
            });

            if !found {
                return false;
            }
        }

        true
    }

    fn find_queue_families(
        instance: &ash::Instance,
        surface: &ash::extensions::khr::Surface,
        surface_khr: vk::SurfaceKHR,
        device: vk::PhysicalDevice,
    ) -> (Option<u32>, Option<u32>, Option<u32>) {
        let mut graphics = None;
        let mut present = None;
        let mut compute = None;

        let props = unsafe { instance.get_physical_device_queue_family_properties(device) };
        for (index, family) in props.iter().filter(|f| f.queue_count > 0).enumerate() {
            let index = index as u32;

            if family.queue_flags.contains(vk::QueueFlags::GRAPHICS) && graphics.is_none() {
                graphics = Some(index);
            }

            if family.queue_flags.contains(vk::QueueFlags::COMPUTE) && compute.is_none() {
                compute = Some(index);
            }

            let present_support = unsafe {
                surface
                    .get_physical_device_surface_support(device, index, surface_khr)
                    .unwrap()
            };
            if present_support && present.is_none() {
                present = Some(index);
            }

            if graphics.is_some() && present.is_some() {
                break;
            }
        }

        (graphics, present, compute)
    }

    #[cfg(feature = "statistic")]
    fn pipeline_statistic() -> u32 {
        return 1;
    }

    #[cfg(not(feature = "statistic"))]
    fn pipeline_statistic() -> u32 {
        return 0;
    }


    pub unsafe fn create<W: HasRawWindowHandle>(window: &W, window_width: u32, window_height: u32) -> Self {
        let app_name = CString::new("RichRender").unwrap();

        let layer_names = [CString::new("VK_LAYER_KHRONOS_validation").unwrap()];
        let layers_names_raw: Vec<*const i8> = layer_names
            .iter()
            .map(|raw_name| raw_name.as_ptr())
            .collect();

        let surface_extensions = ash_window::enumerate_required_extensions(window).unwrap();
        let mut extension_names_raw = surface_extensions
            .iter()
            .map(|ext| ext.as_ptr())
            .collect::<Vec<_>>();
        extension_names_raw.push(ash::extensions::ext::DebugUtils::name().as_ptr());
        //extension_names_raw.push(ash::extensions::khr::Maintenance1::name().as_ptr());

        let appinfo = vk::ApplicationInfo::builder()
            .application_name(&app_name)
            .application_version(vk::make_version(0, 1, 0))
            .engine_name(&app_name)
            .engine_version(vk::make_version(0, 1, 0))
            .api_version(vk::make_api_version(0, 1, 0, 0));

        let create_info = vk::InstanceCreateInfo::builder()
            .application_info(&appinfo)
            .enabled_layer_names(&layers_names_raw)
            .enabled_extension_names(&extension_names_raw);

        let entry = ash::Entry::new().unwrap();
        let instance: ash::Instance = entry
            .create_instance(&create_info, None)
            .expect("Instance creation error");

        let debug_info = vk::DebugUtilsMessengerCreateInfoEXT::builder()
            .message_severity(
                vk::DebugUtilsMessageSeverityFlagsEXT::ERROR
                    | vk::DebugUtilsMessageSeverityFlagsEXT::WARNING
                    | vk::DebugUtilsMessageSeverityFlagsEXT::INFO,
            )
            .message_type(vk::DebugUtilsMessageTypeFlagsEXT::all())
            .pfn_user_callback(Some(vulkan_debug_callback));

        let debug_utils_loader = ash::extensions::ext::DebugUtils::new(&entry, &instance);
        let debug_call_back = debug_utils_loader
            .create_debug_utils_messenger(&debug_info, None)
            .unwrap();
        let surface = ash_window::create_surface(&entry, &instance, window, None).unwrap();

        let surface_loader = ash::extensions::khr::Surface::new(&entry, &instance);

        let devices = instance
            .enumerate_physical_devices()
            .expect("Physical device error");

        let physical_device = devices
            .into_iter()
            .find(|device| Self::is_device_suitable(&instance, &surface_loader, surface, *device))
            .expect("No suitable physical device.");
        let (graphics_index_o, present_index_o, compute_index_o) = Self::find_queue_families(&instance,
                                                                                             &surface_loader, surface, physical_device);

        let graphics_index = graphics_index_o.expect("no graphic queue found");
        let present_index = present_index_o.expect("no present queue found");
        let compute_index = compute_index_o.expect("no compute queue found");

        let device_extension_names_raw = [ash::extensions::khr::Swapchain::name().as_ptr(),
            ash::extensions::khr::Maintenance1::name().as_ptr()];
        let features = vk::PhysicalDeviceFeatures {
            shader_clip_distance: 1,
            tessellation_shader: 1,
            pipeline_statistics_query: Self::pipeline_statistic(),
            ..Default::default()
        };

        let queue_priorities = [1.0f32];

        let queue_create_infos = {
            let mut indices = vec![graphics_index, present_index];
            indices.dedup();
            indices
                .iter()
                .map(|index| {
                    vk::DeviceQueueCreateInfo::builder()
                        .queue_family_index(*index)
                        .queue_priorities(&queue_priorities)
                        .build()
                })
                .collect::<Vec<_>>()
        };


        let device_create_info = vk::DeviceCreateInfo::builder()
            .queue_create_infos(&queue_create_infos)
            .enabled_extension_names(&device_extension_names_raw)
            .enabled_features(&features);

        let device: ash::Device = instance
            .create_device(physical_device, &device_create_info, None)
            .unwrap();

        let present_queue = device.get_device_queue(present_index, 0);
        let graphics_queue = device.get_device_queue(graphics_index, 0);
        let compute_queue = device.get_device_queue(compute_index, 0);
        let device_memory_properties = instance.get_physical_device_memory_properties(physical_device);
        let swapchain_loader = ash::extensions::khr::Swapchain::new(&instance, &device);

        let render_config = RenderConfig {
            msaa: vk::SampleCountFlags::TYPE_1,
            apply_post_effect: false,
            apply_shadow: false,
            color_format: vk::Format::B8G8R8A8_UNORM,
            depth_format: vk::Format::D32_SFLOAT,
            shadow_map_dim: 2048f32,
        };

        //todo description size
        let pool_size = [
            vk::DescriptorPoolSize {
                ty: vk::DescriptorType::UNIFORM_BUFFER,
                descriptor_count: 10,
            },
            vk::DescriptorPoolSize {
                ty: vk::DescriptorType::COMBINED_IMAGE_SAMPLER,
                descriptor_count: 1000,
            },
        ];

        let descriptor_pool = device.create_descriptor_pool(
            &vk::DescriptorPoolCreateInfo::builder()
                .max_sets(2000)
                .flags(vk::DescriptorPoolCreateFlags::FREE_DESCRIPTOR_SET)
                .pool_sizes(&pool_size).build(), None,
        ).expect("create descriptor pool failed");


        let props = unsafe {
            instance
                .get_physical_device_properties(physical_device)
        };
        let min_uniform_buffer_offset_alignment = props.limits.min_uniform_buffer_offset_alignment as u32;

        let collection = ShaderCollection::create();

        #[cfg(feature = "statistic")]
        let statistic = RenderStatistic::create(&device);

        RenderContext {
            window_width,
            window_height,
            entry,
            instance,
            device,
            device_memory_properties,
            graphics_queue,
            present_queue,
            compute_queue,
            debug_utils_loader,
            physical_device,
            surface_loader,
            swapchain_loader,
            surface,
            debug_call_back,
            graphics_queue_family_index: graphics_index,
            compute_queue_family_index: compute_index,
            render_config,
            descriptor_pool,
            staging_buffers: vec![],
            resources: HashMap::new(),
            per_frame_uniform: None,
            models: HashMap::new(),
            min_uniform_buffer_offset_alignment,
            shader_modules: collection,
            #[cfg(feature = "statistic")]
            statistic,
        }
    }

    pub fn find_memory_type_index(
        &self,
        memory_req: &vk::MemoryRequirements,
        flags: vk::MemoryPropertyFlags,
    ) -> Option<u32> {
        self.device_memory_properties.memory_types[..self.device_memory_properties.memory_type_count as _]
            .iter()
            .enumerate()
            .find(|(index, memory_type)| {
                (1 << index) & memory_req.memory_type_bits != 0
                    && memory_type.property_flags & flags == flags
            })
            .map(|(index, _memory_type)| index as _)
    }

    pub fn push_staging_buffer(&mut self, buffer: Buffer) {
        self.staging_buffers.push(buffer);
    }

    pub fn flush_staging_buffer(&mut self) {
        let mut buffers = mem::take(&mut self.staging_buffers);
        for buffer in buffers.iter_mut() {
            buffer.destroy(self)
        }
    }

    pub fn insert_resource<T>(&mut self, resource: T) where T: RenderResource {
        self.resources.insert(TypeId::of::<T>(), Box::new(resource));
    }

    pub fn get_resource<T>(&self) -> &T where T: RenderResource {
        let id = TypeId::of::<T>();
        let res = self.resources.get(&id).unwrap().as_any();
        match res.downcast_ref::<T>() {
            Some(t) => t,
            None => panic!("error resource"),
        }
    }

    pub fn get_resource_mut<T>(&mut self) -> &mut T where T: RenderResource {
        let id = TypeId::of::<T>();
        let res = self.resources.get_mut(&id).unwrap().as_any_mut();
        match res.downcast_mut::<T>() {
            Some(t) => t,
            None => panic!("error resource"),
        }
    }

    pub fn insert_model(&mut self, handle: Handle<GltfAsset>, model: ModelRenderer) {
        self.models.insert(handle, model);
    }

    pub fn get_model(&self, handle: &Handle<GltfAsset>) -> Option<&ModelRenderer> {
        self.models.get(&handle)
    }

    pub fn get_ubo_alignment<T>(&self) -> u32 {
        let min_alignment = self.min_uniform_buffer_offset_alignment;
        let t_size = size_of::<T>() as u32;

        if t_size <= min_alignment {
            min_alignment
        } else {
            min_alignment * (t_size as f32 / min_alignment as f32).ceil() as u32
        }
    }
}

