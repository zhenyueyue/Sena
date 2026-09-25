use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    path::{Path, PathBuf},
};

use serde::Deserialize;
use serde_json::{Map, Value, json};

#[allow(dead_code)]
#[path = "../src/render/spine/mod.rs"]
mod spine;

use spine::{SpineBlendMode, SpineRuntime};

#[derive(Debug, Clone, Deserialize)]
struct RequiredSlotContract {
    bone: String,
    setup_attachment: String,
    blend: String,
}

#[derive(Debug, Clone, Deserialize)]
struct DrawOrderConstraint {
    behind: String,
    front: String,
}

#[derive(Debug, Clone, Deserialize)]
struct BootstrapContract {
    schema_version: u32,
    character: String,
    spine_editor_major_minor: String,
    spine_editor_version: String,
    default_skin: String,
    required_animations: Vec<String>,
    required_bones: Vec<String>,
    required_bone_parents: BTreeMap<String, Option<String>>,
    required_slots: BTreeMap<String, RequiredSlotContract>,
    bootstrap_slot_order: Vec<String>,
    draw_order_constraints: Vec<DrawOrderConstraint>,
}

#[derive(Debug)]
struct Args {
    contract: PathBuf,
    output: PathBuf,
    check: bool,
    force: bool,
}

fn main() {
    if let Err(error) = run() {
        eprintln!("\n[FAIL] Sena Spine bootstrap: {error}");
        std::process::exit(1);
    }
}

fn run() -> Result<(), String> {
    let args = parse_args()?;
    let contract = load_contract(&args.contract)?;
    validate_contract(&contract)?;

    let value = generate_bootstrap(&contract);
    let generated = format!(
        "{}\n",
        serde_json::to_string_pretty(&value)
            .map_err(|error| format!("failed to serialize bootstrap JSON: {error}"))?
    );

    validate_generated_runtime(&contract, &generated)?;

    if args.check {
        let existing = fs::read_to_string(&args.output).map_err(|error| {
            format!(
                "bootstrap output missing/unreadable {}: {error}\nRun: cargo run --example sena_spine_bootstrap -- --force",
                args.output.display()
            )
        })?;
        if normalize_newlines(&existing) != normalize_newlines(&generated) {
            return Err(format!(
                "bootstrap output is stale: {}\nRegenerate it with: cargo run --example sena_spine_bootstrap -- --force",
                args.output.display()
            ));
        }

        println!("Sena Spine Bootstrap");
        println!("  contract : {}", args.contract.display());
        println!("  output   : {}", args.output.display());
        println!("  bones    : {}", contract.required_bones.len());
        println!("  slots    : {}", contract.required_slots.len());
        println!("  skin     : {}", contract.default_skin);
        println!("  animations: {}", contract.required_animations.join(", "));
        println!("\n[PASS] tracked bootstrap matches the R3B contract.");
        return Ok(());
    }

    if args.output.exists() && !args.force {
        return Err(format!(
            "refusing to overwrite existing bootstrap {}\nUse --force only when regenerating from the versioned contract.",
            args.output.display()
        ));
    }

    if let Some(parent) = args.output.parent() {
        fs::create_dir_all(parent).map_err(|error| {
            format!(
                "failed to create bootstrap directory {}: {error}",
                parent.display()
            )
        })?;
    }
    fs::write(&args.output, generated.as_bytes()).map_err(|error| {
        format!(
            "failed to write bootstrap {}: {error}",
            args.output.display()
        )
    })?;

    println!("Sena Spine Bootstrap");
    println!("  contract : {}", args.contract.display());
    println!("  output   : {}", args.output.display());
    println!(
        "  target   : Spine {} Professional",
        contract.spine_editor_version
    );
    println!("  bones    : {}", contract.required_bones.len());
    println!("  slots    : {}", contract.required_slots.len());
    println!("  skin     : {} (empty scaffold)", contract.default_skin);
    println!(
        "  animations: {} (empty scaffolds)",
        contract.required_animations.join(", ")
    );
    println!(
        "\n[PASS] bootstrap generated. It is intentionally incomplete: add real attachments, mesh weights and animation keys in Spine before running the full Asset Gate."
    );
    Ok(())
}

