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

use spine::{
    SpineAnimationInfo, SpineAttachmentType, SpineBlendMode, SpineBoneInfo, SpineRenderFrame,
    SpineRuntime, SpineSlotInfo, SpineTimelineTargetKind, SpineTimelineType,
};

#[derive(Debug, Clone, Deserialize)]
struct GeometryBudget {
    preferred_effective_vertices: usize,
    warning_effective_vertices: usize,
    minimum_setup_width: f32,
    minimum_setup_height: f32,
}

#[derive(Debug, Clone, Deserialize)]
struct ForbiddenTimelineRule {
    target_kind: String,
    target: String,
    timeline_types: Vec<String>,
}

#[derive(Debug, Clone, Deserialize)]
struct AnimationScopeContract {
    intended_track: u32,
    min_duration_seconds: f32,
    max_duration_seconds: f32,
    require_any_timeline: bool,
    allowed_bones: Option<Vec<String>>,
    allowed_slots: Option<Vec<String>>,
    allowed_timeline_types: Option<Vec<String>>,
    forbid_global_timelines: bool,
    forbid_constraint_timelines: bool,
    forbidden_timelines: Vec<ForbiddenTimelineRule>,
}

#[derive(Debug, Clone, Deserialize)]
struct RequiredSlotContract {
    bone: String,
    setup_attachment: String,
    blend: String,
    attachments: BTreeMap<String, Vec<String>>,
}

#[derive(Debug, Clone, Deserialize)]
struct DrawOrderConstraint {
    behind: String,
    front: String,
}

