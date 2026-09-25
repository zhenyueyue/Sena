#![cfg_attr(not(target_os = "windows"), allow(dead_code, unused_imports))]

use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    path::{Path, PathBuf},
};

use serde::Deserialize;

#[allow(dead_code)]
#[path = "../src/render/spine/mod.rs"]
mod spine;

use spine::{SpineAnimationInfo, SpineBoneInfo, SpineRenderFrame, SpineRuntime};

#[derive(Debug, Clone, Deserialize)]
struct GeometryBudget {
    preferred_effective_vertices: usize,
    warning_effective_vertices: usize,
    minimum_setup_width: f32,
    minimum_setup_height: f32,
}

#[derive(Debug, Clone, Deserialize)]
struct AssetContract {
    schema_version: u32,
    character: String,
    spine_editor_major_minor: String,
    default_skin: String,
    required_animations: Vec<String>,
    recommended_next_animations: Vec<String>,
    required_bones: Vec<String>,
    required_bone_parents: BTreeMap<String, Option<String>>,
    recommended_next_bones: Vec<String>,
    geometry_budget: GeometryBudget,
    required_export_files: Vec<String>,
    skeleton_candidates: Vec<String>,
}

#[derive(Debug)]
struct Args {
    skeleton: Option<PathBuf>,
    atlas: Option<PathBuf>,
    contract: PathBuf,
    skin: Option<String>,
    inventory_only: bool,
    contract_only: bool,
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
    let contract = load_contract(&args.contract)?;
    validate_contract(&contract)?;

    println!("Sena Spine Asset Contract");
    println!("  contract : {}", args.contract.display());
    println!("  character: {}", contract.character);
    println!("  target   : Spine {}.x", contract.spine_editor_major_minor);
    println!("  skin     : {}", contract.default_skin);
    println!(
        "  required animations: {}",
        contract.required_animations.join(", ")
    );
    println!(
        "  required bones     : {}",
        contract.required_bones.join(", ")
    );

    if args.contract_only {
        println!("\n[PASS] R3B contract is valid.");
        return Ok(());
    }

    let export_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("pets")
        .join("sena")
        .join("spine")
        .join("export");
    let skeleton = match args.skeleton {
        Some(path) => path,
        None => resolve_skeleton(&export_root, &contract.skeleton_candidates)?,
    };
    let atlas = match args.atlas {
        Some(path) => path,
        None => resolve_default_atlas(&export_root, &contract.required_export_files)?,
    };
    let skin = args.skin.unwrap_or_else(|| contract.default_skin.clone());

    if !skeleton.is_file() {
        return Err(format!("skeleton not found: {}", skeleton.display()));
    }
    if !atlas.is_file() {
        return Err(format!("atlas not found: {}", atlas.display()));
    }

    let mut runtime = SpineRuntime::from_files(&skeleton, &atlas, 1.0)
        .map_err(|error| format!("runtime could not load asset: {error}"))?;

    let version = runtime
        .runtime_version()
        .unwrap_or_else(|| "<unknown>".to_string());
    let skins = runtime.skins();
    let animations = runtime.animations();
    let atlas_pages = runtime.atlas_pages();
    let bones = runtime.bones();

    println!("\nSena Spine Asset Gate");
    println!("  skeleton : {}", skeleton.display());
    println!("  atlas    : {}", atlas.display());
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
    println!(
        "  bones    : {}",
        bones
            .iter()
            .map(|bone| match &bone.parent_name {
                Some(parent) => format!("{}<-{}", bone.name, parent),
                None => format!("{}<-<root>", bone.name),
            })
            .collect::<Vec<_>>()
            .join(", ")
    );

    verify_atlas_pages_exist(&atlas, &atlas_pages)?;

    if args.inventory_only {
        println!("\n[PASS] inventory-only inspection completed.");
        return Ok(());
    }

    if !version.starts_with(&contract.spine_editor_major_minor) {
        return Err(format!(
            "expected Spine {}.x, asset reports {version}",
            contract.spine_editor_major_minor
        ));
    }

    if !skins.iter().any(|available| available == &skin) {
        return Err(format!(
            "required skin {} missing; found: {}",
            skin,
            join_or_none(&skins)
        ));
    }

