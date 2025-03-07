use eframe::egui;

use crate::app::AnnotationApp;

pub fn top_panel(app: &mut AnnotationApp, ctx: &egui::Context) {
    egui::TopBottomPanel::top("top_panel").show(ctx, |ui| {
        ui.horizontal(|ui| {
            if ui.button("选择图片文件夹").clicked() {
                app.select_image_dir();
            }
            if ui.button("选择标签文件夹").clicked() {
                app.select_label_dir();
            }

            if let Some(image_dir) = &app.image_dir {
                ui.label(format!("图片目录: {}", image_dir.display()));
            }
            if let Some(label_dir) = &app.label_dir {
                ui.label(format!("标签目录: {}", label_dir.display()));
            }
        });
    });
}