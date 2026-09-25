#![cfg_attr(not(target_os = "windows"), allow(dead_code, unused_imports))]

use std::{
    collections::BTreeSet,
    path::{Path, PathBuf},
};

#[allow(dead_code)]
#[path = "../src/render/spine/mod.rs"]
mod spine;

use spine::{SpineAnimationInfo, SpineRenderFrame, SpineRuntime};

const REQUIRED_SKIN: &str = "base";
const REQUIRED_ANIMATIONS: &[&str] = &["idle", "blink_l", "blink_r"];
const REQUIRED_BONES: &[&str] = &["root", "body_root", "head", "face_root", "eye_l", "eye_r"];
const FUTURE_ANIMATIONS: &[&str] = &[
    "blink_both",
    "coding_idle",
    "coding_type",
    "listening_idle",
    "music_bob",
    "drowsy_idle",
    "sleep_idle",
    "petting",
    "stretch",
    "look_at_cat",
    "daydream",
    "walk_l",
    "walk_r",
    "turn_l_to_r",
    "turn_r_to_l",
];

#[derive(Debug)]
struct Args {
    skeleton: PathBuf,
    atlas: PathBuf,
    skin: String,
    inventory_only: bool,
}

#[derive(Debug, Clone, Copy)]
struct FrameMetrics {
    batches: usize,
    vertices: usize,
    triangles: usize,
    bounds_width: f32,
    bounds_height: f32,
}

fn main() {
    if let Err(error) = run() {
        eprintln!("\n[FAIL] Sena Spine asset gate: {error}");
        std::process::exit(1);
    }
}

fn run() -> Result<(), String> {
    let args = parse_args()?;

    if !args.skeleton.is_file() {
        return Err(format!("skeleton not found: {}", args.skeleton.display()));
    }
    if !args.atlas.is_file() {
        return Err(format!("atlas not found: {}", args.atlas.display()));
    }

    let mut runtime = SpineRuntime::from_files(&args.skeleton, &args.atlas, 1.0)
        .map_err(|error| format!("runtime could not load asset: {error}"))?;

    let version = runtime
        .runtime_version()
        .unwrap_or_else(|| "<unknown>".to_string());
    let skins = runtime.skins();
    let animations = runtime.animations();
    let atlas_pages = runtime.atlas_pages();

    println!("Sena Spine Asset Gate");
    println!("  skeleton : {}", args.skeleton.display());
    println!("  atlas    : {}", args.atlas.display());
    println!("  version  : {version}");
    println!("  skins    : {}", join_or_none(&skins));
    println!(
        "  animations: {}",
        animations
            .iter()
            .map(|animation| format!("{} ({:.3}s)", animation.name, animation.duration_seconds))
            .collect::<Vec<_>>()
            .join(", ")
    );
    println!("  pages    : {}", join_or_none(&atlas_pages));

    verify_atlas_pages_exist(&args.atlas, &atlas_pages)?;

    if args.inventory_only {
        println!("\n[PASS] inventory-only inspection completed.");
        return Ok(());
    }

    if !version.starts_with("3.8") {
        return Err(format!("expected Spine 3.8.x, asset reports {version}"));
    }

    if !skins.iter().any(|skin| skin == &args.skin) {
        return Err(format!(
            "required skin {} missing; found: {}",
            args.skin,
            join_or_none(&skins)
        ));
    }

    let animation_names = animations
        .iter()
        .map(|animation| animation.name.as_str())
        .collect::<BTreeSet<_>>();
    let missing_animations = REQUIRED_ANIMATIONS
        .iter()
        .copied()
        .filter(|name| !animation_names.contains(name))
        .collect::<Vec<_>>();

    if !missing_animations.is_empty() {
        return Err(format!(
            "R3B requires animations [{}]; missing [{}]",
            REQUIRED_ANIMATIONS.join(", "),
            missing_animations.join(", ")
        ));
    }

    for required in REQUIRED_ANIMATIONS {
        let info = animation_info(&animations, required)
            .ok_or_else(|| format!("required animation {required} disappeared from inventory"))?;
        if !info.duration_seconds.is_finite() || info.duration_seconds <= 0.0 {
            return Err(format!(
                "required animation {required} has invalid duration {:.3}s",
                info.duration_seconds
            ));
        }
    }

    runtime
        .set_skin(&args.skin)
        .map_err(|error| format!("failed to apply skin {}: {error}", args.skin))?;
    runtime.update(0.0);

    let missing_bones = REQUIRED_BONES
        .iter()
        .copied()
        .filter(|name| runtime.bone_world_transform(name).is_none())
        .collect::<Vec<_>>();
    if !missing_bones.is_empty() {
        return Err(format!(
            "R3B skeleton contract missing bones [{}]",
            missing_bones.join(", ")
        ));
    }

    let setup_frame = runtime
        .render_frame()
        .map_err(|error| format!("setup pose extraction failed: {error}"))?;
    let setup_metrics = frame_metrics(&setup_frame)?;

    runtime
        .set_animation(0, "idle", true)
        .map_err(|error| format!("failed to play idle: {error}"))?;
    let idle_duration = animation_info(&animations, "idle")
        .map(|info| info.duration_seconds)
        .unwrap_or(1.0)
        .max(0.001);
    runtime.update((idle_duration * 0.5).min(0.5));
    let idle_frame = runtime
        .render_frame()
        .map_err(|error| format!("idle frame extraction failed: {error}"))?;
    let idle_metrics = frame_metrics(&idle_frame)?;

    println!("\nR3B checks");
    println!("  [OK] Spine version 3.8.x");
    println!("  [OK] skin {}", args.skin);
    println!("  [OK] idle / blink_l / blink_r");
    println!("  [OK] root/body/head/face/eye bones");
    println!(
        "  [OK] setup pose: {} batches, {} vertices, {} triangles, {:.1} x {:.1}",
        setup_metrics.batches,
        setup_metrics.vertices,
        setup_metrics.triangles,
        setup_metrics.bounds_width,
        setup_metrics.bounds_height
    );
    println!(
        "  [OK] idle frame: {} batches, {} vertices, {} triangles",
        idle_metrics.batches, idle_metrics.vertices, idle_metrics.triangles
    );
    println!("  [OK] atlas page files exist");

    if setup_metrics.vertices > 2500 {
        println!(
            "  [WARN] {} effective vertices exceeds the production upper target of about 2500",
            setup_metrics.vertices
        );
    } else if setup_metrics.vertices > 1500 {
        println!(
            "  [WARN] {} effective vertices exceeds the preferred target of 1500",
            setup_metrics.vertices
        );
    }

    let future_missing = FUTURE_ANIMATIONS
        .iter()
        .copied()
        .filter(|name| !animation_names.contains(name))
        .collect::<Vec<_>>();
    if !future_missing.is_empty() {
        println!(
            "  [INFO] later milestones still need: {}",
            future_missing.join(", ")
        );
    }

    println!("\n[PASS] Sena asset satisfies the R3B import gate.");
    Ok(())
}

