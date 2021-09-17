#[macro_use]
extern crate lazy_static;

use bevy::prelude::*;
use crate::render::RenderPlugin;
use bevy::log::{LogSettings, Level};
use std::ops::{Deref, DerefMut};
use crate::render::gltf_asset_loader::GltfAsset;

mod render;
mod game;

fn startup(mut commmands: Commands, mut asset_server: ResMut<AssetServer>) {
    let handle: Handle<GltfAsset> = asset_server.load("gltf/DamagedHelmet/DamagedHelmet.glb");
    let s = 0.919468641f32;
    commmands.spawn().insert(handle).insert(
        Transform::from_scale(Vec3::new(s,s,s)) *
            Transform::from_translation(Vec3::new(0.00248157978f32, 0f32, -1f32)));
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
