
mod egui_integrate;
mod egui_render;

use rich_engine::prelude::*;

struct EditorPlugin {}

impl Plugin for EditorPlugin {
    fn build(&self, app: &mut AppBuilder) {
        
    }
}

pub fn get_external_startup_info() -> rich_engine::ExternalStartupInfo {
    rich_engine::ExternalStartupInfo {
        external_plugins: vec![Box::new(EditorPlugin {})]
    }
}