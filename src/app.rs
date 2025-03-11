use eframe::egui;
use image::DynamicImage;
use rand::seq::IndexedRandom;
use std::collections::{HashMap, HashSet};
use std::fs::{self, File};
use std::io::{BufRead, BufReader, Write};
use std::path::PathBuf;

use crate::models::{BoundingBox, Statistics};
use crate::utils::resize_to_limit;

#[derive(Clone)]
pub struct AnnotationApp {
    pub image_dir: Option<PathBuf>,
    pub label_dir: Option<PathBuf>,
    pub current_image: Option<DynamicImage>,
    pub current_image_path: Option<PathBuf>,
    pub bounding_boxes: Vec<BoundingBox>,
    pub selected_box: Option<usize>,
    pub texture: Option<egui::TextureHandle>,
    pub current_image_name: Option<String>,
    pub modified_images: HashSet<String>,
    pub cached_image_files: Vec<PathBuf>,
    pub status_message: Option<(String, f32)>,
    pub image_cache: HashMap<PathBuf, DynamicImage>,
    pub max_cache_size: usize,
    pub statistics: Statistics,
    pub selected_class: i32,
    pub is_drawing: bool,
    pub drawing_start: Option<egui::Pos2>,
    pub scroll_to_current: bool,
    pub history: Vec<PathBuf>, // 记录浏览历史
    pub show_delete_confirmation: bool,
    pub show_export_result_dialog: bool,
    pub export_result_info: String,
    pub show_labels: bool,       // 控制标签显示
    pub show_center_points: bool, // 控制中心点显示
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
            modified_images: HashSet::new(),
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
            show_export_result_dialog: false,
            export_result_info: String::new(),
            show_labels: true,           // 默认显示标签
            show_center_points: false,   // 默认不显示中心点
        };
        app.load_modified_records();
        app
    }
}

impl AnnotationApp {
    pub fn show_status(&mut self, message: &str) {
        self.status_message = Some((message.to_string(), 2.0));
    }

    pub fn load_image(&mut self, path: &PathBuf) {
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
            let img = resize_to_limit(&img, 1920, 1080);
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

    pub fn load_annotations(&mut self) {
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

    pub fn save_annotations(&mut self) {
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

    pub fn update_file_list(&mut self) {
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

    pub fn update_image_cache(&mut self, path: PathBuf, img: DynamicImage) {
        self.image_cache.insert(path.clone(), img);

        if let Some(current_pos) = self.cached_image_files.iter().position(|p| p == &path) {
            if current_pos + 1 < self.cached_image_files.len() {
                let next_path = &self.cached_image_files[current_pos + 1];
                if !self.image_cache.contains_key(next_path) {
                    if let Ok(img) = image::open(next_path) {
                        let img = resize_to_limit(&img, 1920, 1080);
                        self.image_cache.insert(next_path.clone(), img);
                    }
                }
            }

            if current_pos > 0 {
                let prev_path = &self.cached_image_files[current_pos - 1];
                if !self.image_cache.contains_key(prev_path) {
                    if let Ok(img) = image::open(prev_path) {
                        let img = resize_to_limit(&img, 1920, 1080);
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

    pub fn switch_image(&mut self, next: bool, random_unmodified: bool) {
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

    pub fn switch_to_next_unmodified(&mut self) {
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

    pub fn go_back(&mut self) {
        if let Some(prev_path) = self.history.pop() {
            self.load_image(&prev_path);
            self.scroll_to_current = true;
        }
    }

    pub fn load_modified_records(&mut self) {
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

    pub fn save_modified_records(&self) {
        if let Some(label_dir) = &self.label_dir {
            let record_path = label_dir.join("modified_records.txt");
            if let Ok(mut file) = File::create(record_path) {
                for filename in &self.modified_images {
                    writeln!(file, "{}", filename).ok();
                }
            }
        }
    }

    pub fn select_image_dir(&mut self) {
        if let Some(path) = rfd::FileDialog::new().pick_folder() {
            self.image_dir = Some(path);
            self.update_file_list();

            if !self.cached_image_files.is_empty() {
                let path = self.cached_image_files[0].clone();
                self.load_image(&path);
            }
        }
    }

    pub fn select_label_dir(&mut self) {
        if let Some(path) = rfd::FileDialog::new().pick_folder() {
            self.label_dir = Some(path);
            self.load_modified_records();
            self.update_total_statistics();
            self.show_status("已加载标签目录");
        }
    }
    pub fn update_statistics(&mut self) {
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

    pub fn update_total_statistics(&mut self) {
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

    pub fn on_exit(&mut self) {
        self.save_modified_records();
    }

    pub fn export_modified_files(&mut self, export_dir: PathBuf) -> Result<i32, String> {
        // 检查是否有已修改的文件
        if self.modified_images.is_empty() {
            return Err("没有已修改的文件可导出".to_string());
        }
        
        // 检查源目录是否存在
        if self.image_dir.is_none() || self.label_dir.is_none() {
            return Err("请先选择图片和标签目录".to_string());
        }
        
        let image_dir = self.image_dir.as_ref().unwrap();
        let label_dir = self.label_dir.as_ref().unwrap();
        
        // 创建导出目录结构
        let images_dir = export_dir.join("images");
        let labels_dir = export_dir.join("labels");
        
        // 检查目标目录是否为空
        let images_empty = if images_dir.exists() {
            fs::read_dir(&images_dir).map(|entries| entries.count() == 0).unwrap_or(true)
        } else {
            true
        };
        
        let labels_empty = if labels_dir.exists() {
            fs::read_dir(&labels_dir).map(|entries| entries.count() == 0).unwrap_or(true)
        } else {
            true
        };
        
        // 如果目标目录不为空，返回错误
        if !images_empty || !labels_empty {
            return Err("目标目录不为空，请选择空目录或新目录".to_string());
        }
        
        // 创建目录（如果不存在）
        if !images_dir.exists() {
            fs::create_dir_all(&images_dir).map_err(|e| format!("创建图片目录失败: {}", e))?;
        }
        
        if !labels_dir.exists() {
            fs::create_dir_all(&labels_dir).map_err(|e| format!("创建标签目录失败: {}", e))?;
        }
        
        // 导出已修改的文件
        let mut exported_count = 0;
        
        for filename in &self.modified_images {
            // 复制图片文件
            let src_image_path = image_dir.join(filename);
            let dst_image_path = images_dir.join(filename);
            
            if src_image_path.exists() {
                fs::copy(&src_image_path, &dst_image_path)
                    .map_err(|e| format!("复制图片文件失败 {}: {}", filename, e))?;
            }
            
            // 复制标签文件
            let label_filename = src_image_path.file_stem().unwrap().to_string_lossy().to_string() + ".txt";
            let src_label_path = label_dir.join(&label_filename);
            let dst_label_path = labels_dir.join(&label_filename);
            
            if src_label_path.exists() {
                fs::copy(&src_label_path, &dst_label_path)
                    .map_err(|e| format!("复制标签文件失败 {}: {}", label_filename, e))?;
            }
            
            exported_count += 1;
        }
        
        Ok(exported_count)
    }
}