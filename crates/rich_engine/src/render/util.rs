use std::ffi::c_void;
use ash::vk::DeviceSize;
use ash::vk;
use ash::util::Align;
use std::mem::size_of;
use crate::render::render_context::RenderContext;
use crate::render::texture::Texture;

pub unsafe fn mem_copy<T: Copy>(ptr: *mut c_void, data: &[T]) {
    let elem_size = size_of::<T>() as DeviceSize;
    let size = data.len() as DeviceSize * elem_size;
    let mut align = Align::new(ptr, elem_size, size);
    align.copy_from_slice(data);
}

pub unsafe fn any_as_u8_slice<T: Sized>(p: &T) -> &[u8] {
    ::std::slice::from_raw_parts(
        (p as *const T) as *const u8,
        ::std::mem::size_of::<T>(),
    )
}

pub fn has_stencil_component(format: vk::Format) -> bool {
    format == vk::Format::D32_SFLOAT_S8_UINT || format == vk::Format::D24_UNORM_S8_UINT
}

fn has_mipmaps(filter: gltf::texture::MinFilter) -> bool {
    filter != gltf::texture::MinFilter::Linear &&
        filter != gltf::texture::MinFilter::Nearest
}

fn map_mipmap_filter(min_filter: gltf::texture::MinFilter) -> vk::SamplerMipmapMode {
    match min_filter {
        gltf::texture::MinFilter::Nearest => vk::SamplerMipmapMode::NEAREST,
        gltf::texture::MinFilter::Linear => vk::SamplerMipmapMode::NEAREST,
        gltf::texture::MinFilter::NearestMipmapNearest => vk::SamplerMipmapMode::NEAREST,
        gltf::texture::MinFilter::LinearMipmapNearest => vk::SamplerMipmapMode::NEAREST,
        gltf::texture::MinFilter::NearestMipmapLinear => vk::SamplerMipmapMode::LINEAR,
        gltf::texture::MinFilter::LinearMipmapLinear => vk::SamplerMipmapMode::LINEAR,
    }
}

fn map_mag_filter(mag_filter: gltf::texture::MagFilter) -> vk::Filter {
    match mag_filter {
        gltf::texture::MagFilter::Nearest => vk::Filter::NEAREST,
        gltf::texture::MagFilter::Linear => vk::Filter::LINEAR,
    }
}

fn map_min_filter(min_filter: gltf::texture::MinFilter) -> vk::Filter {
    match min_filter {
        gltf::texture::MinFilter::Nearest => vk::Filter::NEAREST,
        gltf::texture::MinFilter::Linear => vk::Filter::LINEAR,
        gltf::texture::MinFilter::NearestMipmapNearest => vk::Filter::NEAREST,
        gltf::texture::MinFilter::LinearMipmapNearest => vk::Filter::LINEAR,
        gltf::texture::MinFilter::NearestMipmapLinear => vk::Filter::NEAREST,
        gltf::texture::MinFilter::LinearMipmapLinear => vk::Filter::LINEAR,
    }
}

fn map_wrap_mode(wrap_mode: gltf::texture::WrappingMode) -> vk::SamplerAddressMode {
    match wrap_mode {
        gltf::texture::WrappingMode::ClampToEdge => vk::SamplerAddressMode::CLAMP_TO_EDGE,
        gltf::texture::WrappingMode::MirroredRepeat => vk::SamplerAddressMode::MIRRORED_REPEAT,
        gltf::texture::WrappingMode::Repeat => vk::SamplerAddressMode::REPEAT,
    }
}


pub struct Gltf2VkConvertor {}

impl Gltf2VkConvertor {
    pub fn format(gltf_format: gltf::image::Format) -> vk::Format {
        match gltf_format {
            gltf::image::Format::R8 => vk::Format::R8_UNORM,
            gltf::image::Format::R8G8 => vk::Format::R8G8_UNORM,
            gltf::image::Format::R8G8B8 => vk::Format::R8G8B8_UNORM,
            gltf::image::Format::R8G8B8A8 => vk::Format::R8G8B8A8_UNORM,
            gltf::image::Format::B8G8R8 => vk::Format::B8G8R8_UNORM,
            gltf::image::Format::B8G8R8A8 => vk::Format::B8G8R8A8_UNORM,
            gltf::image::Format::R16 => vk::Format::R16_UNORM,
            gltf::image::Format::R16G16 => vk::Format::R16G16_UNORM,
            gltf::image::Format::R16G16B16 => vk::Format::R16G16B16_UNORM,
            gltf::image::Format::R16G16B16A16 => vk::Format::R16G16B16A16_UNORM,
        }
    }

    pub fn sampler(context: &RenderContext, texture: &Texture, gltf_sampler: gltf::texture::Sampler) -> vk::Sampler {
        let min_filter = gltf_sampler.min_filter().unwrap_or(gltf::texture::MinFilter::Linear);
        let mag_filter = gltf_sampler.mag_filter().unwrap_or(gltf::texture::MagFilter::Linear);
        let has_mipmaps = has_mipmaps(min_filter);
        let max_lod = if has_mipmaps {
            texture.get_mip_map_count() as f32
        } else {
            0.25
        };

        let sampler_info = vk::SamplerCreateInfo::builder()
            .mag_filter(map_mag_filter(mag_filter))
            .min_filter(map_min_filter(min_filter))
            .address_mode_u(map_wrap_mode(gltf_sampler.wrap_s()))
            .address_mode_v(map_wrap_mode(gltf_sampler.wrap_t()))
            .address_mode_w(vk::SamplerAddressMode::REPEAT)
            .anisotropy_enable(has_mipmaps)
            .max_anisotropy(16.0)
            .border_color(vk::BorderColor::INT_OPAQUE_BLACK)
            .unnormalized_coordinates(false)
            .compare_enable(false)
            .compare_op(vk::CompareOp::ALWAYS)
            .mipmap_mode(map_mipmap_filter(min_filter))
            .mip_lod_bias(0.0)
            .min_lod(0.0)
            .max_lod(max_lod);

        unsafe {
            context
                .device
                .create_sampler(&sampler_info, None)
                .expect("Failed to create sampler")
        }
    }
}