#[derive(Debug, Clone, Deserialize)]
struct AssetContract {
    schema_version: u32,
    character: String,
    spine_editor_major_minor: String,
    default_skin: String,
    required_animations: Vec<String>,
    require_required_attachments_in_default_skin: bool,
    animation_scopes: BTreeMap<String, AnimationScopeContract>,
    recommended_next_animations: Vec<String>,
    required_bones: Vec<String>,
    required_bone_parents: BTreeMap<String, Option<String>>,
    recommended_next_bones: Vec<String>,
    required_slots: BTreeMap<String, RequiredSlotContract>,
    draw_order_constraints: Vec<DrawOrderConstraint>,
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
    let slots = runtime.slots();

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
    println!(
        "  slots    : {}",
        slots
            .iter()
            .map(|slot| format!(
                "#{} {} -> {} / setup={} / blend={}",
                slot.index,
                slot.name,
                slot.bone_name,
                slot.setup_attachment_name.as_deref().unwrap_or("<none>"),
                blend_name(slot.blend_mode)
            ))
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
    verify_animation_scopes(&contract, &runtime, &animations)?;
    verify_required_skin_attachments(&contract, &runtime)?;

    runtime
        .set_skin(&skin)
        .map_err(|error| format!("failed to apply skin {}: {error}", skin))?;
    runtime.update(0.0);

    verify_bone_hierarchy(&contract, &bones)?;
    verify_slot_contract(&contract, &mut runtime, &slots)?;

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
    println!("  [OK] animation duration / timeline isolation scopes");
    println!(
        "  [OK] required attachments are owned by skin {}",
        contract.default_skin
    );
    println!(
        "  [OK] required bones {}",
        contract.required_bones.join(" / ")
    );
    println!("  [OK] required bone parent hierarchy");
    println!(
        "  [OK] {} required slots / attachments / draw-order constraints",
        contract.required_slots.len()
    );
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

    if contract.required_slots.is_empty() {
        return Err("contract required_slots must not be empty".into());
    }
    for (slot_name, slot) in &contract.required_slots {
        if slot_name.trim().is_empty()
            || slot.bone.trim().is_empty()
            || slot.setup_attachment.trim().is_empty()
        {
            return Err("slot name/bone/setup_attachment must not be empty".into());
        }
        if !contract
            .required_bones
            .iter()
            .any(|bone| bone == &slot.bone)
        {
            return Err(format!(
                "slot {slot_name} references non-required bone {}",
                slot.bone
            ));
        }
        if !matches!(
            slot.blend.as_str(),
            "normal" | "additive" | "multiply" | "screen"
        ) {
            return Err(format!(
                "slot {slot_name} uses unsupported blend {}",
                slot.blend
            ));
        }
        if !slot.attachments.contains_key(&slot.setup_attachment) {
            return Err(format!(
                "slot {slot_name} setup attachment {} is not declared in attachments",
                slot.setup_attachment
            ));
        }
        if slot.attachments.is_empty() {
            return Err(format!("slot {slot_name} declares no attachments"));
        }
        for (attachment, allowed_types) in &slot.attachments {
            if attachment.trim().is_empty() || allowed_types.is_empty() {
                return Err(format!(
                    "slot {slot_name} has an empty attachment name/type list"
                ));
            }
            for attachment_type in allowed_types {
                if !matches!(
                    attachment_type.as_str(),
                    "region"
                        | "bounding_box"
                        | "mesh"
                        | "linked_mesh"
                        | "path"
                        | "point"
                        | "clipping"
                ) {
                    return Err(format!(
                        "slot {slot_name} attachment {attachment} uses unsupported type {attachment_type}"
                    ));
                }
            }
        }
    }

    let mut draw_edges = BTreeSet::new();
    for constraint in &contract.draw_order_constraints {
        if constraint.behind == constraint.front {
            return Err(format!(
                "draw order constraint cannot reference the same slot twice: {}",
                constraint.behind
            ));
        }
        if !contract.required_slots.contains_key(&constraint.behind)
            || !contract.required_slots.contains_key(&constraint.front)
        {
            return Err(format!(
                "draw order constraint references unknown slot {} -> {}",
                constraint.behind, constraint.front
            ));
        }
        if !draw_edges.insert((constraint.behind.as_str(), constraint.front.as_str())) {
            return Err(format!(
                "duplicate draw order constraint {} -> {}",
                constraint.behind, constraint.front
            ));
        }
    }
    ensure_acyclic_draw_order(contract)?;

    let required_animation_names = contract.required_animations.iter().collect::<BTreeSet<_>>();
    let scope_names = contract.animation_scopes.keys().collect::<BTreeSet<_>>();
    if required_animation_names != scope_names {
        return Err("contract animation_scopes keys must exactly match required_animations".into());
    }

    for (animation_name, scope) in &contract.animation_scopes {
        if scope.intended_track > 3 {
            return Err(format!(
                "animation {animation_name} intended_track {} is outside 0..=3",
                scope.intended_track
            ));
        }
        if !scope.min_duration_seconds.is_finite()
            || !scope.max_duration_seconds.is_finite()
            || scope.min_duration_seconds <= 0.0
            || scope.max_duration_seconds < scope.min_duration_seconds
        {
            return Err(format!(
                "animation {animation_name} has invalid duration range {:.3}..{:.3}",
                scope.min_duration_seconds, scope.max_duration_seconds
            ));
        }

        if let Some(bones) = &scope.allowed_bones {
            ensure_unique(
                &format!("animation_scopes.{animation_name}.allowed_bones"),
                bones,
            )?;
            for bone in bones {
                if !contract
                    .required_bones
                    .iter()
                    .any(|required| required == bone)
                {
                    return Err(format!(
                        "animation {animation_name} allows unknown/non-required bone {bone}"
                    ));
                }
            }
        }

        if let Some(slots) = &scope.allowed_slots {
            ensure_unique(
                &format!("animation_scopes.{animation_name}.allowed_slots"),
                slots,
            )?;
            for slot in slots {
                if !contract.required_slots.contains_key(slot) {
                    return Err(format!(
                        "animation {animation_name} allows unknown slot {slot}"
                    ));
                }
            }
        }

        if let Some(types) = &scope.allowed_timeline_types {
            ensure_unique(
                &format!("animation_scopes.{animation_name}.allowed_timeline_types"),
                types,
            )?;
            for timeline_type in types {
                if !is_timeline_type_name(timeline_type) {
                    return Err(format!(
                        "animation {animation_name} allows unsupported timeline type {timeline_type}"
                    ));
                }
            }
        }

        for rule in &scope.forbidden_timelines {
            if !matches!(rule.target_kind.as_str(), "bone" | "slot") {
                return Err(format!(
                    "animation {animation_name} forbidden timeline uses unsupported target kind {}",
                    rule.target_kind
                ));
            }
            match rule.target_kind.as_str() {
                "bone"
                    if !contract
                        .required_bones
                        .iter()
                        .any(|bone| bone == &rule.target) =>
                {
                    return Err(format!(
                        "animation {animation_name} forbids unknown bone {}",
                        rule.target
                    ));
                }
                "slot" if !contract.required_slots.contains_key(&rule.target) => {
                    return Err(format!(
                        "animation {animation_name} forbids unknown slot {}",
                        rule.target
                    ));
                }
                _ => {}
            }
            if rule.timeline_types.is_empty() {
                return Err(format!(
                    "animation {animation_name} forbidden timeline rule for {} has no types",
                    rule.target
                ));
            }
            ensure_unique(
                &format!(
                    "animation_scopes.{animation_name}.forbidden_timelines.{}",
                    rule.target
                ),
                &rule.timeline_types,
            )?;
            for timeline_type in &rule.timeline_types {
                if !is_timeline_type_name(timeline_type) {
                    return Err(format!(
                        "animation {animation_name} forbids unsupported timeline type {timeline_type}"
                    ));
                }
            }
        }
    }

    if let (Some(left), Some(right)) = (
        contract.animation_scopes.get("blink_l"),
        contract.animation_scopes.get("blink_r"),
    ) {
        let left_bones = left
            .allowed_bones
            .as_deref()
            .unwrap_or(&[])
            .iter()
            .collect::<BTreeSet<_>>();
        let right_bones = right
            .allowed_bones
            .as_deref()
            .unwrap_or(&[])
            .iter()
            .collect::<BTreeSet<_>>();
        if !left_bones.is_disjoint(&right_bones) {
            return Err("blink_l and blink_r allowed_bones must be disjoint".into());
        }

        let left_slots = left
            .allowed_slots
            .as_deref()
            .unwrap_or(&[])
            .iter()
            .collect::<BTreeSet<_>>();
        let right_slots = right
            .allowed_slots
            .as_deref()
            .unwrap_or(&[])
            .iter()
            .collect::<BTreeSet<_>>();
        if !left_slots.is_disjoint(&right_slots) {
            return Err("blink_l and blink_r allowed_slots must be disjoint".into());
        }
    }

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

fn ensure_acyclic_draw_order(contract: &AssetContract) -> Result<(), String> {
    let mut indegree = contract
        .required_slots
        .keys()
        .map(|name| (name.as_str(), 0usize))
        .collect::<BTreeMap<_, _>>();
    let mut outgoing = BTreeMap::<&str, Vec<&str>>::new();

    for edge in &contract.draw_order_constraints {
        *indegree
            .get_mut(edge.front.as_str())
            .ok_or("draw-order front slot missing from contract")? += 1;
        outgoing
            .entry(edge.behind.as_str())
            .or_default()
            .push(edge.front.as_str());
    }

    let mut ready = indegree
        .iter()
        .filter_map(|(name, degree)| (*degree == 0).then_some(*name))
        .collect::<Vec<_>>();
    let mut visited = 0usize;

    while let Some(name) = ready.pop() {
        visited += 1;
        if let Some(next) = outgoing.get(name) {
            for target in next {
                let degree = indegree
                    .get_mut(target)
                    .ok_or("draw-order target missing from contract")?;
                *degree -= 1;
                if *degree == 0 {
                    ready.push(target);
                }
            }
        }
    }

    if visited == indegree.len() {
        Ok(())
    } else {
        Err("contract draw_order_constraints contain a cycle".into())
    }
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

fn verify_slot_contract(
    contract: &AssetContract,
    runtime: &mut SpineRuntime,
    slots: &[SpineSlotInfo],
) -> Result<(), String> {
    let slot_map = slots
        .iter()
        .map(|slot| (slot.name.as_str(), slot))
        .collect::<BTreeMap<_, _>>();

    let missing = contract
        .required_slots
        .keys()
        .filter(|name| !slot_map.contains_key(name.as_str()))
        .cloned()
        .collect::<Vec<_>>();
    if !missing.is_empty() {
        return Err(format!(
            "R3B slot contract missing slots [{}]",
            missing.join(", ")
        ));
    }

    for (slot_name, required) in &contract.required_slots {
        let actual = slot_map
            .get(slot_name.as_str())
            .copied()
            .ok_or_else(|| format!("slot {slot_name} disappeared from inventory"))?;

        if actual.bone_name != required.bone {
            return Err(format!(
                "slot {slot_name} bone mismatch: expected {}, got {}",
                required.bone, actual.bone_name
            ));
        }
        if actual.setup_attachment_name.as_deref() != Some(required.setup_attachment.as_str()) {
            return Err(format!(
                "slot {slot_name} setup attachment mismatch: expected {}, got {}",
                required.setup_attachment,
                actual.setup_attachment_name.as_deref().unwrap_or("<none>")
            ));
        }
        if blend_name(actual.blend_mode) != required.blend {
            return Err(format!(
                "slot {slot_name} blend mismatch: expected {}, got {}",
                required.blend,
                blend_name(actual.blend_mode)
            ));
        }

        for (attachment_name, allowed_types) in &required.attachments {
            let actual_type = runtime
                .attachment_type(slot_name, attachment_name)
                .map_err(|error| {
                    format!(
                        "slot {slot_name} required attachment {attachment_name} missing: {error}"
                    )
                })?;
            let actual_name = attachment_type_name(actual_type);
            if !allowed_types.iter().any(|allowed| allowed == actual_name) {
                return Err(format!(
                    "slot {slot_name} attachment {attachment_name} type mismatch: expected one of [{}], got {actual_name}",
                    allowed_types.join(", ")
                ));
            }
        }
    }

    for constraint in &contract.draw_order_constraints {
        let behind = slot_map
            .get(constraint.behind.as_str())
            .copied()
            .ok_or_else(|| format!("draw-order slot {} missing", constraint.behind))?;
        let front = slot_map
            .get(constraint.front.as_str())
            .copied()
            .ok_or_else(|| format!("draw-order slot {} missing", constraint.front))?;

        if behind.index >= front.index {
            return Err(format!(
                "draw order mismatch: {} must be behind {} (indices {} >= {})",
                constraint.behind, constraint.front, behind.index, front.index
            ));
        }
    }

    Ok(())
}

fn blend_name(blend: SpineBlendMode) -> &'static str {
    match blend {
        SpineBlendMode::Normal => "normal",
        SpineBlendMode::Additive => "additive",
        SpineBlendMode::Multiply => "multiply",
        SpineBlendMode::Screen => "screen",
    }
}

fn attachment_type_name(attachment_type: SpineAttachmentType) -> &'static str {
    match attachment_type {
        SpineAttachmentType::Region => "region",
        SpineAttachmentType::BoundingBox => "bounding_box",
        SpineAttachmentType::Mesh => "mesh",
        SpineAttachmentType::LinkedMesh => "linked_mesh",
        SpineAttachmentType::Path => "path",
        SpineAttachmentType::Point => "point",
        SpineAttachmentType::Clipping => "clipping",
    }
}

fn verify_required_skin_attachments(
    contract: &AssetContract,
    runtime: &SpineRuntime,
) -> Result<(), String> {
    if !contract.require_required_attachments_in_default_skin {
        return Ok(());
    }

    for (slot_name, slot) in &contract.required_slots {
        for (attachment_name, allowed_types) in &slot.attachments {
            let actual = runtime
                .skin_attachment_type(
                    &contract.default_skin,
                    slot_name,
                    attachment_name,
                )
                .map_err(|error| {
                    format!(
                        "required attachment must belong directly to skin {}: slot={}, attachment={}: {error}",
                        contract.default_skin, slot_name, attachment_name
                    )
                })?;
            let actual_name = attachment_type_name(actual);
            if !allowed_types.iter().any(|allowed| allowed == actual_name) {
                return Err(format!(
                    "skin {} attachment type mismatch: slot={}, attachment={}, expected one of [{}], got {}",
                    contract.default_skin,
                    slot_name,
                    attachment_name,
                    allowed_types.join(", "),
                    actual_name
                ));
            }
        }
    }

    Ok(())
}

fn verify_animation_scopes(
    contract: &AssetContract,
    runtime: &SpineRuntime,
    animations: &[SpineAnimationInfo],
) -> Result<(), String> {
    for (animation_name, scope) in &contract.animation_scopes {
        let animation = animation_info(animations, animation_name).ok_or_else(|| {
            format!("animation scope references missing animation {animation_name}")
        })?;

        if animation.duration_seconds < scope.min_duration_seconds
            || animation.duration_seconds > scope.max_duration_seconds
        {
            return Err(format!(
                "animation {animation_name} duration {:.3}s is outside contract range {:.3}..{:.3}s",
                animation.duration_seconds, scope.min_duration_seconds, scope.max_duration_seconds
            ));
        }

        let timelines = runtime.animation_timelines(animation_name)?;
        if scope.require_any_timeline && timelines.is_empty() {
            return Err(format!(
                "animation {animation_name} must contain at least one timeline"
            ));
        }

        for timeline in &timelines {
            if scope.forbid_global_timelines
                && timeline.target_kind == SpineTimelineTargetKind::Global
            {
                return Err(format!(
                    "animation {animation_name} contains forbidden global timeline {}",
                    timeline_type_name(timeline.timeline_type)
                ));
            }
            if scope.forbid_constraint_timelines
                && timeline.target_kind == SpineTimelineTargetKind::Constraint
            {
                return Err(format!(
                    "animation {animation_name} contains forbidden constraint timeline {} on {}",
                    timeline_type_name(timeline.timeline_type),
                    timeline.target_name.as_deref().unwrap_or("<unknown>")
                ));
            }

            if let Some(allowed_types) = &scope.allowed_timeline_types {
                let actual = timeline_type_name(timeline.timeline_type);
                if !allowed_types.iter().any(|allowed| allowed == actual) {
                    return Err(format!(
                        "animation {animation_name} timeline type {actual} is outside allowed scope [{}]",
                        allowed_types.join(", ")
                    ));
                }
            }

            match timeline.target_kind {
                SpineTimelineTargetKind::Bone => {
                    let target = timeline.target_name.as_deref().ok_or_else(|| {
                        format!(
                            "animation {animation_name} has bone timeline {} without target name",
                            timeline_type_name(timeline.timeline_type)
                        )
                    })?;
                    if let Some(allowed) = &scope.allowed_bones {
                        if !allowed.iter().any(|bone| bone == target) {
                            return Err(format!(
                                "animation {animation_name} touches forbidden bone {target} via {}",
                                timeline_type_name(timeline.timeline_type)
                            ));
                        }
                    }
                }
                SpineTimelineTargetKind::Slot => {
                    let target = timeline.target_name.as_deref().ok_or_else(|| {
                        format!(
                            "animation {animation_name} has slot timeline {} without target name",
                            timeline_type_name(timeline.timeline_type)
                        )
                    })?;
                    if let Some(allowed) = &scope.allowed_slots {
                        if !allowed.iter().any(|slot| slot == target) {
                            return Err(format!(
                                "animation {animation_name} touches forbidden slot {target} via {}",
                                timeline_type_name(timeline.timeline_type)
                            ));
                        }
                    }
                }
                SpineTimelineTargetKind::Global | SpineTimelineTargetKind::Constraint => {}
            }

            for rule in &scope.forbidden_timelines {
                if timeline_target_kind_name(timeline.target_kind) != rule.target_kind {
                    continue;
                }
                if timeline.target_name.as_deref() != Some(rule.target.as_str()) {
                    continue;
                }
                let actual_type = timeline_type_name(timeline.timeline_type);
                if rule
                    .timeline_types
                    .iter()
                    .any(|forbidden| forbidden == actual_type)
                {
                    return Err(format!(
                        "animation {animation_name} contains forbidden {actual_type} timeline on {} {}",
                        rule.target_kind, rule.target
                    ));
                }
            }
        }
    }

    Ok(())
}

fn timeline_type_name(timeline_type: SpineTimelineType) -> &'static str {
    match timeline_type {
        SpineTimelineType::Rotate => "rotate",
        SpineTimelineType::Translate => "translate",
        SpineTimelineType::Scale => "scale",
        SpineTimelineType::Shear => "shear",
        SpineTimelineType::Attachment => "attachment",
        SpineTimelineType::Color => "color",
        SpineTimelineType::Deform => "deform",
        SpineTimelineType::Event => "event",
        SpineTimelineType::DrawOrder => "draw_order",
        SpineTimelineType::IkConstraint => "ik_constraint",
        SpineTimelineType::TransformConstraint => "transform_constraint",
        SpineTimelineType::PathConstraintPosition => "path_constraint_position",
        SpineTimelineType::PathConstraintSpacing => "path_constraint_spacing",
        SpineTimelineType::PathConstraintMix => "path_constraint_mix",
        SpineTimelineType::TwoColor => "two_color",
    }
}