    let animation_names = animations
        .iter()
        .map(|animation| animation.name.as_str())
        .collect::<BTreeSet<_>>();
    let missing_animations = contract
        .required_animations
        .iter()
        .filter(|name| !animation_names.contains(name.as_str()))
        .cloned()
        .collect::<Vec<_>>();

    if !missing_animations.is_empty() {
        return Err(format!(
            "R3B requires animations [{}]; missing [{}]",
            contract.required_animations.join(", "),
            missing_animations.join(", ")
        ));
    }

    for required in &contract.required_animations {
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
        .set_skin(&skin)
        .map_err(|error| format!("failed to apply skin {}: {error}", skin))?;
    runtime.update(0.0);

    verify_bone_hierarchy(&contract, &bones)?;

    let setup_frame = runtime
        .render_frame()
        .map_err(|error| format!("setup pose extraction failed: {error}"))?;
    let setup_metrics = frame_metrics(&setup_frame, &contract.geometry_budget)?;

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
    let idle_metrics = frame_metrics(&idle_frame, &contract.geometry_budget)?;

    println!("\nR3B checks");
    println!(
        "  [OK] Spine version {}.x",
        contract.spine_editor_major_minor
    );
    println!("  [OK] skin {}", skin);
    println!(
        "  [OK] animations {}",
        contract.required_animations.join(" / ")
    );
    println!(
        "  [OK] required bones {}",
        contract.required_bones.join(" / ")
    );
    println!("  [OK] required bone parent hierarchy");
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

    if setup_metrics.vertices > contract.geometry_budget.warning_effective_vertices {
        println!(
            "  [WARN] {} effective vertices exceeds the warning target of {}",
            setup_metrics.vertices, contract.geometry_budget.warning_effective_vertices
        );
    } else if setup_metrics.vertices > contract.geometry_budget.preferred_effective_vertices {
        println!(
            "  [WARN] {} effective vertices exceeds the preferred target of {}",
            setup_metrics.vertices, contract.geometry_budget.preferred_effective_vertices
        );
    }

    let future_missing = contract
        .recommended_next_animations
        .iter()
        .filter(|name| !animation_names.contains(name.as_str()))
        .cloned()
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
    let default_contract = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("pets")
        .join("sena")
        .join("spine")
        .join("settings")
        .join("r3b_contract.json");

    let mut skeleton = None;
    let mut atlas = None;
    let mut contract = default_contract;
    let mut skin = None;
    let mut inventory_only = false;
    let mut contract_only = false;

    let mut args = std::env::args().skip(1);
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--skeleton" => {
                skeleton = Some(PathBuf::from(
                    args.next().ok_or("--skeleton requires a path")?,
                ));
            }
            "--atlas" => {
                atlas = Some(PathBuf::from(args.next().ok_or("--atlas requires a path")?));
            }
            "--contract" => {
                contract = PathBuf::from(args.next().ok_or("--contract requires a path")?);
            }
            "--skin" => skin = Some(args.next().ok_or("--skin requires a name")?),
            "--inventory-only" => inventory_only = true,
            "--contract-only" => contract_only = true,
            "--help" | "-h" => {
                println!(
                    "Usage: cargo run --example sena_spine_asset_gate -- [--contract PATH] [--skeleton PATH] [--atlas PATH] [--skin NAME] [--inventory-only] [--contract-only]"
                );
                std::process::exit(0);
            }
            unknown => return Err(format!("unknown argument: {unknown}")),
        }
    }

    Ok(Args {
        skeleton,
        atlas,
        contract,
        skin,
        inventory_only,
        contract_only,
    })
}

fn load_contract(path: &Path) -> Result<AssetContract, String> {
    let text = fs::read_to_string(path)
        .map_err(|error| format!("failed to read contract {}: {error}", path.display()))?;
    serde_json::from_str(&text)
        .map_err(|error| format!("failed to parse contract {}: {error}", path.display()))
}

