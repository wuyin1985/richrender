#[macro_use]
extern crate lazy_static;

use bevy::prelude::*;
use crate::render::RenderPlugin;
use bevy::log::{LogSettings, Level};

mod render;
mod game;

fn startup() {
    info!("hello world!")
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
