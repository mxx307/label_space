#![windows_subsystem = "windows"]

use ctrlc;
use eframe::egui;
use image::DynamicImage;
use rand::seq::IndexedRandom;
use std::collections::HashMap;
use std::fs::{self, File};
use std::io::{BufRead, BufReader, Write};
use std::panic;
use std::path::PathBuf;

#[derive(Clone)]
struct BoundingBox {
    class: i32,
    x: f64,
    y: f64,
    width: f64,
    height: f64,
}

#[derive(Clone)]
struct Statistics {
    total_images: usize,
    modified_images: usize,
    total_class_counts: HashMap<i32, usize>, // 所有图片中各类型的数量
    current_class_counts: HashMap<i32, usize>, // 当前图片中各类型的数量
}

impl Default for Statistics {
    fn default() -> Self {
        Self {
            total_images: 0,
            modified_images: 0,
            total_class_counts: HashMap::new(),
            current_class_counts: HashMap::new(),
        }
    }
}

#[derive(Clone)]
// 在AnnotationApp结构体中添加字段
struct AnnotationApp {
    image_dir: Option<PathBuf>,
    label_dir: Option<PathBuf>,
    current_image: Option<DynamicImage>,
    current_image_path: Option<PathBuf>,
    bounding_boxes: Vec<BoundingBox>,
    selected_box: Option<usize>,
    texture: Option<egui::TextureHandle>,
    current_image_name: Option<String>,
    modified_images: std::collections::HashSet<String>,
    cached_image_files: Vec<PathBuf>,
    status_message: Option<(String, f32)>,
    image_cache: HashMap<PathBuf, DynamicImage>,
    max_cache_size: usize,
    statistics: Statistics,
    selected_class: i32,
    is_drawing: bool,
    drawing_start: Option<egui::Pos2>,
    scroll_to_current: bool,
    history: Vec<PathBuf>, // 记录浏览历史
    show_delete_confirmation: bool,
}

impl Default for AnnotationApp {
    fn default() -> Self {
        let mut app = Self {
            image_dir: None,
            label_dir: None,
            current_image: None,
            current_image_path: None,
            bounding_boxes: Vec::new(),
            selected_box: None,
            texture: None,
            current_image_name: None,
            modified_images: std::collections::HashSet::new(),
            cached_image_files: Vec::new(),
            status_message: None,
            image_cache: HashMap::new(),
            max_cache_size: 5,
            statistics: Statistics::default(),
            selected_class: 0,
            is_drawing: false,
            drawing_start: None,
            scroll_to_current: false,
            history: vec![],
            show_delete_confirmation: false,
        };
        app.load_modified_records();
        app
    }
}

impl AnnotationApp {
    fn show_status(&mut self, message: &str) {
        self.status_message = Some((message.to_string(), 2.0));
    }

    fn load_image(&mut self, path: &PathBuf) {
        if let Some(current_path) = &self.current_image_path {
            self.history.push(current_path.clone());
        }

        if let Some(img) = self.image_cache.get(path) {
            self.current_image = Some(img.clone());
            self.current_image_path = Some(path.clone());
            self.texture = None;
            self.current_image_name = path
                .file_name()
                .and_then(|n| n.to_str())
                .map(|s| s.to_string());
            self.load_annotations();
            return;
        }

        if let Ok(img) = image::open(path) {
            let img = Self::resize_to_limit(&img, 1920, 1080);
            self.current_image = Some(img.clone());
            self.current_image_path = Some(path.clone());
            self.texture = None;
            self.current_image_name = path
                .file_name()
                .and_then(|n| n.to_str())
                .map(|s| s.to_string());

            self.update_image_cache(path.clone(), img);
            self.load_annotations();
        } else {
            self.show_status("图片加载失败");
        }
    }

    fn load_annotations(&mut self) {
        if let Some(image_path) = &self.current_image_path {
            if let Some(label_dir) = &self.label_dir {
                let label_path = label_dir
                    .join(image_path.file_stem().unwrap())
                    .with_extension("txt");

                self.bounding_boxes.clear();
                if let Ok(file) = File::open(label_path) {
                    let reader = BufReader::new(file);
                    for line in reader.lines() {
                        if let Ok(line) = line {
                            let parts: Vec<f64> = line
                                .split_whitespace()
                                .map(|s| s.parse().unwrap_or(0.0))
                                .collect();

                            if parts.len() == 5 {
                                self.bounding_boxes.push(BoundingBox {
                                    class: parts[0] as i32,
                                    x: parts[1],
                                    y: parts[2],
                                    width: parts[3],
                                    height: parts[4],
                                });
                            }
                        }
                    }
                }
            }
        }
        self.update_statistics();
    }

