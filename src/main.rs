#![windows_subsystem = "windows"]

mod app;
mod models;
mod ui;
mod utils;

use app::AnnotationApp;
use ctrlc;
use eframe::egui;
use std::panic;
use std::sync::{Arc, Mutex};

fn main() {
    let app = Arc::new(Mutex::new(None::<AnnotationApp>));
    let app_clone1 = app.clone();
    let app_clone2 = app.clone();
    let app_clone3 = app.clone();

    // panic hook
    let old_hook = panic::take_hook();
    panic::set_hook(Box::new(move |panic_info| {
        if let Some(app) = app_clone1.lock().unwrap().as_mut() {
            app.on_exit();
        }
        old_hook(panic_info);
    }));

    // Ctrl+C handler
    ctrlc::set_handler(move || {
        if let Some(app) = app_clone2.lock().unwrap().as_mut() {
            app.on_exit();
        }
        std::process::exit(0);
    })
    .expect("Error setting Ctrl-C handler");

    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1280.0, 720.0])
            .with_title("数据标注平台"),
        ..Default::default()
    };

    if let Err(e) = eframe::run_native(
        "数据标注平台",
        options,
        Box::new(|cc| {
            // 配置中文字体
            let mut fonts = egui::FontDefinitions::default();

            fonts.font_data.insert(
                "simhei".to_owned(),
                egui::FontData::from_static(include_bytes!("../SimHei.ttf")).into(),
            );

            fonts
                .families
                .get_mut(&egui::FontFamily::Proportional)
                .unwrap()
                .insert(0, "simhei".to_owned());

            fonts
                .families
                .get_mut(&egui::FontFamily::Monospace)
                .unwrap()
                .push("simhei".to_owned());

            cc.egui_ctx.set_fonts(fonts);

            let app = AnnotationApp::default();
            *app_clone3.lock().unwrap() = Some(app.clone());
            Ok(Box::new(MyApp { app }))
        }),
    ) {
        eprintln!("Error running native application: {}", e);
    }
}

struct MyApp {
    app: AnnotationApp,
}

impl eframe::App for MyApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        ui::top::top_panel(&mut self.app, ctx);
        ui::side::side_panel(&mut self.app, ctx);
        ui::statistics::statistics_panel(&mut self.app, ctx);
        ui::central::central_panel(&mut self.app, ctx);
    }
    
    fn on_exit(&mut self, _gl: Option<&eframe::glow::Context>) {
        self.app.on_exit();
    }
}