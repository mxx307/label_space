use std::collections::HashMap;

#[derive(Clone)]
pub struct BoundingBox {
    pub class: i32,
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
}

#[derive(Clone)]
pub struct Statistics {
    pub total_images: usize,
    pub modified_images: usize,
    pub total_class_counts: HashMap<i32, usize>, // 所有图片中各类型的数量
    pub current_class_counts: HashMap<i32, usize>, // 当前图片中各类型的数量
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