fn timeline_target_kind_name(kind: SpineTimelineTargetKind) -> &'static str {
    match kind {
        SpineTimelineTargetKind::Global => "global",
        SpineTimelineTargetKind::Bone => "bone",
        SpineTimelineTargetKind::Slot => "slot",
        SpineTimelineTargetKind::Constraint => "constraint",
    }
}

fn is_timeline_type_name(value: &str) -> bool {
    matches!(
        value,
        "rotate"
            | "translate"
            | "scale"
            | "shear"
            | "attachment"
            | "color"
            | "deform"
            | "event"
            | "draw_order"
            | "ik_constraint"
            | "transform_constraint"
            | "path_constraint_position"
            | "path_constraint_spacing"
            | "path_constraint_mix"
            | "two_color"
    )
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

    fn timeline_scope_fixture(blink_l_bone: &str, idle_scales_left_eye: bool) -> String {
        let idle_eye = if idle_scales_left_eye {
            r#", "eye_l": { "scale": [
                { "time": 0.0, "x": 1.0, "y": 1.0 },
                { "time": 2.0, "x": 1.0, "y": 0.1 },
                { "time": 4.0, "x": 1.0, "y": 1.0 }
            ] }"#
        } else {
            ""
        };

        format!(
            r#"{{
              "skeleton": {{ "hash": "", "spine": "3.8.75", "width": 0, "height": 0 }},
              "bones": [
                {{ "name": "root" }},
                {{ "name": "eye_l", "parent": "root" }},
                {{ "name": "eye_r", "parent": "root" }}
              ],
              "animations": {{
                "idle": {{
                  "bones": {{
                    "root": {{ "rotate": [
                      {{ "time": 0.0, "angle": 0 }},
                      {{ "time": 2.0, "angle": 1 }},
                      {{ "time": 4.0, "angle": 0 }}
                    ] }}{idle_eye}
                  }}
                }},
                "blink_l": {{
                  "bones": {{
                    "{blink_l_bone}": {{ "scale": [
                      {{ "time": 0.0, "x": 1.0, "y": 1.0 }},
                      {{ "time": 0.1, "x": 1.0, "y": 0.1 }},
                      {{ "time": 0.2, "x": 1.0, "y": 1.0 }}
                    ] }}
                  }}
                }},
                "blink_r": {{
                  "bones": {{
                    "eye_r": {{ "scale": [
                      {{ "time": 0.0, "x": 1.0, "y": 1.0 }},
                      {{ "time": 0.1, "x": 1.0, "y": 0.1 }},
                      {{ "time": 0.2, "x": 1.0, "y": 1.0 }}
                    ] }}
                  }}
                }}
              }}
            }}"#
        )
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
        assert_eq!(contract.required_slots.len(), 60);
        assert_eq!(
            contract
                .required_slots
                .get("mouth")
                .map(|slot| slot.setup_attachment.as_str()),
            Some("mouth_neutral")
        );
        assert_eq!(
            contract
                .required_slots
                .get("bow_glow")
                .map(|slot| slot.blend.as_str()),
            Some("additive")
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

    #[test]
    fn slot_contract_rejects_non_required_bone() {
        let mut contract = bundled_contract();
        contract
            .required_slots
            .get_mut("mouth")
            .expect("mouth slot")
            .bone = "missing_face_bone".into();

        let error = validate_contract(&contract).expect_err("unknown slot bone should fail");
        assert!(error.contains("mouth"));
        assert!(error.contains("missing_face_bone"));
    }

    #[test]
    fn cyclic_draw_order_is_rejected() {
        let mut contract = bundled_contract();
        contract.draw_order_constraints.push(DrawOrderConstraint {
            behind: "eye_highlight_l".into(),
            front: "head_base".into(),
        });

        let error = validate_contract(&contract).expect_err("draw-order cycle should fail");
        assert!(error.contains("draw_order"));
        assert!(error.contains("cycle"));
    }

    #[test]
    fn isolated_left_and_right_blinks_are_accepted() {
        let contract = bundled_contract();
        let runtime = SpineRuntime::from_json(&timeline_scope_fixture("eye_l", false))
            .expect("valid timeline fixture");
        let animations = runtime.animations();

        verify_animation_scopes(&contract, &runtime, &animations)
            .expect("isolated blink timelines should pass");
    }

    #[test]
    fn blink_l_touching_right_eye_is_rejected() {
        let contract = bundled_contract();
        let runtime = SpineRuntime::from_json(&timeline_scope_fixture("eye_r", false))
            .expect("valid timeline fixture");
        let animations = runtime.animations();

        let error = verify_animation_scopes(&contract, &runtime, &animations)
            .expect_err("blink_l must not touch eye_r");
        assert!(error.contains("blink_l"));
        assert!(error.contains("eye_r"));
    }

    #[test]
    fn idle_baking_eye_scale_blink_is_rejected() {
        let contract = bundled_contract();
        let runtime = SpineRuntime::from_json(&timeline_scope_fixture("eye_l", true))
            .expect("valid timeline fixture");
        let animations = runtime.animations();

        let error = verify_animation_scopes(&contract, &runtime, &animations)
            .expect_err("idle must not bake blink scale into eye_l");
        assert!(error.contains("idle"));
        assert!(error.contains("eye_l"));
        assert!(error.contains("scale"));
    }

    #[test]
    fn blink_scope_contracts_are_disjoint() {
        let mut contract = bundled_contract();
        contract
            .animation_scopes
            .get_mut("blink_r")
            .expect("blink_r scope")
            .allowed_bones = Some(vec!["eye_l".into()]);

        let error = validate_contract(&contract)
            .expect_err("left/right blink target sets must remain disjoint");
        assert!(error.contains("blink_l"));
        assert!(error.contains("blink_r"));
        assert!(error.contains("disjoint"));
    }
}