fn parse_args() -> Result<Args, String> {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("pets")
        .join("sena")
        .join("spine");
    let mut contract = root.join("settings").join("r3b_contract.json");
    let mut output = root.join("project").join("sena.bootstrap.json");
    let mut check = false;
    let mut force = false;

    let mut args = std::env::args().skip(1);
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--contract" => {
                contract = PathBuf::from(args.next().ok_or("--contract requires a path")?);
            }
            "--output" => {
                output = PathBuf::from(args.next().ok_or("--output requires a path")?);
            }
            "--check" => check = true,
            "--force" => force = true,
            "--help" | "-h" => {
                println!(
                    "Usage: cargo run --example sena_spine_bootstrap -- [--contract PATH] [--output PATH] [--check] [--force]"
                );
                std::process::exit(0);
            }
            unknown => return Err(format!("unknown argument: {unknown}")),
        }
    }

    if check && force {
        return Err("--check and --force cannot be used together".into());
    }

    Ok(Args {
        contract,
        output,
        check,
        force,
    })
}

fn load_contract(path: &Path) -> Result<BootstrapContract, String> {
    let text = fs::read_to_string(path)
        .map_err(|error| format!("failed to read contract {}: {error}", path.display()))?;
    serde_json::from_str(&text)
        .map_err(|error| format!("failed to parse contract {}: {error}", path.display()))
}

fn validate_contract(contract: &BootstrapContract) -> Result<(), String> {
    if contract.schema_version != 1 {
        return Err(format!(
            "unsupported R3B contract schema_version {}",
            contract.schema_version
        ));
    }
    if contract.character != "sena" {
        return Err(format!(
            "bootstrap contract character must be sena, got {}",
            contract.character
        ));
    }
    if !contract
        .spine_editor_version
        .starts_with(&contract.spine_editor_major_minor)
    {
        return Err(format!(
            "spine_editor_version {} does not match major/minor {}",
            contract.spine_editor_version, contract.spine_editor_major_minor
        ));
    }
    if contract.default_skin.trim().is_empty()
        || contract.required_bones.is_empty()
        || contract.required_slots.is_empty()
        || contract.required_animations.is_empty()
    {
        return Err("bootstrap contract is missing skin/bones/slots/animations".into());
    }

    ensure_unique("required_bones", &contract.required_bones)?;
    ensure_unique("required_animations", &contract.required_animations)?;
    ensure_unique("bootstrap_slot_order", &contract.bootstrap_slot_order)?;

    let required_bones = contract.required_bones.iter().collect::<BTreeSet<_>>();
    let parent_keys = contract
        .required_bone_parents
        .keys()
        .collect::<BTreeSet<_>>();
    if required_bones != parent_keys {
        return Err("required_bone_parents must exactly cover required_bones".into());
    }

    let mut seen_bones = BTreeSet::new();
    for bone in &contract.required_bones {
        let parent = contract
            .required_bone_parents
            .get(bone)
            .ok_or_else(|| format!("missing parent declaration for bone {bone}"))?;
        if let Some(parent) = parent {
            if !seen_bones.contains(parent) {
                return Err(format!(
                    "required_bones is not parent-first: {bone} appears before parent {parent}"
                ));
            }
        }
        seen_bones.insert(bone);
    }

    let slot_keys = contract.required_slots.keys().collect::<BTreeSet<_>>();
    let slot_order = contract
        .bootstrap_slot_order
        .iter()
        .collect::<BTreeSet<_>>();
    if slot_keys != slot_order {
        let missing = slot_keys
            .difference(&slot_order)
            .map(|name| name.as_str())
            .collect::<Vec<_>>();
        let extra = slot_order
            .difference(&slot_keys)
            .map(|name| name.as_str())
            .collect::<Vec<_>>();
        return Err(format!(
            "bootstrap_slot_order must contain every required slot exactly once; missing=[{}], extra=[{}]",
            missing.join(", "),
            extra.join(", ")
        ));
    }

    let slot_index = contract
        .bootstrap_slot_order
        .iter()
        .enumerate()
        .map(|(index, name)| (name.as_str(), index))
        .collect::<BTreeMap<_, _>>();

    for slot_name in &contract.bootstrap_slot_order {
        let slot = contract
            .required_slots
            .get(slot_name)
            .ok_or_else(|| format!("bootstrap slot {slot_name} missing from required_slots"))?;
        if !required_bones.contains(&slot.bone) {
            return Err(format!(
                "bootstrap slot {slot_name} references unknown bone {}",
                slot.bone
            ));
        }
        if slot.setup_attachment.trim().is_empty() {
            return Err(format!(
                "bootstrap slot {slot_name} has empty setup attachment"
            ));
        }
        if !matches!(
            slot.blend.as_str(),
            "normal" | "additive" | "multiply" | "screen"
        ) {
            return Err(format!(
                "bootstrap slot {slot_name} has unsupported blend {}",
                slot.blend
            ));
        }
    }

    for edge in &contract.draw_order_constraints {
        let behind = *slot_index
            .get(edge.behind.as_str())
            .ok_or_else(|| format!("draw-order behind slot {} missing", edge.behind))?;
        let front = *slot_index
            .get(edge.front.as_str())
            .ok_or_else(|| format!("draw-order front slot {} missing", edge.front))?;
        if behind >= front {
            return Err(format!(
                "bootstrap_slot_order violates draw-order constraint {} < {} ({} >= {})",
                edge.behind, edge.front, behind, front
            ));
        }
    }

    Ok(())
}

