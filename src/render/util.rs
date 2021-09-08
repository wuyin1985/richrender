use std::ffi::c_void;
use ash::vk::DeviceSize;
use ash::vk;
use ash::util::Align;
use std::mem::size_of;

pub unsafe fn mem_copy<T: Copy>(ptr: *mut c_void, data: &[T]) {
    let elem_size = size_of::<T>() as DeviceSize;
    let size = data.len() as DeviceSize * elem_size;
    let mut align = Align::new(ptr, elem_size, size);
    align.copy_from_slice(data);
}


pub fn has_stencil_component(format: vk::Format) -> bool {
    format == vk::Format::D32_SFLOAT_S8_UINT || format == vk::Format::D24_UNORM_S8_UINT
}