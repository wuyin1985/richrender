

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
mod grass;
mod compute;

use bevy::prelude::*;
pub use render_plugin::RenderPlugin;
pub use texture::Texture;
pub use render_context::RenderContext;
pub use forward_render::ForwardRenderPass;
pub use buffer::Buffer;
pub use render_runner::RenderRunner;
pub use render_plugin::RenderInitEvent;
pub use fly_camera::FlyCamera;


#[derive(Debug, Hash, PartialEq, Eq, Clone, StageLabel)]
pub enum RenderStage {
    Prepare,
    BeginUpload,
    Upload,
    EndUpload,
    BeginDraw,
    Draw,
    EndDraw,
}