fn generate_bootstrap(contract: &BootstrapContract) -> Value {
    let bones = contract
        .required_bones
        .iter()
        .map(|name| {
            let mut bone = Map::new();
            bone.insert("name".into(), Value::String(name.clone()));
            if let Some(parent) = contract
                .required_bone_parents
                .get(name)
                .and_then(|parent| parent.as_ref())
            {
                bone.insert("parent".into(), Value::String(parent.clone()));
            }
            Value::Object(bone)
        })
        .collect::<Vec<_>>();

    let slots = contract
        .bootstrap_slot_order
        .iter()
        .map(|name| {
            let slot = contract
                .required_slots
                .get(name)
                .expect("validated bootstrap slot");

            let mut value = Map::new();
            value.insert("name".into(), Value::String(name.clone()));
            value.insert("bone".into(), Value::String(slot.bone.clone()));
            value.insert(
                "attachment".into(),
                Value::String(slot.setup_attachment.clone()),
            );
            if slot.blend != "normal" {
                value.insert("blend".into(), Value::String(slot.blend.clone()));
            }
            Value::Object(value)
        })
        .collect::<Vec<_>>();

    let animations = contract
        .required_animations
        .iter()
        .map(|name| (name.clone(), Value::Object(Map::new())))
        .collect::<Map<_, _>>();

    json!({
        "skeleton": {
            "hash": "sena-r3b-bootstrap",
            "spine": contract.spine_editor_version,
            "width": 0,
            "height": 0,
            "images": "../source/images/",
            "audio": ""
        },
        "bones": bones,
        "slots": slots,
        "skins": [
            {
                "name": contract.default_skin,
                "attachments": {}
            }
        ],
        "animations": animations
    })
}

