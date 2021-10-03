mod egui_integrate;
mod egui_render;

use rich_engine::prelude::*;
use egui;
use crate::egui_integrate::{EguiContext, EguiPlugin};

struct EditorPlugin {}

impl Plugin for EditorPlugin {
    fn build(&self, app: &mut AppBuilder) {
        app.add_plugin(EguiPlugin);

        app.add_system(
            process.system()
        );
    }
}

pub fn process(egui_context: Option<Res<EguiContext>>) {
    if let Some(ctx) = &egui_context {
        egui::Window::new("Hello").show(ctx.ctx(), |ui| {
            ui.label("world");
        });
    }
}

pub fn get_external_startup_info() -> rich_engine::ExternalStartupInfo {
    rich_engine::ExternalStartupInfo {
        external_plugins: vec![Box::new(EditorPlugin {})]
    }
}