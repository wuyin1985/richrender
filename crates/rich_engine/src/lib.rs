#[macro_use]
extern crate lazy_static;

use std::borrow::Cow;
use bevy::prelude::*;
use crate::render::{RenderPlugin};
use bevy::log::{LogSettings, Level};
use std::ops::{Deref, DerefMut};
use crate::render::model_renderer::ModelData;

mod render;
mod game;
mod terrain;

pub mod prelude {
    pub use bevy::prelude::*;
    pub use bevy::window::WindowId;
    pub use bevy::ecs as bevy_ecs;
    pub use crate::render::RenderStage;
    pub use bevy::math::Vec4Swizzles;
    pub use bevy::math::Vec3Swizzles;
    pub use crate::render::CameraOpEvent;
}

pub use winit::window::CursorIcon;
pub use bevy::winit as bevy_winit;
pub use bevy::window;
pub use ash;
pub use bevy::ecs::system::SystemParam;
pub use bevy::input::mouse::*;
pub use bevy::input::*;
pub use bevy::diagnostic::*;

pub use crate::render::Texture;
pub use crate::render::RenderContext;
pub use crate::render::ForwardRenderPass;
pub use crate::render::Buffer;
pub use crate::render::RenderRunner;
pub use crate::render::RenderInitEvent;
pub use crate::render::FlyCamera;
pub use crate::render::AnimationRuntime;
pub use crate::render::AnimCommands;
pub use crate::render::AnimCommand;
pub use crate::render::gltf_asset_loader::GltfAsset;
pub use crate::render::Camera;
pub use crate::render::RenderCamera;



pub struct DisplayName {
    pub name: String,
}

impl DisplayName {
    pub fn from_string(name: String) -> Self {
        Self {
            name
        }
    }

    pub fn from_str(name: &str) -> Self {
        Self {
            name: name.to_string()
        }
    }
}

pub struct StartupArgs {
    pub data: Vec<String>,
}

impl StartupArgs {
    pub fn create(data: Vec<String>) -> Self {
        Self {
            data
        }
    }
}

fn init(mut commmands: Commands, mut asset_server: ResMut<AssetServer>) {
    // let s = 1.0;
    // let scale = Vec3::new(s, s, s);
    // let pos = Vec3::new(0.0, 0f32, -1f32);
    //
    // {
    //     let handle: Handle<GltfAsset> = asset_server.load("gltf/CesiumMan.glb");
    //     let t = Transform::from_scale(scale) *
    //         Transform::from_translation(pos + Vec3::new(0f32, 0f32, 0.0));
    //
    //     commmands.spawn().insert(handle).insert(t)
    //         .insert(AnimationRuntime::default()).insert(AnimCommands::create_with_commands(vec![AnimCommand::Play { index: 0 }]));
    // }
}

pub struct ExternalStartupInfo {
    pub external_plugins: Vec<Box<dyn Plugin>>,
}

pub fn startup(info: ExternalStartupInfo, args: Vec<String>) {
    let mut app = App::build();

    app.insert_resource(StartupArgs::create(args));
    // .insert_resource(LogSettings {
    //     filter: "".to_string(),
    //     level: Level::INFO
    // })
    app.add_plugins(DefaultPlugins)
        .add_plugin(RenderPlugin {})
        .add_plugin(bevy::diagnostic::FrameTimeDiagnosticsPlugin::default());

    for p in info.external_plugins {
        p.build(&mut app);
    }

    app.add_startup_system(init.system()).run();
}