fn validate_generated_runtime(contract: &BootstrapContract, generated: &str) -> Result<(), String> {
    let runtime = SpineRuntime::from_json(generated)
        .map_err(|error| format!("generated bootstrap is not valid Spine 3.8 JSON: {error}"))?;

    let version = runtime
        .runtime_version()
        .ok_or("generated bootstrap has no Spine version")?;
    if version != contract.spine_editor_version {
        return Err(format!(
            "generated bootstrap version mismatch: expected {}, got {}",
            contract.spine_editor_version, version
        ));
    }

    let bones = runtime.bones();
    if bones.len() != contract.required_bones.len() {
        return Err(format!(
            "generated bootstrap bone count mismatch: expected {}, got {}",
            contract.required_bones.len(),
            bones.len()
        ));
    }
    for (index, expected) in contract.required_bones.iter().enumerate() {
        let actual = &bones[index];
        let expected_parent = contract
            .required_bone_parents
            .get(expected)
            .and_then(|parent| parent.as_deref());
        if actual.name != *expected || actual.parent_name.as_deref() != expected_parent {
            return Err(format!(
                "generated bootstrap bone mismatch at {index}: expected {}<-{}, got {}<-{}",
                expected,
                expected_parent.unwrap_or("<root>"),
                actual.name,
                actual.parent_name.as_deref().unwrap_or("<root>")
            ));
        }
    }

    let slots = runtime.slots();
    if slots.len() != contract.bootstrap_slot_order.len() {
        return Err(format!(
            "generated bootstrap slot count mismatch: expected {}, got {}",
            contract.bootstrap_slot_order.len(),
            slots.len()
        ));
    }
    for (index, expected_name) in contract.bootstrap_slot_order.iter().enumerate() {
        let expected = contract
            .required_slots
            .get(expected_name)
            .expect("validated slot");
        let actual = &slots[index];
        if actual.name != *expected_name
            || actual.bone_name != expected.bone
            || actual.setup_attachment_name.as_deref() != Some(expected.setup_attachment.as_str())
            || blend_name(actual.blend_mode) != expected.blend
        {
            return Err(format!(
                "generated bootstrap slot mismatch at {index}: expected {} -> {} / setup={} / blend={}, got {} -> {} / setup={} / blend={}",
                expected_name,
                expected.bone,
                expected.setup_attachment,
                expected.blend,
                actual.name,
                actual.bone_name,
                actual.setup_attachment_name.as_deref().unwrap_or("<none>"),
                blend_name(actual.blend_mode)
            ));
        }
    }

    if !runtime
        .skins()
        .iter()
        .any(|skin| skin == &contract.default_skin)
    {
        return Err(format!(
            "generated bootstrap missing skin {}",
            contract.default_skin
        ));
    }

    let animation_names = runtime
        .animations()
        .into_iter()
        .map(|animation| animation.name)
        .collect::<BTreeSet<_>>();
    for animation in &contract.required_animations {
        if !animation_names.contains(animation) {
            return Err(format!(
                "generated bootstrap missing animation scaffold {animation}"
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

fn normalize_newlines(text: &str) -> String {
    text.replace("\r\n", "\n")
}

fn ensure_unique(label: &str, values: &[String]) -> Result<(), String> {
    let unique = values.iter().collect::<BTreeSet<_>>();
    if unique.len() == values.len() {
        Ok(())
    } else {
        Err(format!("{label} contains duplicate entries"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn bundled_contract() -> BootstrapContract {
        let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("pets")
            .join("sena")
            .join("spine")
            .join("settings")
            .join("r3b_contract.json");
        load_contract(&path).expect("load bundled R3B contract")
    }

    #[test]
    fn bundled_contract_generates_runtime_parseable_bootstrap() {
        let contract = bundled_contract();
        validate_contract(&contract).expect("bootstrap contract should be valid");
        let json = serde_json::to_string(&generate_bootstrap(&contract)).expect("serialize");
        validate_generated_runtime(&contract, &json)
            .expect("generated bootstrap should parse in spine-c 3.8");
    }

    #[test]
    fn bootstrap_order_covers_all_required_slots() {
        let contract = bundled_contract();
        let expected = contract.required_slots.keys().collect::<BTreeSet<_>>();
        let actual = contract
            .bootstrap_slot_order
            .iter()
            .collect::<BTreeSet<_>>();
        assert_eq!(expected, actual);
        assert_eq!(contract.bootstrap_slot_order.len(), 60);
    }

    #[test]
    fn bootstrap_order_respects_all_draw_constraints() {
        let contract = bundled_contract();
        validate_contract(&contract).expect("draw-order seed should satisfy constraints");
    }

    #[test]
    fn bootstrap_is_intentionally_incomplete() {
        let contract = bundled_contract();
        let json = generate_bootstrap(&contract);

        assert_eq!(
            json["skins"][0]["attachments"].as_object().map(Map::len),
            Some(0)
        );
        for name in &contract.required_animations {
            assert_eq!(
                json["animations"][name].as_object().map(Map::len),
                Some(0),
                "{name} should remain an empty animation scaffold"
            );
        }
    }

    #[test]
    fn missing_bootstrap_slot_is_rejected() {
        let mut contract = bundled_contract();
        contract.bootstrap_slot_order.pop();

        let error = validate_contract(&contract)
            .expect_err("bootstrap order missing a required slot must fail");
        assert!(error.contains("bootstrap_slot_order"));
        assert!(error.contains("missing"));
    }

    #[test]
    fn bootstrap_draw_order_violation_is_rejected() {
        let mut contract = bundled_contract();
        let rear = contract
            .bootstrap_slot_order
            .iter()
            .position(|name| name == "hair_back_c")
            .expect("hair_back_c");
        let head = contract
            .bootstrap_slot_order
            .iter()
            .position(|name| name == "head_base")
            .expect("head_base");
        contract.bootstrap_slot_order.swap(rear, head);

        let error = validate_contract(&contract)
            .expect_err("bootstrap order violating a relative draw constraint must fail");
        assert!(error.contains("hair_back_c"));
        assert!(error.contains("head_base"));
    }
}
