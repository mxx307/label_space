use image::DynamicImage;

pub fn resize_to_limit(img: &DynamicImage, max_width: u32, max_height: u32) -> DynamicImage {
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