    fn save_annotations(&mut self) {
        if let Some(image_path) = &self.current_image_path {
            if let Some(label_dir) = &self.label_dir {
                let label_path = label_dir
                    .join(image_path.file_stem().unwrap())
                    .with_extension("txt");

                if let Ok(mut file) = File::create(label_path) {
                    for bbox in &self.bounding_boxes {
                        writeln!(
                            file,
                            "{} {} {} {} {}",
                            bbox.class, bbox.x, bbox.y, bbox.width, bbox.height
                        )
                        .ok();
                    }
                    if let Some(name) = &self.current_image_name {
                        self.modified_images.insert(name.clone());
                    }
                }
            }
        }
        self.update_statistics();
    }

    fn update_file_list(&mut self) {
        if let Some(image_dir) = &self.image_dir {
            if let Ok(entries) = fs::read_dir(image_dir) {
                self.cached_image_files = entries
                    .filter_map(|entry| entry.ok())
                    .filter(|entry| {
                        entry
                            .path()
                            .extension()
                            .map_or(false, |ext| ext == "jpg" || ext == "png")
                    })
                    .map(|e| e.path())
                    .collect();
                self.cached_image_files.sort();
            }
        }
        self.update_statistics();
    }

    fn update_image_cache(&mut self, path: PathBuf, img: DynamicImage) {
        self.image_cache.insert(path.clone(), img);

        if let Some(current_pos) = self.cached_image_files.iter().position(|p| p == &path) {
            if current_pos + 1 < self.cached_image_files.len() {
                let next_path = &self.cached_image_files[current_pos + 1];
                if !self.image_cache.contains_key(next_path) {
                    if let Ok(img) = image::open(next_path) {
                        let img = Self::resize_to_limit(&img, 1920, 1080);
                        self.image_cache.insert(next_path.clone(), img);
                    }
                }
            }

            if current_pos > 0 {
                let prev_path = &self.cached_image_files[current_pos - 1];
                if !self.image_cache.contains_key(prev_path) {
                    if let Ok(img) = image::open(prev_path) {
                        let img = Self::resize_to_limit(&img, 1920, 1080);
                        self.image_cache.insert(prev_path.clone(), img);
                    }
                }
            }
        }

        while self.image_cache.len() > self.max_cache_size {
            if let Some(current_pos) = self
                .current_image_path
                .as_ref()
                .and_then(|p| self.cached_image_files.iter().position(|fp| fp == p))
            {
                let mut furthest_path = None;
                let mut max_distance = 0;

                for cached_path in self.image_cache.keys() {
                    if let Some(pos) = self
                        .cached_image_files
                        .iter()
                        .position(|p| p == cached_path)
                    {
                        let distance = (pos as i32 - current_pos as i32).abs();
                        if distance > max_distance {
                            max_distance = distance;
                            furthest_path = Some(cached_path.clone());
                        }
                    }
                }

                if let Some(path_to_remove) = furthest_path {
                    self.image_cache.remove(&path_to_remove);
                }
            }
        }
    }

    fn resize_to_limit(img: &DynamicImage, max_width: u32, max_height: u32) -> DynamicImage {
        let width = img.width();
        let height = img.height();

        if width <= max_width && height <= max_height {
            return img.clone();
        }

        let ratio = (max_width as f32 / width as f32).min(max_height as f32 / height as f32);

        let new_width = (width as f32 * ratio) as u32;
        let new_height = (height as f32 * ratio) as u32;

        img.resize(new_width, new_height, image::imageops::FilterType::Triangle)
    }

