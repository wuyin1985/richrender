mod render_plugin;
mod swapchain_mgr;
mod render_context;
mod render_runner;
mod render_texture;
mod forward_render;
mod command_buffer_list;
mod simple_draw_object;
mod model;
mod aabb;
mod material;
mod mesh;

pub use render_plugin::RenderPlugin;

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        assert_eq!(2 + 2, 4);
    }
}
