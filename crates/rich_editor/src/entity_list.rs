use std::num::ParseFloatError;
use egui::{Align2, Direction, Ui};
use crate::EditorState;
use rich_engine::prelude::*;
use rich_engine::RenderRunner;
use crate::egui_integrate::egui::{Align, ScrollArea};
use crate::egui_integrate::EguiContext;
use crate::event::EditorEvent;

pub fn draw_entity_list(mut state: ResMut<EditorState>
                        , egui_context: Option<Res<EguiContext>>
                        , mut query: Query<(Entity, &Transform)>) {
    if let Some(ctx) = &egui_context {
        egui::Window::new("EntityList").anchor(Align2::LEFT_BOTTOM, egui::Vec2::new(0.0, 0.0)).show(ctx.ctx(), |ui| {
            let mut scroll_area = ScrollArea::from_max_height(300.0);
            scroll_area.show(ui, |ui|
                {
                    ui.vertical(|ui|
                        {
                            ui.with_layout(egui::Layout::from_main_dir_and_cross_align(Direction::TopDown, Align::Max).with_cross_justify(true),
                                           |ui| {
                                               for (entity, mut transform) in query.iter_mut() {
                                                   if ui.button(format!("[{:?}]", entity).as_str()).clicked() {
                                                       state.current_select_entity = Some(entity);
                                                   }
                                               }
                                           },
                            );
                        });
                });
        });
    }
}

fn draw_and_edit_float(ui: &mut Ui, data: &mut f32) -> bool {
    ui.add(egui::DragValue::new(data).speed(0.01f32)).changed()
}

pub fn draw_entity_property(mut state: ResMut<EditorState>
                            , egui_context: Option<Res<EguiContext>>
                            , mut event_writer: EventWriter<EditorEvent>
                            , mut query: Query<(Entity, &mut Transform)>) {
    if let Some(ctx) = &egui_context {
        egui::Window::new("EntityProperty").anchor(Align2::RIGHT_BOTTOM, egui::Vec2::new(0.0, 0.0)).show(ctx.ctx(), |ui| {
            if let Some(et) = state.current_select_entity {
                if let Ok((entity, mut transform)) = query.get_mut(et) {
                    ui.label(format!("Entity {:?}", entity));
                    ui.horizontal(|ui| {
                        let p = &mut transform.translation;
                        ui.label("position:");
                        let mut changed = false;
                        changed |= draw_and_edit_float(ui, &mut p.x);
                        changed |= draw_and_edit_float(ui, &mut p.y);
                        changed |= draw_and_edit_float(ui, &mut p.z);
                    });

                    ui.horizontal(|ui| {
                        ui.label("rotation:");
                        let mut angles = transform.rotation.to_euler(EulerRot::ZXY);
                        let mut changed = false;
                        changed |= draw_and_edit_float(ui, &mut angles.1);
                        changed |= draw_and_edit_float(ui, &mut angles.2);
                        changed |= draw_and_edit_float(ui, &mut angles.0);

                        if changed {
                            transform.rotation = Quat::from_euler(EulerRot::ZXY, angles.0, angles.1, angles.2);
                        }
                    });
                }
            }
        });
    }
}

