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

fn startup(mut commmands: Commands, mut asset_server: ResMut<AssetServer>) {
    let s = 0.919468641f32;
    let scale = Vec3::new(s, s, s);
    let pos = Vec3::new(0.00248157978f32, 0f32, -1f32);
    // 
    // {
    //     let handle: Handle<GltfAsset> = asset_server.load("gltf/DamagedHelmet/DamagedHelmet.glb");
    //     let t = Transform::from_scale(scale) *
    //         Transform::from_translation(pos);
    // 
    //     commmands.spawn().insert(handle).insert(t);
    // }

    {
        let handle: Handle<GltfAsset> = asset_server.load("gltf/2CylinderEngine/2CylinderEngine.glb");
        let t = Transform::from_scale(scale) *
            Transform::from_translation(pos - Vec3::new(-0.02f32, 0.0, 0.0));

        commmands.spawn().insert(handle).insert(t);
    }
}


fn main() {
    App::build()
        // .insert_resource(LogSettings {
        //     filter: "".to_string(),
        //     level: Level::INFO
        // })
        .add_plugins(DefaultPlugins)
        .add_plugin(RenderPlugin {})
        .add_startup_system(startup.system()).run();
}