    // 修改switch_image函数
    fn switch_image(&mut self, next: bool, random_unmodified: bool) {
        if self.cached_image_files.is_empty() {
            self.update_file_list();
        }

        let mut target_path: Option<PathBuf> = None;

        if random_unmodified {
            let unmodified_files: Vec<PathBuf> = self
                .cached_image_files
                .iter()
                .filter(|path| {
                    let file_name = path
                        .file_name()
                        .and_then(|n| n.to_str())
                        .map(|s| s.to_string());
                    file_name
                        .as_ref()
                        .map_or(false, |name| !self.modified_images.contains(name))
                })
                .cloned()
                .collect();

            if let Some(random_path) = unmodified_files.choose(&mut rand::rng()) {
                target_path = Some(random_path.clone());
                self.load_image(random_path);
                // 添加滚动标记
                self.scroll_to_current = true;
            }
        } else {
            if let Some(current_path) = &self.current_image_path {
                if let Some(current_pos) = self
                    .cached_image_files
                    .iter()
                    .position(|p| p == current_path)
                {
                    let new_pos = if next {
                        if current_pos + 1 < self.cached_image_files.len() {
                            current_pos + 1
                        } else {
                            0
                        }
                    } else {
                        if current_pos > 0 {
                            current_pos - 1
                        } else {
                            self.cached_image_files.len() - 1
                        }
                    };
                    let path = self.cached_image_files[new_pos].clone();
                    target_path = Some(path.clone());
                    self.load_image(&path);
                    self.scroll_to_current = true;
                }
            } else if !self.cached_image_files.is_empty() {
                let path = self.cached_image_files[0].clone();
                target_path = Some(path.clone());
                self.load_image(&path);
            }
        }

        // 记录目标图片路径，用于后续滚动到该图片
        if let Some(path) = target_path {
            self.current_image_path = Some(path.clone());
            self.current_image_name = path
                .file_name()
                .and_then(|n| n.to_str())
                .map(|s| s.to_string());
        }
    }

    fn switch_to_next_unmodified(&mut self) {
        if self.cached_image_files.is_empty() {
            self.update_file_list();
        }

        let start_pos = if let Some(current_path) = &self.current_image_path {
            self.cached_image_files
                .iter()
                .position(|p| p == current_path)
                .map(|pos| pos + 1)
                .unwrap_or(0)
        } else {
            0
        };

        // 从当前位置开始查找下一个未修改的图片
        for i in start_pos..self.cached_image_files.len() {
            let path = self.cached_image_files[i].clone();
            if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
                if !self.modified_images.contains(name) {
                    self.load_image(&path);
                    self.scroll_to_current = true;
                    return;
                }
            }
        }

