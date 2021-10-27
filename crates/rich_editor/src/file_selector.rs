use std::path::PathBuf;
use crate::egui_integrate::egui::{Align, Color32, Ui};
use egui::containers::ScrollArea;
use glob::glob;
use rich_engine::{AnimCommand, AnimCommands, GltfAsset};
use rich_engine::prelude::*;
use crate::EditorState;
use crate::event::EditorEvent;

#[derive(Debug, Default)]
pub struct FileSelector {
    track_item: usize,
    tack_item_align: Align,
    offset: f32,
    search_dirs: Vec<PathBuf>,
    cache_files: Vec<PathBuf>,
}

impl FileSelector {
    pub fn create(search_dirs: Vec<PathBuf>) -> Self {
        let mut s = Self {
            track_item: 25,
            tack_item_align: Align::Center,
            offset: 0.0,
            search_dirs,
            cache_files: Vec::new(),
        };
        s.refresh_search_dirs();
        s
    }

    pub fn refresh_search_dirs(&mut self) {
        let paths = self.search_dirs.iter().map(|dir| {
            glob(format!("{}/**/*.glb", dir.to_str().unwrap()).as_str()).expect("failed to glob")
        }).flatten().filter_map(|file| {
            if let Ok(fp) = file {
                return Some(fp);
            }
            None
        }).collect::<Vec<PathBuf>>();
        self.cache_files = paths;
    }

    pub fn draw(&mut self, ui: &mut Ui,mut event_writer: EventWriter<EditorEvent>) {
        ui.set_min_width(200.0);
        let mut scroll_area = ScrollArea::from_max_height(300.0);
        ui.separator();
        scroll_area.show(ui, |ui| {
            ui.vertical(|ui| {
                for path in &self.cache_files {
                    let name = path.file_name().unwrap().to_str().unwrap();
                    if ui.add(egui::Button::new(name).fill(Color32::TRANSPARENT)).clicked() {
                        let rp = path.strip_prefix("assets/").unwrap();
                        event_writer.send(EditorEvent::CreateAsset(rp.to_str().unwrap().to_string()));
                        // let handle: Handle<GltfAsset> = asset_server.load(rp.to_str().unwrap());
                        // let t = Transform::from_scale(scale) *
                        //     Transform::from_translation(pos);
                        //
                        // commmands.spawn().insert(handle).insert(t)
                        //     .insert(AnimationRuntime::default()).insert(AnimCommands::create_with_commands(vec![AnimCommand::Play { index: 0 }]));
                    }
                }
            });
        });

        ui.separator();
    }
}