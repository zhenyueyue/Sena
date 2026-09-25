use std::{
    collections::BTreeMap,
    env, fs,
    path::{Path, PathBuf},
    process::ExitCode,
};

use image::GenericImageView;
use serde::Deserialize;

const EXPECTED_WIDTH: u32 = 768;
const EXPECTED_HEIGHT: u32 = 1024;
const RECOMMENDED_PADDING: u32 = 32;

#[derive(Debug, Deserialize)]
struct Manifest {
    schema_version: u32,
    renderer: String,
    #[serde(default)]
    animations: BTreeMap<String, AnimationDefinition>,
    #[serde(default)]
    interactions: BTreeMap<String, AnimationDefinition>,
}

#[derive(Debug, Deserialize)]
struct AnimationDefinition {
    #[serde(default)]
    frames: Vec<String>,
    #[serde(default)]
    frame_durations_ms: Vec<u64>,
    #[serde(default)]
    interval_ms: Option<u64>,
    #[serde(default = "default_looping")]
    looping: bool,
}

const fn default_looping() -> bool {
    true
}

#[derive(Default)]
struct Report {
    errors: Vec<String>,
    warnings: Vec<String>,
    checked_frames: usize,
}

fn main() -> ExitCode {
    let root = env::args()
        .nth(1)
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("pets/sena"));

    let mut report = Report::default();
    let manifest = match load_manifest(&root) {
        Ok(manifest) => manifest,
        Err(error) => {
            eprintln!("Sena asset check failed: {error}");
            return ExitCode::FAILURE;
        }
    };

    if manifest.schema_version != 1 {
        report.errors.push(format!(
            "pet.json schema_version is {}, expected 1",
            manifest.schema_version
        ));
    }
    if manifest.renderer != "sprite" {
        report.warnings.push(format!(
            "renderer is {:?}; this checker is optimized for the sprite package",
            manifest.renderer
        ));
    }

    for (name, animation) in &manifest.animations {
        check_animation(&root, "animation", name, animation, false, &mut report);
    }

    for (name, animation) in &manifest.interactions {
        check_animation(&root, "interaction", name, animation, true, &mut report);
    }

    println!(
        "Sena asset check: {} frame(s) inspected",
        report.checked_frames
    );
    for warning in &report.warnings {
        println!("  warning: {warning}");
    }
    for error in &report.errors {
        eprintln!("  error: {error}");
    }

    if report.errors.is_empty() {
        println!("Asset package is valid.");
        ExitCode::SUCCESS
    } else {
        eprintln!("Asset package has {} error(s).", report.errors.len());
        ExitCode::FAILURE
    }
}

fn load_manifest(root: &Path) -> Result<Manifest, String> {
    let path = root.join("pet.json");
    let json = fs::read_to_string(&path)
        .map_err(|error| format!("could not read {}: {error}", path.display()))?;
    serde_json::from_str(&json)
        .map_err(|error| format!("could not parse {}: {error}", path.display()))
}

fn check_animation(
    root: &Path,
    kind: &str,
    name: &str,
    animation: &AnimationDefinition,
    interaction: bool,
    report: &mut Report,
) {
    if animation.frames.is_empty() {
        if interaction {
            report.warnings.push(format!(
                "{kind} {name} has no frames yet; runtime fallback will be used"
            ));
        }
        return;
    }

    if interaction && animation.looping {
        report.errors.push(format!(
            "interaction {name} must be one-shot (set looping to false)"
        ));
    }

    if !animation.frame_durations_ms.is_empty()
        && animation.frame_durations_ms.len() != animation.frames.len()
    {
        report.errors.push(format!(
            "{kind} {name} has {} frame(s) but {} frame duration(s)",
            animation.frames.len(),
            animation.frame_durations_ms.len()
        ));
    }

    if animation.frame_durations_ms.is_empty() && animation.interval_ms.unwrap_or(0) == 0 {
        report.warnings.push(format!(
            "{kind} {name} has no positive interval_ms or frame_durations_ms"
        ));
    }

    for (index, relative) in animation.frames.iter().enumerate() {
        let path = root.join(relative);
        check_frame(&path, kind, name, index, report);
    }
}

fn check_frame(path: &Path, kind: &str, name: &str, index: usize, report: &mut Report) {
    if !path.is_file() {
        report.errors.push(format!(
            "{kind} {name} frame {index:03} is missing: {}",
            path.display()
        ));
        return;
    }

    let decoded = match image::ImageReader::open(path)
        .map_err(|error| error.to_string())
        .and_then(|reader| reader.decode().map_err(|error| error.to_string()))
    {
        Ok(image) => image,
        Err(error) => {
            report.errors.push(format!(
                "{kind} {name} frame {index:03} could not be decoded ({}): {error}",
                path.display()
            ));
            return;
        }
    };

    report.checked_frames += 1;
    let (width, height) = decoded.dimensions();
    if width != EXPECTED_WIDTH || height != EXPECTED_HEIGHT {
        report.errors.push(format!(
            "{kind} {name} frame {index:03} is {width}x{height}; expected {EXPECTED_WIDTH}x{EXPECTED_HEIGHT}"
        ));
    }

    let rgba = decoded.to_rgba8();
    for (label, x, y) in [
        ("top-left", 0, 0),
        ("top-right", width.saturating_sub(1), 0),
        ("bottom-left", 0, height.saturating_sub(1)),
        (
            "bottom-right",
            width.saturating_sub(1),
            height.saturating_sub(1),
        ),
    ] {
        if rgba.get_pixel(x, y).0[3] != 0 {
            report.errors.push(format!(
                "{kind} {name} frame {index:03} has a non-transparent {label} corner"
            ));
        }
    }

    if width >= RECOMMENDED_PADDING * 2 && height >= RECOMMENDED_PADDING * 2 {
        let mut touches_padding = false;
        'outer: for y in 0..height {
            for x in 0..width {
                if x >= RECOMMENDED_PADDING
                    && x < width - RECOMMENDED_PADDING
                    && y >= RECOMMENDED_PADDING
                    && y < height - RECOMMENDED_PADDING
                {
                    continue;
                }
                if rgba.get_pixel(x, y).0[3] >= 8 {
                    touches_padding = true;
                    break 'outer;
                }
            }
        }

        if touches_padding {
            report.warnings.push(format!(
                "{kind} {name} frame {index:03} enters the recommended {RECOMMENDED_PADDING}px transparent padding"
            ));
        }
    }
}
