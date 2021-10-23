use std::num::ParseFloatError;
use egui::{Align2, Color32, Direction, Painter, Shape, Ui};
use egui::WidgetType::Button;
use crate::EditorState;
use rich_engine::prelude::*;
use rich_engine::{Camera, DisplayName, RenderCamera, RenderRunner};
use crate::egui_integrate::egui::{Align, ScrollArea};
use crate::egui_integrate::EguiContext;
use crate::event::EditorEvent;

pub fn draw_entity_list(mut state: ResMut<EditorState>
                        , egui_context: Option<Res<EguiContext>>
                        , keyboard_input: Res<Input<KeyCode>>
                        , mut editor_event_writer: EventWriter<EditorEvent>
                        , mut camera_op_event_writer: EventWriter<CameraOpEvent>
                        , mut query: Query<(Entity, &Transform, Option<&DisplayName>)>) {
    if let Some(ctx) = &egui_context {
        egui::Window::new("EntityList").anchor(Align2::LEFT_BOTTOM, egui::Vec2::new(0.0, 0.0)).show(ctx.ctx(), |ui| {
            let mut scroll_area = ScrollArea::from_max_height(300.0);
            scroll_area.show(ui, |ui|
                {
                    ui.vertical(|ui|
                        {
                            ui.with_layout(egui::Layout::from_main_dir_and_cross_align(Direction::TopDown, Align::Max)
                                               .with_cross_justify(true), |ui| {
                                for (entity, mut transform, od) in query.iter_mut() {
                                    let name = match od {
                                        Some(d) => { &d.name }
                                        None => "",
                                    };
                                    let mut bc: Option<Color32> = None;
                                    if state.current_select_entity == Some(entity) {
                                        bc = Some(Color32::GREEN);

                                        if keyboard_input.pressed(KeyCode::F) && keyboard_input.pressed(KeyCode::LShift) {
                                            camera_op_event_writer.send(CameraOpEvent::Focus(*transform, 5f32));
                                        }

                                        if keyboard_input.pressed(KeyCode::Delete) {
                                            editor_event_writer.send(EditorEvent::DeleteEntity(entity));
                                            state.current_select_entity = None;
                                        }
                                    }
                                    if ui.add(egui::Button::new(format!("[{:?} {}]", entity, name).as_str()).text_color_opt(bc)).clicked() {
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
    ui.add(egui::DragValue::new(data).speed(0.1f32)).changed()
}

fn to_egui_pos(vec2: Vec2) -> egui::Pos2 {
    egui::Pos2::from(vec2.to_array())
}

pub fn draw_entity_property(mut state: ResMut<EditorState>
                            , egui_context: Option<Res<EguiContext>>
                            , render_runner: Option<Res<RenderRunner>>
                            , render_camera: Res<RenderCamera>
                            , mut editor_event_writer: EventWriter<EditorEvent>
                            , mut camera_op_event_writer: EventWriter<CameraOpEvent>
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

                        if changed {
                            if render_camera.camera == entity {
                                camera_op_event_writer.send(CameraOpEvent::ChangeTranslation(*p));
                            }
                        }
                    });

                    ui.horizontal(|ui| {
                        ui.label("rotation:");

                        let deg = 1f32.to_degrees();
                        let mut angles = Vec3::from(transform.rotation.to_euler(EulerRot::ZXY)) * deg;

                        let mut changed = false;
                        changed |= draw_and_edit_float(ui, &mut angles.y);
                        changed |= draw_and_edit_float(ui, &mut angles.z);
                        changed |= draw_and_edit_float(ui, &mut angles.x);

                        if changed {
                            angles = angles * 1f32.to_radians();
                            transform.rotation = Quat::from_euler(EulerRot::ZXY, angles.x, angles.y, angles.z);
                            if changed {
                                if render_camera.camera == entity {
                                    camera_op_event_writer.send(CameraOpEvent::ChangeRotation(transform.rotation));
                                }
                            }
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

                                shapes.push(compute_line(camera, camera_transform, rotate_translation(&transform, Vec3::Z * 0.1f32),
                                                         window_size, ui_pos, DRAW_SIZE, egui::Color32::BLUE));

                                shapes.push(compute_line(camera, camera_transform, rotate_translation(&transform, Vec3::X * 0.1f32),
                                                         window_size, ui_pos, DRAW_SIZE, egui::Color32::GREEN));

                                shapes.push(compute_line(camera, camera_transform, rotate_translation(&transform, Vec3::Y * 0.1f32),
                                                         window_size, ui_pos, DRAW_SIZE, egui::Color32::RED));
                                painter.extend(shapes);
                            }
                        }
                    }
                }
            });
        }
    }
}

fn rotate_translation(transform: &Transform, delta: Vec3) -> Vec3 {
    transform.translation + transform.rotation.mul_vec3(delta)
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
