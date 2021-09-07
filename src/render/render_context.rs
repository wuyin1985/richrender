use std::ffi::{CString, CStr};
use ash::vk;
use std::borrow::Cow;
use raw_window_handle::HasRawWindowHandle;
use crate::render::swapchain_mgr::SwapchainSupportDetails;

pub struct RenderConfig {
    pub msaa: vk::SampleCountFlags,
    pub apply_post_effect: bool,
    pub apply_shadow: bool,
    pub color_format: vk::Format,
    pub depth_format: vk::Format,
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
    pub device_memory_properties: vk::PhysicalDeviceMemoryProperties,
    pub swapchain_loader: ash::extensions::khr::Swapchain,
    pub graphics_queue_family_index: u32,
    pub render_config: RenderConfig,
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

    println!(
        "{:?}:\n{:?} [{} ({})] : {}\n",
        message_severity,
        message_type,
        message_id_name,
        &message_id_number.to_string(),
        message,
    );

    vk::FALSE
}

impl RenderContext {
    pub fn destroy(&mut self) {
        unsafe {
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
        let (graphics, present) = Self::find_queue_families(instance, surface, surface_khr, device);
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

    fn get_required_device_extensions() -> [&'static CStr; 1] {
        [ vk::KhrSwapchainFn::name()]
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
    ) -> (Option<u32>, Option<u32>) {
        let mut graphics = None;
        let mut present = None;

        let props = unsafe { instance.get_physical_device_queue_family_properties(device) };
        for (index, family) in props.iter().filter(|f| f.queue_count > 0).enumerate() {
            let index = index as u32;

            if family.queue_flags.contains(vk::QueueFlags::GRAPHICS) && graphics.is_none() {
                graphics = Some(index);
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

        (graphics, present)
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

        let appinfo = vk::ApplicationInfo::builder()
            .application_name(&app_name)
            .application_version(0)
            .engine_name(&app_name)
            .engine_version(0)
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
        let (graphics_index_o, present_index_o) = Self::find_queue_families(&instance, 
                                                                            &surface_loader, surface, physical_device);
        
        let graphics_index = graphics_index_o.unwrap();
        let present_index = present_index_o.unwrap();
        
        let device_extension_names_raw = [ash::extensions::khr::Swapchain::name().as_ptr()];
        let features = vk::PhysicalDeviceFeatures {
            shader_clip_distance: 1,
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
        let device_memory_properties = instance.get_physical_device_memory_properties(physical_device);
        let swapchain_loader = ash::extensions::khr::Swapchain::new(&instance, &device);

        let render_config = RenderConfig {
            msaa: vk::SampleCountFlags::TYPE_1,
            apply_post_effect: false,
            apply_shadow: false,
            color_format: vk::Format::B8G8R8A8_UNORM,
            depth_format: vk::Format::D32_SFLOAT,
        };

        RenderContext {
            window_width,
            window_height,
            entry,
            instance,
            device,
            device_memory_properties,
            graphics_queue,
            present_queue,
            debug_utils_loader,
            physical_device,
            surface_loader,
            swapchain_loader,
            surface,
            debug_call_back,
            graphics_queue_family_index: graphics_index,
            render_config,
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
}