        // 如果从当前位置到末尾没有找到，从头开始找到当前位置
        for i in 0..start_pos {
            let path = self.cached_image_files[i].clone();
            if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
                if !self.modified_images.contains(name) {
                    self.load_image(&path);
                    self.scroll_to_current = true;
                    return;
                }
            }
        }

        self.show_status("没有找到未修改的图片");
    }

    // 添加返回上一张图片的函数
    fn go_back(&mut self) {
        if let Some(prev_path) = self.history.pop() {
            self.load_image(&prev_path);
            self.scroll_to_current = true;
        }
    }

    fn load_modified_records(&mut self) {
        if let Some(label_dir) = &self.label_dir {
            let record_path = label_dir.join("modified_records.txt");
            if let Ok(file) = File::open(record_path) {
                let reader = BufReader::new(file);
                for line in reader.lines() {
                    if let Ok(filename) = line {
                        self.modified_images.insert(filename);
                    }
                }
            }
        }
    }

    fn save_modified_records(&self) {
        if let Some(label_dir) = &self.label_dir {
            let record_path = label_dir.join("modified_records.txt");
            if let Ok(mut file) = File::create(record_path) {
                for filename in &self.modified_images {
                    writeln!(file, "{}", filename).ok();
                }
            }
        }
    }

    fn select_image_dir(&mut self) {
        if let Some(path) = rfd::FileDialog::new().pick_folder() {
            self.image_dir = Some(path);
            self.update_file_list();

            if !self.cached_image_files.is_empty() {
                let path = self.cached_image_files[0].clone();
                self.load_image(&path);
            }
        }
    }

    fn select_label_dir(&mut self) {
        if let Some(path) = rfd::FileDialog::new().pick_folder() {
            self.label_dir = Some(path);
            self.load_modified_records();
            self.update_total_statistics();
            self.show_status("已加载标签目录");
        }
    }

    fn on_exit(&mut self) {
        self.save_modified_records();
    }

    fn update_statistics(&mut self) {
        let mut stats = Statistics::default();

        // 统计总图片数
        stats.total_images = self.cached_image_files.len();
        stats.modified_images = self.modified_images.len();

        // 当前图片中的标注类型数量
        for bbox in &self.bounding_boxes {
            *stats.current_class_counts.entry(bbox.class).or_insert(0) += 1;
        }

        // 保持总体统计不变
        stats.total_class_counts = self.statistics.total_class_counts.clone();

        self.statistics = stats;
    }

    fn update_total_statistics(&mut self) {
        let mut stats = self.statistics.clone();
        stats.total_class_counts.clear();

        // 统计所有图片中的标注类型数量
        if let Some(label_dir) = &self.label_dir {
            for image_path in &self.cached_image_files {
                let label_path = label_dir
                    .join(image_path.file_stem().unwrap())
                    .with_extension("txt");

                if let Ok(file) = File::open(label_path) {
                    let reader = BufReader::new(file);
                    for line in reader.lines() {
                        if let Ok(line) = line {
                            let parts: Vec<f32> = line
                                .split_whitespace()
                                .map(|s| s.parse().unwrap_or(0.0))
                                .collect();

                            if parts.len() >= 1 {
                                let class = parts[0] as i32;
                                *stats.total_class_counts.entry(class).or_insert(0) += 1;
                            }
                        }
                    }
                }
            }
        }

        self.statistics = stats;
    }

    fn show_statistics(&mut self, ui: &mut egui::Ui) {
        ui.heading("统计信息");
        ui.label(format!("总图片数: {}", self.statistics.total_images));
        ui.label(format!("已标注图片: {}", self.statistics.modified_images));
        ui.label(format!(
            "完成进度: {:.1}%",
            (self.statistics.modified_images as f32 / self.statistics.total_images as f32 * 100.0)
                .max(0.0)
        ));

        ui.separator();
        ui.heading("所有图片标注统计");
        // 获取并排序类别
        let mut classes: Vec<_> = self.statistics.total_class_counts.keys().collect();
        classes.sort();
        // 按排序后的类别显示统计
        for class in classes {
            if let Some(count) = self.statistics.total_class_counts.get(class) {
                ui.label(format!("类别 {}: {} 个", class, count));
            }
        }

        if !self.bounding_boxes.is_empty() {
            ui.separator();
            ui.heading("当前图片标注统计");
            // 获取并排序当前图片的类别
            let mut classes: Vec<_> = self.statistics.current_class_counts.keys().collect();
            classes.sort();
            // 按排序后的类别显示统计
            for class in classes {
                if let Some(count) = self.statistics.current_class_counts.get(class) {
                    ui.label(format!("类别 {}: {} 个", class, count));
                }
            }
        }

        ui.separator();
        ui.heading("操作");
        if ui.button("删除当前图片及标签").clicked() {
            self.show_delete_confirmation = true; // 点击删除按钮时显示确认对话框
        }
        // 显示确认对话框
        if self.show_delete_confirmation {
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
                                (&self.current_image_path, &self.label_dir)
                            {
                                // 获取标签文件路径
                                let label_path = label_dir
                                    .join(image_path.file_stem().unwrap())
                                    .with_extension("txt");

                                // 删除标签文件
                                if label_path.exists() {
                                    if let Err(e) = std::fs::remove_file(&label_path) {
                                        self.show_status(&format!("删除标签文件失败: {}", e));
                                        return;
                                    }
                                }

                                // 删除图片文件
                                if let Err(e) = std::fs::remove_file(image_path) {
                                    self.show_status(&format!("删除图片文件失败: {}", e));
                                    return;
                                }

                                // 从缓存中移除
                                if let Some(name) = &self.current_image_name {
                                    self.modified_images.remove(name);
                                }
                                self.image_cache.remove(image_path);

                                // 更新文件列表并切换到下一张图片
                                self.update_file_list();
                                self.update_total_statistics();
                                if let Some(prev_path) = self.history.pop() {
                                    self.load_image(&prev_path);
                                } else if let Some(next_path) =
                                    self.cached_image_files.first().cloned()
                                {
                                    self.load_image(&next_path);
                                }
                                self.show_status("已删除当前图片及标签");
                            }
                            self.show_delete_confirmation = false; // 关闭确认对话框
                        }
                        if ui.button("取消").clicked() {
                            self.show_delete_confirmation = false; // 关闭确认对话框
                        }
                    });
                });
        }

        ui.separator();
        ui.heading("操作模式");
        ui.horizontal(|ui| {
            if ui
                .button(if self.is_drawing {
                    "退出绘制"
                } else {
                    "进入绘制"
                })
                .clicked()
            {
                self.is_drawing = !self.is_drawing;
                self.drawing_start = None;
                self.selected_box = None;
                self.show_status(if self.is_drawing {
                    "已进入绘制模式"
                } else {
                    "已退出绘制模式"
                });
            }
        });

        if self.is_drawing {
            ui.heading("添加边界框");
            // 添加类型选择按钮
            ui.horizontal_wrapped(|ui| {
                for class in 0..10 {
                    let text = if class == self.selected_class {
                        format!("【类别 {}】", class)
                    } else {
                        format!("类别 {}", class)
                    };
                    if ui.button(text).clicked() {
                        self.selected_class = class;
                    }
                }
            });
        }

        // 添加弹性空间，将状态消息推到底部
        ui.add_space(ui.available_height() - 30.0);

        // 在底部显示状态消息
        if let Some((message, _)) = &self.status_message {
            ui.horizontal(|ui| {
                ui.label(message);
            });
        }
    }
}

