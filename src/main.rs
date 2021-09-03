use bevy::prelude::*;
use crate::render::RenderPlugin;

mod render;

fn startup() {
    println!("hello world!")
}

fn main() {
    App::build().add_plugins(DefaultPlugins).add_plugin(RenderPlugin {}).add_startup_system(startup.system()).run();
}
