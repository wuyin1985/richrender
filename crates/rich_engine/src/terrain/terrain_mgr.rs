// use serde_json::{Map, Number};
// use serde::{Serialize, Deserialize};
// use crate::{RenderContext, Texture};
// use ash::vk;
// use image::{DynamicImage, GenericImageView};
//
// const TERRAIN_DIR: &str = "assets/terrain";
// const TERRAIN_TEXTURE_DIR: &str = "assets/terrain/textures";
//
// #[derive(Serialize, Deserialize)]
// struct TerrainLayerInfo {
//     diffuse: String,
//     normal: String,
//     mask: String,
//     normal_scale: f32,
// }
//
// #[derive(Serialize, Deserialize)]
// struct TerrainInfo {
//     layers: Vec<TerrainLayerInfo>,
//     x: f32,
//     z: f32,
//     height: f32,
//     height_map_resolution: i32,
// }
//
// pub struct TerrainMgr {}
//
// pub struct TerrainLayer {
//     diffuse: Texture,
//     normal: Texture,
//     mask: Texture,
// }
//
// pub struct Terrain {
//     height_map: Texture,
//     layers: Vec<TerrainLayer>,
//     layer_weight_map: Texture,
//     x: f32,
//     z: f32,
//     height: f32,
//     height_map_resolution: i32,
// }
//
// fn image_format_2_vk_format(image: &DynamicImage) -> vk::Format {
//     match image {
//         DynamicImage::ImageRgb8(_) => {
//             vk::Format::R8G8B8_UNORM
//         }
//         DynamicImage::ImageRgba8(_) => {
//             vk::Format::R8G8B8A8_UNORM
//         }
//         _ => {
//             panic!("not supported image format {:?}", image);
//         }
//     }
// }
//
// fn load_terrain_texture(context: &mut RenderContext, upload_command_buffer: vk::CommandBuffer, name: &str) -> Texture {
//     let p = format!("{}/{}", TERRAIN_TEXTURE_DIR, name);
//     let img_data = image::io::Reader::open(p).unwrap().decode().unwrap();
//     let buffer = img_data.as_bytes();
//     let format = image_format_2_vk_format(&img_data);
//     Texture::create_from_data_with_format(context, upload_command_buffer, img_data.width(), img_data.height(), format, buffer)
// }
//
// impl Terrain {
//     pub fn destroy(&mut self, context: &RenderContext) {
//         let mut layers = std::mem::take(&mut self.layers);
//         for layer in &mut layers {
//             layer.diffuse.destroy(context);
//             layer.normal.destroy(context);
//             layer.mask.destroy(context);
//         }
//         self.layer_weight_map.destroy(context);
//         self.height_map.destroy(context);
//     }
//
//     pub fn load(context: &mut RenderContext, upload_command_buffer: vk::CommandBuffer, terrain_name: &str) -> Self {
//         let info_path = format!("{}/{}/info.json", TERRAIN_DIR, terrain_name);
//         let info_content = std::fs::read_to_string(info_path).unwrap();
//         let info: TerrainInfo = serde_json::from_str(&info_content).unwrap();
//
//         let layers = info.layers.iter().map(|li| {
//             TerrainLayer {
//                 diffuse: load_terrain_texture(context, upload_command_buffer, &li.diffuse),
//                 normal: load_terrain_texture(context, upload_command_buffer, &li.normal),
//                 mask: load_terrain_texture(context, upload_command_buffer, &li.mask),
//             }
//         }).collect::<Vec<_>>();
//
//         let height_map = {
//             let height_map_path = format!("{}/{}/{}.raw", TERRAIN_DIR, terrain_name, terrain_name);
//             let height_bytes = std::fs::read(height_map_path).unwrap();
//             Texture::create_from_data_with_format(context, upload_command_buffer, info.height_map_resolution as _, info
//                 .height_map_resolution as _, vk::Format::R8_UINT, &height_bytes)
//         };
//
//         let (base_width, base_height) = {
//             let layer = &layers[0];
//             layer.diffuse.get_size()
//         };
//
//         let layer_weight_map = {
//             let layer_weight_map_path = format!("{}/{}/{}_splatmap_0.png", TERRAIN_DIR, terrain_name, terrain_name);
//             let bytes = std::fs::read(layer_weight_map_path).unwrap();
//             //todo format?
//             Texture::create_from_data_with_format(context, upload_command_buffer, base_width, base_height, vk::Format::R32G32B32A32_SFLOAT, &bytes)
//         };
//
//
//         Self {
//             x: info.x,
//             z: info.z,
//             height: info.height,
//             height_map_resolution: info.height_map_resolution,
//             layers,
//             height_map,
//             layer_weight_map,
//         }
//     }
// }
//
