use std::{
    cell::RefCell,
    collections::HashMap,
    path::{Path, PathBuf},
};

use slint::{Image, Rgba8Pixel, SharedPixelBuffer};

thread_local! {
    static IMAGE_CACHE: RefCell<HashMap<PathBuf, Option<Image>>> = RefCell::new(HashMap::new());
}

pub fn load_cached(path: &Path) -> Option<Image> {
    IMAGE_CACHE.with(|cache| {
        if let Some(cached) = cache.borrow().get(path) {
            return cached.clone();
        }

        let image = match decode(path) {
            Ok(image) => Some(image),
            Err(error) => {
                eprintln!("failed to decode sprite frame {}: {error}", path.display());
                None
            }
        };

        cache.borrow_mut().insert(path.to_path_buf(), image.clone());

        image
    })
}

fn decode(path: &Path) -> Result<Image, String> {
    let decoded = image::ImageReader::open(path)
        .map_err(|error| error.to_string())?
        .decode()
        .map_err(|error| error.to_string())?
        .into_rgba8();

    let buffer = SharedPixelBuffer::<Rgba8Pixel>::clone_from_slice(
        decoded.as_raw(),
        decoded.width(),
        decoded.height(),
    );

    Ok(Image::from_rgba8(buffer))
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

        let decoded = decode(&path).expect("test image should decode");
        let size = decoded.size();

        let _ = std::fs::remove_file(path);

        assert_eq!(size.width, 2);
        assert_eq!(size.height, 3);
    }

    #[test]
    fn decodes_png_into_slint_image() {
        round_trip(image::ImageFormat::Png, "png");
    }

    #[test]
    fn decodes_webp_into_slint_image() {
        round_trip(image::ImageFormat::WebP, "webp");
    }
}
