mod egui_integrate;
mod egui_render;

use rich_engine::prelude::*;
use egui;
use egui::Align2;
use rich_engine::{Diagnostic, Diagnostics, FlyCamera, FrameTimeDiagnosticsPlugin, InputSystem, RenderRunner};
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


fn parse_diagnostic(diagnostic: &Diagnostic) -> Option<(String, String)> {
    if let Some(value) = diagnostic.value() {
        if let Some(average) = diagnostic.average() {
            return Some(
                (format!("{:.2}{:1}", value, diagnostic.suffix),
                 format!("{:.4}{:}", average, diagnostic.suffix))
            );
        }
    }

    None
}

fn process(mut state: Local<EditorState>
           , egui_context: Option<Res<EguiContext>>
           , render_runner: Option<Res<RenderRunner>>
           , time: Res<Time>
           , diagnostics: Res<Diagnostics>) {
    if let Some(ctx) = &egui_context {
        egui::Window::new("Statistics").anchor(Align2::RIGHT_TOP, egui::Vec2::new(0.0, 0.0)).show(ctx.ctx(), |ui| {
            ui.heading("summary");
            ui.horizontal(|ui| {
                if let Some((fps, average_fps)) = parse_diagnostic(diagnostics.get(FrameTimeDiagnosticsPlugin::FPS).unwrap()) {
                    ui.label(format!("Fps: {}", average_fps));
                }
                if let Some((frame_time, average_frame_time)) = parse_diagnostic(diagnostics.get(FrameTimeDiagnosticsPlugin::FRAME_TIME).unwrap()) {
                    ui.label(format!("Frame Time: {}", average_frame_time));
                }
            });

            if let Some(rr) = &render_runner {
                ui.heading("rendering");
                let statistic = &rr.context.statistic;

                ui.horizontal(|ui| {
                    // for i in 0..co
                    // ui.label(format!("Fps: {}", average_fps));
                });

            }
        });
    }
}

pub fn get_external_startup_info() -> rich_engine::ExternalStartupInfo {
    rich_engine::ExternalStartupInfo {
        external_plugins: vec![Box::new(EditorPlugin {})]
    }
}