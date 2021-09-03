use std::ffi::{CString, CStr};
use ash::vk;
use std::borrow::Cow;
use raw_window_handle::HasRawWindowHandle;

pub struct DeviceMgr {
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
    pub present_queue: vk::Queue,
    pub device_memory_properties: vk::PhysicalDeviceMemoryProperties,
    pub swapchain_loader: ash::extensions::khr::Swapchain,
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

impl DeviceMgr {
    pub fn destroy(&mut self) {
        unsafe {
            self.device.destroy_device(None);
            self.surface_loader.destroy_surface(self.surface, None);
            self.debug_utils_loader
                .destroy_debug_utils_messenger(self.debug_call_back, None);
            self.instance.destroy_instance(None);
        }
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

        let pdevices = instance
            .enumerate_physical_devices()
            .expect("Physical device error");
        let surface_loader = ash::extensions::khr::Surface::new(&entry, &instance);
        let (physical_device, queue_family_index) = pdevices
            .iter()
            .map(|physical_device| {
                instance
                    .get_physical_device_queue_family_properties(*physical_device)
                    .iter()
                    .enumerate()
                    .filter_map(|(index, info)| {
                        let supports_graphic_and_surface =
                            info.queue_flags.contains(vk::QueueFlags::GRAPHICS)
                                && surface_loader
                                .get_physical_device_surface_support(
                                    *physical_device,
                                    index as u32,
                                    surface,
                                )
                                .unwrap();
                        if supports_graphic_and_surface {
                            Some((*physical_device, index))
                        } else {
                            None
                        }
                    })
                    .next()
            })
            .flatten()
            .next()
            .expect("Couldn't find suitable device.");
        let queue_family_index_u = queue_family_index as u32;
        let device_extension_names_raw = [ash::extensions::khr::Swapchain::name().as_ptr()];
        let features = vk::PhysicalDeviceFeatures {
            shader_clip_distance: 1,
            ..Default::default()
        };
        let priorities = [1.0];

        let queue_info = [vk::DeviceQueueCreateInfo::builder()
            .queue_family_index(queue_family_index_u)
            .queue_priorities(&priorities)
            .build()];

        let device_create_info = vk::DeviceCreateInfo::builder()
            .queue_create_infos(&queue_info)
            .enabled_extension_names(&device_extension_names_raw)
            .enabled_features(&features);

        let device: ash::Device = instance
            .create_device(physical_device, &device_create_info, None)
            .unwrap();

        let present_queue = device.get_device_queue(queue_family_index_u, 0);
        let device_memory_properties = instance.get_physical_device_memory_properties(physical_device);
        let swapchain_loader = ash::extensions::khr::Swapchain::new(&instance, &device);

        DeviceMgr {
            window_width,
            window_height,
            entry,
            instance,
            device,
            device_memory_properties,
            present_queue,
            debug_utils_loader,
            physical_device,
            surface_loader,
            swapchain_loader,
            surface,
            debug_call_back,
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