fn parse_args() -> Result<Args, String> {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("pets")
        .join("sena")
        .join("spine")
        .join("export");
    let default_skel = root.join("sena.skel");
    let default_json = root.join("sena.json");

    let mut skeleton = if default_skel.is_file() {
        default_skel
    } else {
        default_json
    };
    let mut atlas = root.join("sena.atlas");
    let mut skin = REQUIRED_SKIN.to_string();
    let mut inventory_only = false;

    let mut args = std::env::args().skip(1);
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--skeleton" => {
                skeleton = PathBuf::from(args.next().ok_or("--skeleton requires a path")?);
            }
            "--atlas" => {
                atlas = PathBuf::from(args.next().ok_or("--atlas requires a path")?);
            }
            "--skin" => skin = args.next().ok_or("--skin requires a name")?,
            "--inventory-only" => inventory_only = true,
            "--help" | "-h" => {
                println!(
                    "Usage: cargo run --example sena_spine_asset_gate -- [--skeleton PATH] [--atlas PATH] [--skin base] [--inventory-only]"
                );
                std::process::exit(0);
            }
            unknown => return Err(format!("unknown argument: {unknown}")),
        }
    }

    Ok(Args {
        skeleton,
        atlas,
        skin,
        inventory_only,
    })
}

fn verify_atlas_pages_exist(atlas: &Path, pages: &[String]) -> Result<(), String> {
    if pages.is_empty() {
        return Err("atlas contains no texture pages".into());
    }

    let root = atlas.parent().unwrap_or_else(|| Path::new("."));
    let missing = pages
        .iter()
        .filter(|page| !root.join(page).is_file())
        .cloned()
        .collect::<Vec<_>>();

    if missing.is_empty() {
        Ok(())
    } else {
        Err(format!(
            "atlas references missing texture page files: {}",
            missing.join(", ")
        ))
    }
}

fn animation_info<'a>(
    animations: &'a [SpineAnimationInfo],
    name: &str,
) -> Option<&'a SpineAnimationInfo> {
    animations.iter().find(|animation| animation.name == name)
}

fn frame_metrics(frame: &SpineRenderFrame) -> Result<FrameMetrics, String> {
    if frame.batches.is_empty() {
        return Err("setup/animation frame has no renderable batches".into());
    }

    let mut min = [f32::INFINITY; 2];
    let mut max = [f32::NEG_INFINITY; 2];
    let mut vertices = 0usize;
    let mut triangles = 0usize;

    for batch in &frame.batches {
        if batch.vertices.is_empty() {
            return Err(format!(
                "attachment {} produced an empty vertex buffer",
                batch.attachment_name
            ));
        }
        if batch.indices.is_empty() || batch.indices.len() % 3 != 0 {
            return Err(format!(
                "attachment {} produced invalid triangle indices",
                batch.attachment_name
            ));
        }

        vertices += batch.vertices.len();
        triangles += batch.indices.len() / 3;

        for vertex in &batch.vertices {
            if !vertex.position[0].is_finite() || !vertex.position[1].is_finite() {
                return Err(format!(
                    "attachment {} contains non-finite vertex coordinates",
                    batch.attachment_name
                ));
            }

            min[0] = min[0].min(vertex.position[0]);
            min[1] = min[1].min(vertex.position[1]);
            max[0] = max[0].max(vertex.position[0]);
            max[1] = max[1].max(vertex.position[1]);
        }
    }

    let bounds_width = max[0] - min[0];
    let bounds_height = max[1] - min[1];
    if !bounds_width.is_finite()
        || !bounds_height.is_finite()
        || bounds_width <= 1.0
        || bounds_height <= 1.0
    {
        return Err(format!(
            "renderable bounds are invalid: {:.2} x {:.2}",
            bounds_width, bounds_height
        ));
    }

    Ok(FrameMetrics {
        batches: frame.batches.len(),
        vertices,
        triangles,
        bounds_width,
        bounds_height,
    })
}

fn join_or_none(values: &[String]) -> String {
    if values.is_empty() {
        "<none>".to_string()
    } else {
        values.join(", ")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_frame_is_rejected() {
        assert!(frame_metrics(&SpineRenderFrame::default()).is_err());
    }
}
