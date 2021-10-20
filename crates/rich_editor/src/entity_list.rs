use std::num::ParseFloatError;
use egui::{Align2, Color32, Direction, Painter, Shape, Ui};
use crate::EditorState;
use rich_engine::prelude::*;
use rich_engine::{Camera, DisplayName, RenderCamera, RenderRunner};
use crate::egui_integrate::egui::{Align, ScrollArea};
use crate::egui_integrate::EguiContext;
use crate::event::EditorEvent;

pub fn draw_entity_list(mut state: ResMut<EditorState>
                        , egui_context: Option<Res<EguiContext>>
                        , mut query: Query<(Entity, &Transform, Option<&DisplayName>)>) {
    if let Some(ctx) = &egui_context {
        egui::Window::new("EntityList").anchor(Align2::LEFT_BOTTOM, egui::Vec2::new(0.0, 0.0)).show(ctx.ctx(), |ui| {
            let mut scroll_area = ScrollArea::from_max_height(300.0);
            scroll_area.show(ui, |ui|
                {
                    ui.vertical(|ui|
                        {
                            ui.with_layout(egui::Layout::from_main_dir_and_cross_align(Direction::TopDown, Align::Max)
                                               .with_cross_justify(true),
                                           |ui| {
                                               for (entity, mut transform, od) in query.iter_mut() {
                                                   let name = match od {
                                                       Some(d) => { &d.name }
                                                       None => "",
                                                   };
                                                   if ui.button(format!("[{:?} {}]", entity, name).as_str()).clicked() {
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

fn to_egui_pos(vec2: Vec2) -> egui::Pos2 {
    egui::Pos2::from(vec2.to_array())
}

pub fn draw_entity_property(mut state: ResMut<EditorState>
                            , egui_context: Option<Res<EguiContext>>
                            , render_runner: Option<Res<RenderRunner>>
                            , render_camera: Res<RenderCamera>
                            , mut event_writer: EventWriter<EditorEvent>
                            , mut queries: QuerySet<(
        Query<(Entity, &mut Transform)>,
        Query<(&Camera, &Transform)>,
    )>,
) {
    if let Some(ctx) = &egui_context {
        if let Some(et) = state.current_select_entity {
            let mut select_transform: Option<Transform> = None;
            egui::Window::new("EntityProperty").anchor(Align2::RIGHT_BOTTOM, egui::Vec2::new(0.0, 0.0)).show(ctx.ctx(), |ui| {
                if let Ok((entity, mut transform)) = queries.q0_mut().get_mut(et) {
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

                    select_transform = Some(transform.clone());
                }

                if let Ok((camera, mut camera_transform)) = queries.q1().get(render_camera.camera) {
                    if let Some(transform) = select_transform {
                        let render_runner = render_runner.unwrap();
                        let window_size = Vec2::new(render_runner.context.window_width as _, render_runner.context.window_height as _);
                        let ui_pos = Camera::compute_world_2_ui_position(camera, camera_transform, transform.translation, window_size);
                        if ui_pos.z > 0f32 {
                            const DRAW_SIZE: f32 = 50f32;
                            let draw_rect = egui::Rect::from_min_size(to_egui_pos(ui_pos.xy() - Vec2::new(DRAW_SIZE, DRAW_SIZE)),
                                                                      egui::Vec2::new(2f32 * DRAW_SIZE, 2f32 * DRAW_SIZE));
                            let window_rect = egui::Rect::from_min_size(egui::Pos2::new(0.0, 0.0), egui::Vec2::new(window_size.x, window_size.y));

                            if window_rect.intersects(draw_rect) {
                                let painter = Painter::new(
                                    ui.ctx().clone(),
                                    ui.layer_id(),
                                    draw_rect,
                                );

                                let mut shapes: Vec<Shape> = Vec::new();
                                shapes.push(compute_line(camera, camera_transform, transform.translation + Vec3::Z * 0.1f32, window_size,
                                                         ui_pos, DRAW_SIZE, egui::Color32::BLUE));

                                shapes.push(compute_line(camera, camera_transform, transform.translation + Vec3::X * 10f32, window_size,
                                                         ui_pos, DRAW_SIZE, egui::Color32::GREEN));

                                shapes.push(compute_line(camera, camera_transform, transform.translation + Vec3::Y * 10f32, window_size,
                                                         ui_pos, DRAW_SIZE, egui::Color32::RED));
                                painter.extend(shapes);
                            }
                        }
                    }
                }
            });
        }
    }
}


fn compute_line(camera: &Camera, camera_transform: &Transform,
                target_world_pos: Vec3, window_size: Vec2, base_ui_pos: Vec3, length: f32, color: egui::Color32) -> egui::Shape {
    let base_ui_pos = base_ui_pos.xy();
    let t = Camera::compute_world_2_ui_position(camera, camera_transform, target_world_pos, window_size);
    let mut target = t.xy();
    target = base_ui_pos + (target - base_ui_pos).normalize() * length;
    let line = [to_egui_pos(base_ui_pos), to_egui_pos(target)];
    Shape::line_segment(line, (2f32, color))
}
