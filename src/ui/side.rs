use eframe::egui;
use std::fs;

use crate::app::AnnotationApp;

pub fn side_panel(app: &mut AnnotationApp, ctx: &egui::Context) {
    egui::SidePanel::left("side_panel").show(ctx, |ui| {
        if let Some(image_dir) = &app.image_dir {
            let entries = fs::read_dir(image_dir)
                .unwrap_or_else(|_| return std::fs::read_dir(".").unwrap());
            let mut image_files: Vec<_> = entries
                .filter_map(|entry| entry.ok())
                .filter(|entry| {
                    entry
                        .path()
                        .extension()
                        .map_or(false, |ext| ext == "jpg" || ext == "png")
                })
                .collect();

            image_files.sort_by_key(|entry| entry.path());

            let scroll_area = egui::ScrollArea::vertical().auto_shrink([false; 2]);

            scroll_area.show(ui, |ui| {
                for entry in image_files {
                    let path = entry.path();
                    let file_name = path
                        .file_name()
                        .unwrap_or_default()
                        .to_string_lossy()
                        .to_string();

                    let is_selected = app
                        .current_image_name
                        .as_ref()
                        .map_or(false, |current| current == &file_name);

                    let is_modified = app.modified_images.contains(&file_name);

                    let button = egui::Button::new(egui::RichText::new(&file_name).color(
                        if is_selected {
                            egui::Color32::YELLOW
                        } else if is_modified {
                            egui::Color32::from_rgb(0, 100, 0)
                        } else {
                            egui::Color32::BLACK
                        },
                    ))
                    .fill(if is_selected {
                        egui::Color32::DARK_BLUE
                    } else {
                        egui::Color32::from_gray(230)
                    });

                    let response = ui.add(button);
                    if response.clicked() {
                        app.load_image(&path);
                    }

                    if is_selected && app.scroll_to_current {
                        response.scroll_to_me(Some(egui::Align::Center));
                        app.scroll_to_current = false; // 重置滚动标记
                    }
                }
            });
        }
    });
}