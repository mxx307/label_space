use core::f32;

use eframe::egui;

use crate::app::AnnotationApp;

pub fn statistics_panel(app: &mut AnnotationApp, ctx: &egui::Context) {
    egui::SidePanel::right("statistics_panel")
        .default_width(200.0)
        .show(ctx, |ui| {
            ui.heading("统计信息");
            ui.label(format!("总图片数: {}", app.statistics.total_images));
            ui.label(format!("已标注图片: {}", app.statistics.modified_images));
            ui.label(format!(
                "完成进度: {:.1}%",
                (app.statistics.modified_images as f32 / app.statistics.total_images as f32 * 100.0)
                    .max(0.0)
            ));

            ui.separator();
            ui.heading("所有图片标注统计");
            // 获取并排序类别
            let mut classes: Vec<_> = app.statistics.total_class_counts.keys().collect();
            classes.sort();
            // 按排序后的类别显示统计
            for class in classes {
                if let Some(count) = app.statistics.total_class_counts.get(class) {
                    ui.label(format!("类别 {}: {} 个", class, count));
                }
            }

            if !app.bounding_boxes.is_empty() {
                ui.separator();
                ui.heading("当前图片标注统计");
                // 获取并排序当前图片的类别
                let mut classes: Vec<_> = app.statistics.current_class_counts.keys().collect();
                classes.sort();
                // 按排序后的类别显示统计
                for class in classes {
                    if let Some(count) = app.statistics.current_class_counts.get(class) {
                        ui.label(format!("类别 {}: {} 个", class, count));
                    }
                }
            }

            ui.separator();
            ui.heading("导出功能");
            
            if ui.button("导出已修改的文件").clicked() {
                if app.modified_images.is_empty() {
                    app.show_status("没有已修改的文件可导出");
                } else if app.image_dir.is_none() || app.label_dir.is_none() {
                    app.show_status("请先选择图片和标签目录");
                } else {
                    if let Some(export_dir) = rfd::FileDialog::new().pick_folder() {
                        // 将导出结果保存到临时变量，用于在弹窗中显示
                        let export_result = app.export_modified_files(export_dir.clone());
                        let export_dir_display = export_dir.display().to_string();
                        let modified_count = app.modified_images.len();
                        
                        // 设置弹窗标志
                        app.show_export_result_dialog = true;
                        app.export_result_info = match export_result {
                            Ok(count) => {
                                format!("实际已修改数量 {}, 成功导出 {} 个文件到 {}", 
                                    modified_count, count, export_dir_display)
                            },
                            Err(e) => {
                                format!("导出失败: {}", e)
                            }
                        };
                    }
                }
            }
            
            // 显示导出结果弹窗
            if app.show_export_result_dialog {
                let screen_size = ui.ctx().screen_rect().size();
                let window_size = egui::vec2(350.0, 100.0);
                let pos = egui::pos2(
                    (screen_size.x - window_size.x) / 2.0,
                    (screen_size.y - window_size.y) / 2.0,
                );
                
                egui::Window::new("导出结果")
                    .collapsible(false)
                    .resizable(false)
                    .fixed_pos(pos)
                    .show(ui.ctx(), |ui| {
                        ui.label(&app.export_result_info);
                        ui.horizontal(|ui| {
                            if ui.button("确定").clicked() {
                                app.show_export_result_dialog = false;
                            }
                        });
                    });
            }


            ui.separator();
            ui.heading("操作");
            if ui.button("删除当前图片及标签").clicked() {
                app.show_delete_confirmation = true; // 点击删除按钮时显示确认对话框
            }
            
            // 显示确认对话框
            if app.show_delete_confirmation {
                let screen_size = ui.ctx().screen_rect().size();
                let window_size = egui::vec2(250.0, 100.0); // 假设窗口大小
                let pos = egui::pos2(
                    (screen_size.x - window_size.x) / 2.0,
                    (screen_size.y - window_size.y) / 2.0,
                );
                egui::Window::new("确认删除")
                    .collapsible(false)
                    .resizable(false)
                    .fixed_pos(pos)
                    .show(ui.ctx(), |ui| {
                        ui.label("你确定要删除当前图片及标签吗？");
                        ui.horizontal(|ui| {
                            if ui.button("确定").clicked() {
                                if let (Some(image_path), Some(label_dir)) =
                                    (&app.current_image_path, &app.label_dir)
                                {
                                    // 获取标签文件路径
                                    let label_path = label_dir
                                        .join(image_path.file_stem().unwrap())
                                        .with_extension("txt");

                                    // 删除标签文件
                                    if label_path.exists() {
                                        if let Err(e) = std::fs::remove_file(&label_path) {
                                            app.show_status(&format!("删除标签文件失败: {}", e));
                                            return;
                                        }
                                    }

                                    // 删除图片文件
                                    if let Err(e) = std::fs::remove_file(image_path) {
                                        app.show_status(&format!("删除图片文件失败: {}", e));
                                        return;
                                    }

                                    // 从缓存中移除
                                    if let Some(name) = &app.current_image_name {
                                        app.modified_images.remove(name);
                                    }
                                    app.image_cache.remove(image_path);

                                    // 更新文件列表并切换到下一张图片
                                    app.update_file_list();
                                    app.update_total_statistics();
                                    if let Some(prev_path) = app.history.pop() {
                                        app.load_image(&prev_path);
                                    } else if let Some(next_path) =
                                        app.cached_image_files.first().cloned()
                                    {
                                        app.load_image(&next_path);
                                    }
                                    app.show_status("已删除当前图片及标签");
                                }
                                app.show_delete_confirmation = false; // 关闭确认对话框
                            }
                            if ui.button("取消").clicked() {
                                app.show_delete_confirmation = false; // 关闭确认对话框
                            }
                        });
                    });
            }

            ui.separator();
            ui.heading("操作模式");
            ui.horizontal(|ui| {
                if ui
                    .button(if app.is_drawing {
                        "退出绘制"
                    } else {
                        "进入绘制"
                    })
                    .clicked()
                {
                    app.is_drawing = !app.is_drawing;
                    app.drawing_start = None;
                    app.selected_box = None;
                    app.show_status(if app.is_drawing {
                        "已进入绘制模式"
                    } else {
                        "已退出绘制模式"
                    });
                }
            });

            // 添加显示控制按钮
            ui.separator();
            ui.heading("显示设置");

            // 标签显示控制
            if ui.button(if app.show_labels {
                "隐藏标签"
            } else {
                "显示标签"
            }).clicked() {
                app.show_labels = !app.show_labels;
                app.show_status(if app.show_labels {
                    "已显示标签"
                } else {
                    "已隐藏标签"
                });
            }

            // 中心点显示控制
            if ui.button(if app.show_center_points {
                "隐藏中心点"
            } else {
                "显示中心点"
            }).clicked() {
                app.show_center_points = !app.show_center_points;
                app.show_status(if app.show_center_points {
                    "已显示中心点"
                } else {
                    "已隐藏中心点"
                });
            }
            if app.is_drawing {
                ui.heading("添加边界框");
                // 添加类型选择按钮
                ui.horizontal_wrapped(|ui| {
                    for class in 0..10 {
                        let text = if class == app.selected_class {
                            format!("【类别 {}】", class)
                        } else {
                            format!("类别 {}", class)
                        };
                        if ui.button(text).clicked() {
                            app.selected_class = class;
                        }
                    }
                });
            }

            // 添加弹性空间，将状态消息推到底部
            ui.add_space(ui.available_height() - 30.0);

            // 在底部显示状态消息
            if let Some((message, time_left)) = &app.status_message {
                ui.horizontal(|ui| {
                    ui.label(message);
                });
                let time_left = &mut time_left.clone();
                *time_left -= ctx.input(|i| i.predicted_dt);
                if *time_left <= 0.0 {
                    app.status_message = None;
                }
            }
        });
}