impl eframe::App for AnnotationApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::TopBottomPanel::top("top_panel").show(ctx, |ui| {
            ui.horizontal(|ui| {
                if ui.button("选择图片文件夹").clicked() {
                    self.select_image_dir();
                }
                if ui.button("选择标签文件夹").clicked() {
                    self.select_label_dir();
                }

                if let Some(image_dir) = &self.image_dir {
                    ui.label(format!("图片目录: {}", image_dir.display()));
                }
                if let Some(label_dir) = &self.label_dir {
                    ui.label(format!("标签目录: {}", label_dir.display()));
                }
            });
        });

        egui::SidePanel::left("side_panel").show(ctx, |ui| {
            if let Some(image_dir) = &self.image_dir {
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

                let mut scroll_to_item = false;
                let scroll_area = egui::ScrollArea::vertical().auto_shrink([false; 2]);

                scroll_area.show(ui, |ui| {
                    for entry in image_files {
                        let path = entry.path();
                        let file_name = path
                            .file_name()
                            .unwrap_or_default()
                            .to_string_lossy()
                            .to_string();

                        let is_selected = self
                            .current_image_name
                            .as_ref()
                            .map_or(false, |current| current == &file_name);

                        let is_modified = self.modified_images.contains(&file_name);

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
                            self.load_image(&path);
                        }

                        if is_selected && scroll_to_item {
                            response.scroll_to_me(Some(egui::Align::Center));
                            scroll_to_item = false;
                        }

                        if is_selected && self.scroll_to_current {
                            response.scroll_to_me(Some(egui::Align::Center));
                            self.scroll_to_current = false; // 重置滚动标记
                        }
                    }
                });
            }
        });

        egui::SidePanel::right("statistics_panel")
            .default_width(200.0)
            .show(ctx, |ui| {
                self.show_statistics(ui);
            });

        egui::CentralPanel::default().show(ctx, |ui| {
            if ui.input(|i| i.key_pressed(egui::Key::W) || i.key_pressed(egui::Key::A)) {
                self.switch_image(false, false);
            }
            if ui.input(|i| i.key_pressed(egui::Key::S) || i.key_pressed(egui::Key::D)) {
                self.switch_image(true, false);
            }
            if ui.input(|i| i.key_pressed(egui::Key::Space)) {
                self.switch_image(false, true);
            }
            if ui.input(|i| i.key_pressed(egui::Key::B)) {
                self.go_back();
            }
            if ui.input(|i| i.key_pressed(egui::Key::N)) {
                self.switch_to_next_unmodified();
            }
            if ui.input(|i| i.key_pressed(egui::Key::E)) {
                self.is_drawing = !self.is_drawing;
                self.drawing_start = None;
                self.selected_box = None;
                self.show_status(if self.is_drawing {
                    "已进入绘制模式"
                } else {
                    "已退出绘制模式"
                });
            }

            if let Some(image) = &self.current_image {
                let available_size = ui.available_size();
                let image_size = egui::vec2(image.width() as f32, image.height() as f32);

                let scale = (available_size.x / image_size.x).min(available_size.y / image_size.y);
                let displayed_size = image_size * scale;

                let texture: &egui::TextureHandle = self.texture.get_or_insert_with(|| {
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

                if self.is_drawing {
                    // 监听数字键输入
                    for key_num in 0..10 {
                        let key = egui::Key::from_name(&format!("{}", key_num)).unwrap();
                        if ui.input(|i| i.key_pressed(key)) {
                            self.selected_class = key_num as i32 - 1;
                            self.show_status(&format!("已切换到类别 {}", &self.selected_class));
                        }
                    }

                    if let Some(pointer) = ui.input(|i| i.pointer.hover_pos()) {
                        if ui.input(|i| i.pointer.primary_pressed()) {
                            self.drawing_start = Some(pointer);
                        }

                        if let Some(start) = self.drawing_start {
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
                                    self.bounding_boxes.push(BoundingBox {
                                        class: self.selected_class,
                                        x: ((min_x + max_x) / 2.0) as f64,
                                        y: ((min_y + max_y) / 2.0) as f64,
                                        width: (max_x - min_x) as f64,
                                        height: (max_y - min_y) as f64,
                                    });
                                    self.save_annotations();
                                    self.show_status("已添加新边界框");
                                }
                                self.drawing_start = None;
                            }
                        }
                    }
                } else {
                    if let Some(pointer) = ui.input(|i| i.pointer.hover_pos()) {
                        let mut hovered_box = None;

                        for (i, bbox) in self.bounding_boxes.iter().enumerate().rev() {
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
                            if self.selected_box != hovered_box {
                                self.selected_box = hovered_box;
                                if let Some(idx) = hovered_box {
                                    self.show_status(&format!("已选中边界框 {}", idx));
                                } else {
                                    self.show_status("取消选中");
                                }
                            }
                        }

                        if let Some(selected_idx) = self.selected_box {
                            if ui.input(|i| i.pointer.primary_down()) {
                                let delta = ui.input(|i| i.pointer.delta());
                                let dx = (delta.x as f64) / (displayed_size.x as f64);
                                let dy = (delta.y as f64) / (displayed_size.y as f64);

                                if let Some(bbox) = self.bounding_boxes.get_mut(selected_idx) {
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
                                self.save_annotations();
                                self.show_status("已保存边界框位置");
                            }
                        }
                    }
                }

                if ui.input(|i| i.pointer.secondary_clicked()) {
                    self.is_drawing = false;
                    self.drawing_start = None;
                    self.selected_box = None;
                    self.show_status("已退出绘制模式");
                }

                for (i, bbox) in self.bounding_boxes.iter().enumerate() {
                    let box_width = bbox.width as f32 * displayed_size.x;
                    let box_height = bbox.height as f32 * displayed_size.y;
                    let center_x = offset_x + (bbox.x as f32 * displayed_size.x);
                    let center_y = offset_y + (bbox.y as f32 * displayed_size.y);

                    let rect = egui::Rect::from_min_size(
                        egui::pos2(center_x - box_width / 2.0, center_y - box_height / 2.0),
                        egui::vec2(box_width, box_height),
                    );

                    let box_color = if Some(i) == self.selected_box {
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

                    ui.painter().text(
                        rect.min,
                        egui::Align2::LEFT_TOP,
                        format!("Class {}", bbox.class),
                        egui::FontId::default(),
                        box_color,
                    );
                }
            }

            if ui.input(|i| i.key_pressed(egui::Key::Q)) {
                self.save_annotations();
            }

            if ui.input(|i| i.key_pressed(egui::Key::Delete)) {
                if let Some(selected) = self.selected_box {
                    self.bounding_boxes.remove(selected);
                    self.selected_box = None;
                    self.save_annotations();
                    self.update_total_statistics();
                    self.show_status("已删除选中的边界框");
                }
            }

            if ui.input(|i| i.key_pressed(egui::Key::Escape)) {
                self.is_drawing = false;
                self.drawing_start = None;
                self.selected_box = None;
                self.show_status("已退出绘制模式");
            }
        });
    }

    fn on_exit(&mut self, _gl: Option<&eframe::glow::Context>) {
        self.on_exit();
    }
}

fn main() {
    let app = std::sync::Arc::new(std::sync::Mutex::new(None::<AnnotationApp>));
    let app_clone1 = app.clone();
    let app_clone2 = app.clone();
    let app_clone3 = app.clone();

    // panic hook
    let old_hook = panic::take_hook();
    panic::set_hook(Box::new(move |panic_info| {
        if let Some(app) = app_clone1.lock().unwrap().as_mut() {
            app.save_modified_records();
        }
        old_hook(panic_info);
    }));

    // Ctrl+C handler
    ctrlc::set_handler(move || {
        if let Some(app) = app_clone2.lock().unwrap().as_mut() {
            app.save_modified_records();
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
            Ok(Box::new(app))
        }),
    ) {
        eprintln!("Error running native application: {}", e);
    }
}
