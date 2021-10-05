#[macro_use]
extern crate lazy_static;

use bevy::prelude::*;
use crate::render::RenderPlugin;
use bevy::log::{LogSettings, Level};
use std::ops::{Deref, DerefMut};
use crate::render::gltf_asset_loader::GltfAsset;
use crate::render::model_renderer::ModelData;

mod render;
mod game;

pub mod prelude {
    pub use bevy::prelude::*;
    pub use bevy::window::WindowId;
    pub use bevy::ecs as bevy_ecs;
    pub use crate::render::RenderStage;
}

pub use winit::window::CursorIcon;
pub use bevy::winit as bevy_winit;
pub use bevy::window;
pub use ash;
pub use bevy::ecs::system::SystemParam;
pub use bevy::input::mouse::*;
pub use bevy::input::*;

pub use crate::render::Texture;
pub use crate::render::RenderContext;
pub use crate::render::ForwardRenderPass;
pub use crate::render::Buffer;
pub use crate::render::RenderRunner;
pub use crate::render::RenderInitEvent;
pub use crate::render::FlyCamera;

fn init(mut commmands: Commands, mut asset_server: ResMut<AssetServer>) {
    let s = 0.919468641f32;
    let scale = Vec3::new(s, s, s);
    let pos = Vec3::new(0.00248157978f32, 0f32, -1f32);

    // {
    //     let handle: Handle<GltfAsset> = asset_server.load("gltf/vulkanscene_shadow.gltf");
    //     let t = Transform::from_scale(scale) *
    //         Transform::from_translation(pos);
    // 
    //     commmands.spawn().insert(handle).insert(t);
    // }

    {
        let handle: Handle<GltfAsset> = asset_server.load("gltf/samplescene.gltf");
        let t = Transform::from_scale(scale) *
            Transform::from_translation(pos + Vec3::new(0f32, 0f32, 0.0));

        commmands.spawn().insert(handle).insert(t);
    }
}

pub struct ExternalStartupInfo {
    pub external_plugins: Vec<Box<dyn Plugin>>,
}

pub fn startup(info: ExternalStartupInfo) {
    let mut app = App::build();
    // .insert_resource(LogSettings {
    //     filter: "".to_string(),
    //     level: Level::INFO
    // })
    app.add_plugins(DefaultPlugins).add_plugin(RenderPlugin {});

    for p in info.external_plugins {
        p.build(&mut app);
    }

    app.add_startup_system(init.system()).run();
}
