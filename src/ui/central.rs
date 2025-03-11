use eframe::egui;

use crate::app::AnnotationApp;
use crate::models::BoundingBox;

pub fn central_panel(app: &mut AnnotationApp, ctx: &egui::Context) {
    egui::CentralPanel::default().show(ctx, |ui| {
        if ui.input(|i| i.key_pressed(egui::Key::W) || i.key_pressed(egui::Key::A)) {
            app.switch_image(false, false);
        }
        if ui.input(|i| i.key_pressed(egui::Key::S) || i.key_pressed(egui::Key::D)) {
            app.switch_image(true, false);
        }
        if ui.input(|i| i.key_pressed(egui::Key::Space)) {
            app.switch_image(false, true);
        }
        if ui.input(|i| i.key_pressed(egui::Key::B)) {
            app.go_back();
        }
        if ui.input(|i| i.key_pressed(egui::Key::N)) {
            app.switch_to_next_unmodified();
        }
        if ui.input(|i| i.key_pressed(egui::Key::E)) {
            app.is_drawing = !app.is_drawing;
            app.drawing_start = None;
            app.selected_box = None;
            app.show_status(if app.is_drawing {
                "已进入绘制模式"
            } else {
                "已退出绘制模式"
            });
        }

        if let Some(image) = &app.current_image {
            let available_size = ui.available_size();
            let image_size = egui::vec2(image.width() as f32, image.height() as f32);

            let scale = (available_size.x / image_size.x).min(available_size.y / image_size.y);
            let displayed_size = image_size * scale;

            let texture: &egui::TextureHandle = app.texture.get_or_insert_with(|| {
                ui.ctx().load_texture(
                    "current_image",
                    egui::ColorImage::from_rgb(
                        [image.width() as _, image.height() as _],
                        image.to_rgb8().as_raw(),
                    ),
                    Default::default(),
                )
            });

            let response = ui
                .with_layout(
                    egui::Layout::centered_and_justified(egui::Direction::LeftToRight),
                    |ui| ui.image((texture.id(), displayed_size)),
                )
                .inner;

            let image_rect = response.rect;
            let offset_x = image_rect.min.x + (available_size.x - displayed_size.x) / 2.0;
            let offset_y = image_rect.min.y + (available_size.y - displayed_size.y) / 2.0;

            if app.is_drawing {
                // 监听数字键输入
                for key_num in 0..10 {
                    let key = egui::Key::from_name(&format!("{}", key_num)).unwrap();
                    if ui.input(|i| i.key_pressed(key)) {
                        app.selected_class = key_num as i32 - 1;
                        app.show_status(&format!("已切换到类别 {}", &app.selected_class));
                    }
                }

                if let Some(pointer) = ui.input(|i| i.pointer.hover_pos()) {
                    if ui.input(|i| i.pointer.primary_pressed()) {
                        app.drawing_start = Some(pointer);
                    }

                    if let Some(start) = app.drawing_start {
                        let rect = egui::Rect::from_two_pos(start, pointer);

                        let min_x =
                            ((rect.min.x - offset_x) / displayed_size.x).clamp(0.0, 1.0);
                        let min_y =
                            ((rect.min.y - offset_y) / displayed_size.y).clamp(0.0, 1.0);
                        let max_x =
                            ((rect.max.x - offset_x) / displayed_size.x).clamp(0.0, 1.0);
                        let max_y =
                            ((rect.max.y - offset_y) / displayed_size.y).clamp(0.0, 1.0);

                        ui.painter().rect_stroke(
                            rect,
                            0.0,
                            egui::Stroke::new(2.0, egui::Color32::YELLOW),
                            egui::StrokeKind::Middle,
                        );

                        if ui.input(|i| i.pointer.primary_released()) {
                            if min_x < max_x && min_y < max_y {
                                app.bounding_boxes.push(BoundingBox {
                                    class: app.selected_class,
                                    x: ((min_x + max_x) / 2.0) as f64,
                                    y: ((min_y + max_y) / 2.0) as f64,
                                    width: (max_x - min_x) as f64,
                                    height: (max_y - min_y) as f64,
                                });
                                app.save_annotations();
                                app.show_status("已添加新边界框");
                            }
                            app.drawing_start = None;
                        }
                    }
                }
            } else {
                if let Some(pointer) = ui.input(|i| i.pointer.hover_pos()) {
                    let mut hovered_box = None;

                    for (i, bbox) in app.bounding_boxes.iter().enumerate().rev() {
                        let box_width = bbox.width as f32 * displayed_size.x;
                        let box_height = bbox.height as f32 * displayed_size.y;
                        let center_x = offset_x + (bbox.x as f32 * displayed_size.x);
                        let center_y = offset_y + (bbox.y as f32 * displayed_size.y);

                        let rect = egui::Rect::from_min_size(
                            egui::pos2(center_x - box_width / 2.0, center_y - box_height / 2.0),
                            egui::vec2(box_width, box_height),
                        );

                        if rect.contains(pointer) {
                            hovered_box = Some(i);
                            break;
                        }
                    }

                    if ui.input(|i| i.pointer.primary_clicked()) {
                        if app.selected_box != hovered_box {
                            app.selected_box = hovered_box;
                            if let Some(idx) = hovered_box {
                                app.show_status(&format!("已选中边界框 {}", idx));
                            } else {
                                app.show_status("取消选中");
                            }
                        }
                    }

                    if let Some(selected_idx) = app.selected_box {
                        if ui.input(|i| i.pointer.primary_down()) {
                            let delta = ui.input(|i| i.pointer.delta());
                            let dx = (delta.x as f64) / (displayed_size.x as f64);
                            let dy = (delta.y as f64) / (displayed_size.y as f64);

                            if let Some(bbox) = app.bounding_boxes.get_mut(selected_idx) {
                                // 确保边界框不会超出图像范围
                                let new_x = (bbox.x + dx)
                                    .clamp(bbox.width / 2.0, 1.0 - bbox.width / 2.0);
                                let new_y = (bbox.y + dy)
                                    .clamp(bbox.height / 2.0, 1.0 - bbox.height / 2.0);
                                // 进行舍入处理
                                bbox.x = new_x;
                                bbox.y = new_y;
                            }
                        }

                        if ui.input(|i| i.pointer.primary_released()) {
                            app.save_annotations();
                            app.show_status("已保存边界框位置");
                        }
                    }
                }
            }

            if ui.input(|i| i.pointer.secondary_clicked()) {
                app.is_drawing = false;
                app.drawing_start = None;
                app.selected_box = None;
                app.show_status("已退出绘制模式");
            }

            for (i, bbox) in app.bounding_boxes.iter().enumerate() {
                let box_width = bbox.width as f32 * displayed_size.x;
                let box_height = bbox.height as f32 * displayed_size.y;
                let center_x = offset_x + (bbox.x as f32 * displayed_size.x);
                let center_y = offset_y + (bbox.y as f32 * displayed_size.y);

                let rect = egui::Rect::from_min_size(
                    egui::pos2(center_x - box_width / 2.0, center_y - box_height / 2.0),
                    egui::vec2(box_width, box_height),
                );

                let box_color = if Some(i) == app.selected_box {
                    egui::Color32::GREEN
                } else {
                    egui::Color32::RED
                };

                ui.painter().rect_stroke(
                    rect,
                    0.0,
                    egui::Stroke::new(2.0, box_color),
                    egui::StrokeKind::Middle,
                );

                // 根据设置显示或隐藏标签
                if app.show_labels {
                    ui.painter().text(
                        rect.min,
                        egui::Align2::LEFT_TOP,
                        format!("Class {}", bbox.class),
                        egui::FontId::default(),
                        box_color,
                    );
                }
                
                // 根据设置显示或隐藏中心点
                if app.show_center_points {
                    // 绘制中心点
                    let center_point_size = 5.0;
                    
                    // 绘制中心点（实心圆）
                    ui.painter().circle_filled(
                        egui::pos2(center_x, center_y),
                        center_point_size / 2.0,
                        box_color
                    );
                    
                    // 绘制十字线
                    let line_length = 10.0;
                    ui.painter().line_segment(
                        [
                            egui::pos2(center_x - line_length, center_y),
                            egui::pos2(center_x + line_length, center_y),
                        ],
                        egui::Stroke::new(1.0, box_color),
                    );
                    ui.painter().line_segment(
                        [
                            egui::pos2(center_x, center_y - line_length),
                            egui::pos2(center_x, center_y + line_length),
                        ],
                        egui::Stroke::new(1.0, box_color),
                    );
                }
            }
        }

        if ui.input(|i| i.key_pressed(egui::Key::Q)) {
            app.save_annotations();
            app.save_modified_records();
        }

        if ui.input(|i| i.key_pressed(egui::Key::Delete)) {
            if let Some(selected) = app.selected_box {
                app.bounding_boxes.remove(selected);
                app.selected_box = None;
                app.save_annotations();
                app.update_total_statistics();
                app.show_status("已删除选中的边界框");
            }
        }

        if ui.input(|i| i.key_pressed(egui::Key::Escape)) {
            app.is_drawing = false;
            app.drawing_start = None;
            app.selected_box = None;
            app.show_status("已退出绘制模式");
        }
    });
}