

mod render_plugin;
mod swapchain_mgr;
mod render_context;
mod render_runner;
mod texture;
mod forward_render;
mod command_buffer_list;
mod model;
mod aabb;
mod material;
mod mesh;
mod buffer;
mod vertex;
mod util;
mod model_meta;
mod node;
pub mod model_renderer;
mod graphic_pipeline;
mod vertex_layout;
mod camera;
mod fly_camera;
mod uniform;
pub mod gltf_asset_loader;
mod shader_const;
mod shader_collection;
mod debug;

pub use render_plugin::RenderPlugin;

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        assert_eq!(2 + 2, 4);
    }
}