fn validate_contract(contract: &AssetContract) -> Result<(), String> {
    if contract.schema_version != 1 {
        return Err(format!(
            "unsupported R3B contract schema_version {}",
            contract.schema_version
        ));
    }
    if contract.character.trim().is_empty()
        || contract.spine_editor_major_minor.trim().is_empty()
        || contract.default_skin.trim().is_empty()
    {
        return Err("contract character/version/default_skin must not be empty".into());
    }
    if contract.required_animations.is_empty() || contract.required_bones.is_empty() {
        return Err("contract must define required animations and bones".into());
    }
    ensure_unique("required_animations", &contract.required_animations)?;
    ensure_unique(
        "recommended_next_animations",
        &contract.recommended_next_animations,
    )?;
    ensure_unique("required_bones", &contract.required_bones)?;
    ensure_unique("recommended_next_bones", &contract.recommended_next_bones)?;

    let required_bones = contract.required_bones.iter().collect::<BTreeSet<_>>();
    let parent_keys = contract
        .required_bone_parents
        .keys()
        .collect::<BTreeSet<_>>();
    if required_bones != parent_keys {
        return Err("contract required_bone_parents keys must exactly match required_bones".into());
    }
    if contract
        .required_bone_parents
        .get("root")
        .is_none_or(Option::is_some)
    {
        return Err("contract root bone must have no parent".into());
    }
    for (bone, parent) in &contract.required_bone_parents {
        if bone != "root" && parent.is_none() {
            return Err(format!("contract bone {bone} must declare a parent"));
        }
        if let Some(parent) = parent {
            if !required_bones.contains(&parent) {
                return Err(format!(
                    "contract bone {bone} references non-required parent {parent}"
                ));
            }
            if parent == bone {
                return Err(format!("contract bone {bone} cannot parent itself"));
            }
        }
    }
    ensure_acyclic_bone_contract(contract)?;

    let budget = &contract.geometry_budget;
    if budget.preferred_effective_vertices == 0
        || budget.warning_effective_vertices < budget.preferred_effective_vertices
        || !budget.minimum_setup_width.is_finite()
        || !budget.minimum_setup_height.is_finite()
        || budget.minimum_setup_width <= 0.0
        || budget.minimum_setup_height <= 0.0
    {
        return Err("contract geometry_budget is invalid".into());
    }
    if contract.skeleton_candidates.is_empty() {
        return Err("contract skeleton_candidates must not be empty".into());
    }
    if !contract
        .required_export_files
        .iter()
        .any(|file| file.to_ascii_lowercase().ends_with(".atlas"))
    {
        return Err("contract required_export_files must include an .atlas file".into());
    }
    Ok(())
}

fn ensure_acyclic_bone_contract(contract: &AssetContract) -> Result<(), String> {
    for bone in &contract.required_bones {
        let mut current = Some(bone.as_str());
        let mut visited = BTreeSet::new();

        while let Some(name) = current {
            if !visited.insert(name) {
                return Err(format!(
                    "contract bone hierarchy contains a cycle at {name}"
                ));
            }
            current = contract
                .required_bone_parents
                .get(name)
                .and_then(|parent| parent.as_deref());
        }
    }

    Ok(())
}

fn verify_bone_hierarchy(contract: &AssetContract, bones: &[SpineBoneInfo]) -> Result<(), String> {
    let bone_parents = bones
        .iter()
        .map(|bone| (bone.name.as_str(), bone.parent_name.as_deref()))
        .collect::<BTreeMap<_, _>>();

    let missing_bones = contract
        .required_bones
        .iter()
        .filter(|name| !bone_parents.contains_key(name.as_str()))
        .cloned()
        .collect::<Vec<_>>();
    if !missing_bones.is_empty() {
        return Err(format!(
            "R3B skeleton contract missing bones [{}]",
            missing_bones.join(", ")
        ));
    }

    let wrong_parents = contract
        .required_bone_parents
        .iter()
        .filter_map(|(bone, expected_parent)| {
            let actual_parent = bone_parents.get(bone.as_str()).copied().flatten();
            (actual_parent != expected_parent.as_deref()).then(|| {
                format!(
                    "{}: expected {}, got {}",
                    bone,
                    expected_parent.as_deref().unwrap_or("<root>"),
                    actual_parent.unwrap_or("<root>")
                )
            })
        })
        .collect::<Vec<_>>();

    if wrong_parents.is_empty() {
        Ok(())
    } else {
        Err(format!(
            "R3B skeleton parent hierarchy mismatch [{}]",
            wrong_parents.join("; ")
        ))
    }
}

