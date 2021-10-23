mod egui_integrate;
mod egui_render;
mod file_selector;
mod event;
mod entity_list;

use std::cell::{Cell, RefCell};
use rich_engine::prelude::*;
use egui;
use egui::Align2;
use rich_engine::{AnimationRuntime, AnimCommand, AnimCommands, Diagnostic, Diagnostics, DisplayName, FlyCamera, FrameTimeDiagnosticsPlugin, GltfAsset, InputSystem, RenderRunner};
use crate::egui_integrate::{EguiContext, EguiPlugin};
use crate::file_selector::FileSelector;
use std::env;
use structopt::StructOpt;
use std::path::{Path, PathBuf};
use crate::event::EditorEvent;

#[derive(Debug, StructOpt)]
#[structopt(name = "args", about = "rich args.")]
struct Cli {
    #[structopt(parse(from_os_str))]
    search_dirs: Vec<PathBuf>,
}

struct EditorPlugin {}

impl Plugin for EditorPlugin {
    fn build(&self, app: &mut AppBuilder) {
        app.add_plugin(EguiPlugin);
        app.add_event::<EditorEvent>();

        let cli = Cli::from_args();
        let search_dirs = cli.search_dirs;

        app.insert_resource(EditorState::create(search_dirs));

        app.add_system(process.system());

        app.add_system(entity_list::draw_entity_list.system());
        app.add_system(entity_list::draw_entity_property.system());

        app.add_system(process_editor_events.system());

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
pub struct EditorState {
    file_selector: FileSelector,
    current_select_entity: Option<Entity>,
}

impl EditorState {
    pub fn create(search_dirs: Vec<PathBuf>) -> Self {
        EditorState {
            file_selector: FileSelector::create(search_dirs),
            current_select_entity: None,
        }
    }
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

fn process(mut state: ResMut<EditorState>
           , egui_context: Option<Res<EguiContext>>
           , mut render_runner: Option<ResMut<RenderRunner>>
           , time: Res<Time>
           , diagnostics: Res<Diagnostics>
           , mut event_writer: EventWriter<EditorEvent>) {
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

            if let Some(rr) = &mut render_runner {
                ui.heading("rendering");
                let statistic = &rr.context.statistic;

                ui.horizontal(|ui| {
                    // for i in 0..co
                    // ui.label(format!("Fps: {}", average_fps));
                });

                ui.checkbox(&mut rr.grass.enable_draw, "draw grass");
            }
        });

        //editor
        egui::Window::new("Entities").anchor(Align2::LEFT_TOP, egui::Vec2::new(0.0, 0.0)).show(ctx.ctx(), |ui| {
            ui.horizontal(|ui| {
                ui.heading("list");
                let popup_id = ui.make_persistent_id("open_model_list_popup");
                let button_rsp = ui.add(egui::Button::new("+").small());
                if button_rsp.clicked() {
                    ui.memory().toggle_popup(popup_id);
                }

                egui::popup::popup_below_widget(ui, popup_id, &button_rsp, |ui| {
                    state.file_selector.draw(ui, event_writer);
                })
            });
        });
    }
}

fn process_editor_events(mut event_reader: EventReader<EditorEvent>
                         , mut commands: Commands
                         , mut asset_server: ResMut<AssetServer>) {
    for e in event_reader.iter() {
        match e {
            EditorEvent::CreateAsset(p) => {
                let handle: Handle<GltfAsset> = asset_server.load(p.as_str());
                let s = 1.0;
                let scale = Vec3::new(s, s, s);
                let pos = Vec3::new(0.0, 0f32, -1f32);
                let t = Transform::from_scale(scale) *
                    Transform::from_translation(pos + Vec3::new(0f32, 0f32, 0.0));

                let path = Path::new(&p).file_stem().unwrap().to_str().unwrap();
                commands.spawn().insert(handle).insert(t).insert(DisplayName::from_str(path))
                    .insert(AnimationRuntime::default()).insert(AnimCommands::create_with_commands(vec![AnimCommand::Play { index: 0 }]));
            }

            EditorEvent::DeleteEntity(e) => {
                commands.entity(*e).despawn_recursive();
            }
        }
    }
}

pub fn get_external_startup_info() -> rich_engine::ExternalStartupInfo {
    rich_engine::ExternalStartupInfo {
        external_plugins: vec![Box::new(EditorPlugin {})]
    }
}