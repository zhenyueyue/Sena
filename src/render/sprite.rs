use std::{
    cell::RefCell,
    collections::HashMap,
    path::{Path, PathBuf},
};

use slint::{Image, Rgba8Pixel, SharedPixelBuffer};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AlphaRect {
    pub left: u32,
    pub top: u32,
    pub right: u32,
    pub bottom: u32,
}

#[derive(Debug, Clone)]
pub struct SpriteFrame {
    pub image: Image,
    pub source_width: u32,
    pub source_height: u32,
    pub width: u32,
    pub height: u32,
    pub alpha_rects: Vec<AlphaRect>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
struct CacheScale {
    logical_scale_bits: u32,
    dpi_scale_bits: u32,
}

thread_local! {
    static IMAGE_CACHE: RefCell<HashMap<(PathBuf, u8, CacheScale), Option<SpriteFrame>>> =
        RefCell::new(HashMap::new());
}

pub fn load_cached(
    path: &Path,
    alpha_threshold: u8,
    logical_scale: f32,
    dpi_scale: f32,
) -> Option<SpriteFrame> {
    IMAGE_CACHE.with(|cache| {
        let cache_scale = CacheScale {
            logical_scale_bits: logical_scale.to_bits(),
            dpi_scale_bits: dpi_scale.to_bits(),
        };
        let key = (path.to_path_buf(), alpha_threshold, cache_scale);

        if let Some(cached) = cache.borrow().get(&key) {
            return cached.clone();
        }

        let frame = match decode(path, alpha_threshold, logical_scale, dpi_scale) {
            Ok(frame) => Some(frame),
            Err(error) => {
                eprintln!("failed to decode sprite frame {}: {error}", path.display());
                None
            }
        };

        cache.borrow_mut().insert(key, frame.clone());

        frame
    })
}

fn decode(
    path: &Path,
    alpha_threshold: u8,
    logical_scale: f32,
    dpi_scale: f32,
) -> Result<SpriteFrame, String> {
    let decoded = image::ImageReader::open(path)
        .map_err(|error| error.to_string())?
        .decode()
        .map_err(|error| error.to_string())?;

    let source_width = decoded.width();
    let source_height = decoded.height();
    let runtime_scale = (logical_scale * dpi_scale).max(0.01);
    let target_width = ((source_width as f32 * runtime_scale).round() as u32).max(1);
    let target_height = ((source_height as f32 * runtime_scale).round() as u32).max(1);

    let decoded = if target_width < source_width || target_height < source_height {
        decoded.resize_exact(
            target_width,
            target_height,
            image::imageops::FilterType::Lanczos3,
        )
    } else {
        decoded
    }
    .into_rgba8();

    let width = decoded.width();
    let height = decoded.height();
    let alpha_rects = build_alpha_rects(decoded.as_raw(), width, height, alpha_threshold);

    let buffer = SharedPixelBuffer::<Rgba8Pixel>::clone_from_slice(decoded.as_raw(), width, height);

    Ok(SpriteFrame {
        image: Image::from_rgba8(buffer),
        source_width,
        source_height,
        width,
        height,
        alpha_rects,
    })
}

fn build_alpha_rects(rgba: &[u8], width: u32, height: u32, threshold: u8) -> Vec<AlphaRect> {
    let mut rectangles = Vec::<AlphaRect>::new();
    let mut previous_row = HashMap::<(u32, u32), usize>::new();

    for y in 0..height {
        let mut current_row = HashMap::<(u32, u32), usize>::new();
        let mut x = 0;

        while x < width {
            let alpha = rgba[((y * width + x) * 4 + 3) as usize];
            if alpha < threshold {
                x += 1;
                continue;
            }

            let left = x;
            x += 1;

            while x < width {
                let alpha = rgba[((y * width + x) * 4 + 3) as usize];
                if alpha < threshold {
                    break;
                }
                x += 1;
            }

            let right = x;
            let key = (left, right);

            if let Some(index) = previous_row.get(&key).copied() {
                rectangles[index].bottom = y + 1;
                current_row.insert(key, index);
            } else {
                let index = rectangles.len();
                rectangles.push(AlphaRect {
                    left,
                    top: y,
                    right,
                    bottom: y + 1,
                });
                current_row.insert(key, index);
            }
        }

        previous_row = current_row;
    }

    rectangles
}

#[cfg(test)]
mod tests {
    use super::*;

    fn round_trip(format: image::ImageFormat, extension: &str) {
        let nonce = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("system time should be after Unix epoch")
            .as_nanos();
        let path = std::env::temp_dir().join(format!(
            "sena-sprite-test-{}-{nonce}.{extension}",
            std::process::id()
        ));

        let pixels = image::RgbaImage::from_pixel(2, 3, image::Rgba([255, 0, 128, 200]));
        pixels
            .save_with_format(&path, format)
            .expect("test image should save");

        let decoded = decode(&path, 8, 1.0, 1.0).expect("test image should decode");

        let _ = std::fs::remove_file(path);

        assert_eq!(decoded.width, 2);
        assert_eq!(decoded.height, 3);
        assert_eq!(decoded.image.size().width, 2);
        assert_eq!(decoded.image.size().height, 3);
    }

    #[test]
    fn decodes_png_into_slint_image() {
        round_trip(image::ImageFormat::Png, "png");
    }

    #[test]
    fn decodes_webp_into_slint_image() {
        round_trip(image::ImageFormat::WebP, "webp");
    }

    #[test]
    fn decode_downscales_to_runtime_size() {
        let nonce = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("system time should be after Unix epoch")
            .as_nanos();
        let path = std::env::temp_dir().join(format!(
            "sena-sprite-scale-test-{}-{nonce}.png",
            std::process::id()
        ));

        let pixels = image::RgbaImage::from_pixel(100, 200, image::Rgba([255, 255, 255, 255]));
        pixels
            .save_with_format(&path, image::ImageFormat::Png)
            .expect("test PNG should save");

        let decoded = decode(&path, 8, 0.5, 1.0).expect("scaled image should decode");
        let _ = std::fs::remove_file(path);

        assert_eq!(decoded.width, 50);
        assert_eq!(decoded.height, 100);
    }

    #[test]
    fn alpha_rects_merge_identical_runs_vertically() {
        let rgba = [
            0, 0, 0, 0, 0, 0, 0, 255, 0, 0, 0, 255, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 255, 0, 0, 0,
            255, 0, 0, 0, 0,
        ];

        assert_eq!(
            build_alpha_rects(&rgba, 4, 2, 8),
            vec![AlphaRect {
                left: 1,
                top: 0,
                right: 3,
                bottom: 2,
            }]
        );
    }
}