fn ensure_unique(label: &str, values: &[String]) -> Result<(), String> {
    let unique = values.iter().collect::<BTreeSet<_>>();
    if unique.len() == values.len() {
        Ok(())
    } else {
        Err(format!("contract {label} contains duplicate entries"))
    }
}

fn resolve_skeleton(root: &Path, candidates: &[String]) -> Result<PathBuf, String> {
    for candidate in candidates {
        let path = root.join(candidate);
        if path.is_file() {
            return Ok(path);
        }
    }

    let first = candidates
        .first()
        .ok_or("contract has no skeleton candidates")?;
    Ok(root.join(first))
}

fn resolve_default_atlas(root: &Path, required_files: &[String]) -> Result<PathBuf, String> {
    required_files
        .iter()
        .find(|file| file.to_ascii_lowercase().ends_with(".atlas"))
        .map(|file| root.join(file))
        .ok_or_else(|| "contract has no .atlas required export".into())
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

fn frame_metrics(
    frame: &SpineRenderFrame,
    budget: &GeometryBudget,
) -> Result<FrameMetrics, String> {
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
        || bounds_width < budget.minimum_setup_width
        || bounds_height < budget.minimum_setup_height
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

    fn test_budget() -> GeometryBudget {
        GeometryBudget {
            preferred_effective_vertices: 1500,
            warning_effective_vertices: 2500,
            minimum_setup_width: 64.0,
            minimum_setup_height: 128.0,
        }
    }

    #[test]
    fn empty_frame_is_rejected() {
        assert!(frame_metrics(&SpineRenderFrame::default(), &test_budget()).is_err());
    }

    fn bundled_contract() -> AssetContract {
        let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("pets")
            .join("sena")
            .join("spine")
            .join("settings")
            .join("r3b_contract.json");
        load_contract(&path).expect("load bundled contract")
    }

    fn matching_bones(contract: &AssetContract) -> Vec<SpineBoneInfo> {
        contract
            .required_bone_parents
            .iter()
            .map(|(name, parent_name)| SpineBoneInfo {
                name: name.clone(),
                parent_name: parent_name.clone(),
            })
            .collect()
    }

    #[test]
    fn bundled_contract_is_valid() {
        let contract = bundled_contract();
        validate_contract(&contract).expect("bundled R3B contract should be valid");
        assert_eq!(contract.default_skin, "base");
        assert!(
            contract
                .required_animations
                .iter()
                .any(|name| name == "idle")
        );
        assert!(
            contract
                .required_bones
                .iter()
                .any(|name| name == "bow_root")
        );
        assert_eq!(
            contract
                .required_bone_parents
                .get("eye_l")
                .and_then(|parent| parent.as_deref()),
            Some("face_root")
        );
    }

    #[test]
    fn matching_bone_hierarchy_is_accepted() {
        let contract = bundled_contract();
        let bones = matching_bones(&contract);
        verify_bone_hierarchy(&contract, &bones).expect("matching hierarchy should pass");
    }

    #[test]
    fn wrong_bone_parent_is_rejected() {
        let contract = bundled_contract();
        let mut bones = matching_bones(&contract);
        let eye = bones
            .iter_mut()
            .find(|bone| bone.name == "eye_l")
            .expect("eye_l");
        eye.parent_name = Some("head".into());

        let error = verify_bone_hierarchy(&contract, &bones)
            .expect_err("wrong parent should fail the gate");
        assert!(error.contains("eye_l"));
        assert!(error.contains("face_root"));
    }

    #[test]
    fn cyclic_contract_is_rejected() {
        let mut contract = bundled_contract();
        contract
            .required_bone_parents
            .insert("body_root".into(), Some("skirt_root".into()));
        contract
            .required_bone_parents
            .insert("skirt_root".into(), Some("body_root".into()));

        let error = validate_contract(&contract).expect_err("cycle should fail");
        assert!(error.contains("cycle"));
    }
}
