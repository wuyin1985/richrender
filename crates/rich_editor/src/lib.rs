mod egui_integrate;
mod egui_render;

use rich_engine::prelude::*;
use egui;
use rich_engine::{FlyCamera, InputSystem};
use crate::egui_integrate::{EguiContext, EguiPlugin};

struct EditorPlugin {}

impl Plugin for EditorPlugin {
    fn build(&self, app: &mut AppBuilder) {
        app.add_plugin(EguiPlugin);

        app.add_system(
            process.system().config(|config| {
                config.0 = Some(EditorState {
                    name: "hello".to_string(),
                    age: 2,

                })
            })
        );

        app.add_system_to_stage(CoreStage::PreUpdate, enable_fly_camera.system().after(InputSystem));
    }
}

fn enable_fly_camera(egui_context: Option<Res<EguiContext>>, mut query: Query<(&mut FlyCamera)>) {
    if let Some(ctx) = &egui_context {
        if let Some(rd) = &ctx.render {
            for (mut options) in query.iter_mut() {
                options.enabled = !ctx.ctx().is_pointer_over_area();
            }
        }
    }
}

#[derive(Debug, Default)]
struct EditorState {
    name: String,
    age: i32,
}

fn process(mut state: Local<EditorState>, egui_context: Option<Res<EguiContext>>) {
    if let Some(ctx) = &egui_context {
        egui::Window::new("Hello").show(ctx.ctx(), |ui| {
            ui.heading("My egui Application");
            ui.horizontal(|ui| {
                ui.label("Your name: ");
                ui.text_edit_singleline(&mut state.name);
            });
            ui.add(egui::Slider::new(&mut state.age, 0..=120).text("age"));
            if ui.button("Click each year").clicked() {
                state.age += 1;
            }
            ui.label(format!("Hello '{}', age {}", state.name, state.age));
        });
    }
}

pub fn get_external_startup_info() -> rich_engine::ExternalStartupInfo {
    rich_engine::ExternalStartupInfo {
        external_plugins: vec![Box::new(EditorPlugin {})]
    }
}