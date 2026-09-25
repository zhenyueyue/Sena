use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    path::{Path, PathBuf},
};

use serde::Deserialize;

#[derive(Debug, Deserialize)]
struct LayerContract {
    schema_version: u32,
    character: String,
    spine_editor: String,
    naming: String,
    required_groups: BTreeMap<String, Vec<String>>,
    optional_props: Vec<String>,
    forbidden_in_base_setup_pose: Vec<String>,
}

#[derive(Debug, Deserialize)]
struct LayerInventory {
    schema_version: u32,
    source_file: String,
    width: f64,
    height: f64,
    layer_count: usize,
    layers: Vec<InventoryLayer>,
}

#[derive(Debug, Deserialize)]
struct InventoryLayer {
    name: String,
    path: String,
    #[serde(rename = "type")]
    layer_type: String,
    visible: bool,
    opacity: f64,
    bounds: Option<[f64; 4]>,
}

#[derive(Debug)]
struct Args {
    contract: PathBuf,
    inventory: PathBuf,
    contract_only: bool,
}

fn main() {
    if let Err(error) = run() {
        eprintln!("\n[FAIL] Sena source gate: {error}");
        std::process::exit(1);
    }
}

fn run() -> Result<(), String> {
    let args = parse_args()?;
    let contract = load_json::<LayerContract>(&args.contract, "layer contract")?;
    validate_contract(&contract)?;

    println!("Sena PSD Source Contract");
    println!("  contract : {}", args.contract.display());
    println!("  character: {}", contract.character);
    println!("  editor   : {}", contract.spine_editor);
    println!("  naming   : {}", contract.naming);
    println!(
        "  required : {} named layers",
        required_names(&contract).len()
    );

    if args.contract_only {
        println!("\n[PASS] Sena PSD layer contract is valid.");
        return Ok(());
    }

    if !args.inventory.is_file() {
        return Err(format!(
            "layer inventory not found: {}\nOpen sena.psd in Photoshop and run tools/spine/export_psd_layers.jsx first.",
            args.inventory.display()
        ));
    }

    let inventory = load_json::<LayerInventory>(&args.inventory, "layer inventory")?;
    validate_inventory(&contract, &inventory)?;

    println!("\nSena PSD Source Inventory");
    println!("  source   : {}", inventory.source_file);
    println!(
        "  size     : {:.0} x {:.0}",
        inventory.width, inventory.height
    );
    println!("  entries  : {}", inventory.layer_count);

    let all_names = inventory
        .layers
        .iter()
        .map(|layer| layer.name.as_str())
        .collect::<BTreeSet<_>>();
    let required = required_names(&contract);

    let missing = required
        .iter()
        .filter(|name| !all_names.contains(name.as_str()))
        .cloned()
        .collect::<Vec<_>>();
    if !missing.is_empty() {
        return Err(format!(
            "missing required PSD layers [{}]",
            missing.join(", ")
        ));
    }

    let forbidden = contract
        .forbidden_in_base_setup_pose
        .iter()
        .filter(|name| all_names.contains(name.as_str()))
        .cloned()
        .collect::<Vec<_>>();
    if !forbidden.is_empty() {
        return Err(format!(
            "base setup pose contains forbidden layers [{}]",
            forbidden.join(", ")
        ));
    }

    let duplicate_names = duplicate_layer_names(&inventory.layers);
    if !duplicate_names.is_empty() {
        return Err(format!(
            "duplicate layer names are not allowed because Spine attachment mapping becomes ambiguous: [{}]",
            duplicate_names.join(", ")
        ));
    }

    let invalid_names = inventory
        .layers
        .iter()
        .filter(|layer| layer.layer_type == "layer")
        .map(|layer| layer.name.as_str())
        .filter(|name| !is_snake_case(name))
        .map(str::to_string)
        .collect::<Vec<_>>();
    if !invalid_names.is_empty() {
        return Err(format!(
            "art layers must use lower snake_case; invalid names [{}]",
            invalid_names.join(", ")
        ));
    }

    verify_independent_pairs(&all_names)?;
    verify_structure(&all_names)?;

    let zero_area = inventory
        .layers
        .iter()
        .filter(|layer| layer.layer_type == "layer")
        .filter(|layer| layer.bounds.is_none_or(|b| b[2] <= b[0] || b[3] <= b[1]))
        .map(|layer| layer.path.clone())
        .collect::<Vec<_>>();
    if !zero_area.is_empty() {
        return Err(format!(
            "layers with empty/invalid pixel bounds [{}]",
            zero_area.join(", ")
        ));
    }

    let hidden_required = inventory
        .layers
        .iter()
        .filter(|layer| required.contains(&layer.name))
        .filter(|layer| !layer.visible || layer.opacity <= 0.0)
        .map(|layer| layer.name.clone())
        .collect::<Vec<_>>();
    if !hidden_required.is_empty() {
        println!(
            "  [WARN] required layers hidden/transparent in PSD: {}",
            hidden_required.join(", ")
        );
    }

    let optional_present = contract
        .optional_props
        .iter()
        .filter(|name| all_names.contains(name.as_str()))
        .cloned()
        .collect::<Vec<_>>();
    if !optional_present.is_empty() {
        println!(
            "  [INFO] optional prop layers present: {}",
            optional_present.join(", ")
        );
    }

    println!("\nSource checks");
    println!("  [OK] all {} required named layers exist", required.len());
    println!("  [OK] no forbidden base-pose layers");
    println!("  [OK] no duplicate layer names");
    println!("  [OK] art layer names use lower snake_case");
    println!("  [OK] left/right eyes are independently layered");
    println!("  [OK] rear hair is split into multiple deformable masses");
    println!("  [OK] bow wings/tails are independently layered");
    println!("  [OK] skirt front/mid/back are independently layered");
    println!("  [OK] all art layers have non-empty pixel bounds");

    println!("\n[PASS] Sena PSD source satisfies the layer gate.");
    Ok(())
}

fn parse_args() -> Result<Args, String> {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("pets")
        .join("sena")
        .join("spine");

    let mut contract = root.join("settings").join("layer_contract.json");
    let mut inventory = root.join("source").join("sena.layers.json");
    let mut contract_only = false;

    let mut args = std::env::args().skip(1);
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--contract" => {
                contract = PathBuf::from(args.next().ok_or("--contract requires a path")?);
            }
            "--inventory" => {
                inventory = PathBuf::from(args.next().ok_or("--inventory requires a path")?);
            }
            "--contract-only" => contract_only = true,
            "--help" | "-h" => {
                println!(
                    "Usage: cargo run --example sena_source_gate -- [--contract PATH] [--inventory PATH] [--contract-only]"
                );
                std::process::exit(0);
            }
            unknown => return Err(format!("unknown argument: {unknown}")),
        }
    }

    Ok(Args {
        contract,
        inventory,
        contract_only,
    })
}

fn load_json<T: for<'de> Deserialize<'de>>(path: &Path, label: &str) -> Result<T, String> {
    let text = fs::read_to_string(path)
        .map_err(|error| format!("failed to read {label} {}: {error}", path.display()))?;
    serde_json::from_str(&text)
        .map_err(|error| format!("failed to parse {label} {}: {error}", path.display()))
}

fn validate_contract(contract: &LayerContract) -> Result<(), String> {
    if contract.schema_version != 1 {
        return Err(format!(
            "unsupported layer contract schema_version {}",
            contract.schema_version
        ));
    }
    if contract.character != "sena" {
        return Err(format!(
            "layer contract character must be sena, got {}",
            contract.character
        ));
    }
    if contract.naming != "snake_case" {
        return Err(format!(
            "layer contract naming must be snake_case, got {}",
            contract.naming
        ));
    }
    if contract.required_groups.is_empty() {
        return Err("layer contract required_groups must not be empty".into());
    }

    let required = required_names(contract);
    let expected_count: usize = contract.required_groups.values().map(Vec::len).sum();
    if required.len() != expected_count {
        return Err("layer contract contains duplicate required layer names".into());
    }
    if required.iter().any(|name| !is_snake_case(name)) {
        return Err("layer contract contains non-snake_case required names".into());
    }

    Ok(())
}

fn validate_inventory(contract: &LayerContract, inventory: &LayerInventory) -> Result<(), String> {
    if inventory.schema_version != 1 {
        return Err(format!(
            "unsupported layer inventory schema_version {}",
            inventory.schema_version
        ));
    }
    if inventory.source_file.to_ascii_lowercase() != "sena.psd" {
        return Err(format!(
            "expected source file sena.psd, inventory reports {}",
            inventory.source_file
        ));
    }
    if !inventory.width.is_finite()
        || !inventory.height.is_finite()
        || inventory.width < 1024.0
        || inventory.height < 1024.0
    {
        return Err(format!(
            "source canvas is too small or invalid: {:.0} x {:.0}; work source should be high resolution",
            inventory.width, inventory.height
        ));
    }
    if inventory.layer_count != inventory.layers.len() {
        return Err(format!(
            "inventory layer_count mismatch: header={}, actual={}",
            inventory.layer_count,
            inventory.layers.len()
        ));
    }
    if inventory.layers.is_empty() {
        return Err("inventory contains no layers".into());
    }

    validate_contract(contract)
}

fn required_names(contract: &LayerContract) -> BTreeSet<String> {
    contract
        .required_groups
        .values()
        .flat_map(|names| names.iter().cloned())
        .collect()
}

fn duplicate_layer_names(layers: &[InventoryLayer]) -> Vec<String> {
    let mut counts = BTreeMap::<&str, usize>::new();
    for layer in layers.iter().filter(|layer| layer.layer_type == "layer") {
        *counts.entry(&layer.name).or_default() += 1;
    }
    counts
        .into_iter()
        .filter(|(_, count)| *count > 1)
        .map(|(name, _)| name.to_string())
        .collect()
}

fn is_snake_case(name: &str) -> bool {
    if name.is_empty() || name.starts_with('_') || name.ends_with('_') || name.contains("__") {
        return false;
    }

    name.bytes()
        .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'_')
        && name.as_bytes().first().is_some_and(u8::is_ascii_lowercase)
}

fn verify_independent_pairs(names: &BTreeSet<&str>) -> Result<(), String> {
    const PAIRS: &[(&str, &str)] = &[
        ("eye_white_l", "eye_white_r"),
        ("iris_l", "iris_r"),
        ("pupil_l", "pupil_r"),
        ("eye_highlight_l", "eye_highlight_r"),
        ("eyelid_upper_l", "eyelid_upper_r"),
        ("eyelid_lower_l", "eyelid_lower_r"),
        ("lash_l", "lash_r"),
        ("brow_l", "brow_r"),
    ];

    for (left, right) in PAIRS {
        if !names.contains(left) || !names.contains(right) {
            return Err(format!(
                "left/right pair must be independently layered: {left} + {right}"
            ));
        }
    }
    Ok(())
}

fn verify_structure(names: &BTreeSet<&str>) -> Result<(), String> {
    for required in [
        "hair_back_c",
        "hair_back_l1",
        "hair_back_l2",
        "hair_back_r1",
        "hair_back_r2",
        "bow_upper_l",
        "bow_upper_r",
        "bow_lower_l",
        "bow_lower_r",
        "bow_tail_l",
        "bow_tail_r",
        "skirt_back",
        "skirt_mid_l",
        "skirt_mid_r",
        "skirt_front_l",
        "skirt_front_c",
        "skirt_front_r",
    ] {
        if !names.contains(required) {
            return Err(format!("structural layer missing: {required}"));
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn snake_case_rules_are_strict() {
        assert!(is_snake_case("hair_back_l1"));
        assert!(is_snake_case("eye_l"));
        assert!(!is_snake_case("Hair_Back_L1"));
        assert!(!is_snake_case("hair back"));
        assert!(!is_snake_case("_hair"));
        assert!(!is_snake_case("hair__back"));
    }

    #[test]
    fn bundled_layer_contract_is_valid() {
        let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("pets")
            .join("sena")
            .join("spine")
            .join("settings")
            .join("layer_contract.json");
        let contract = load_json::<LayerContract>(&path, "layer contract")
            .expect("load bundled layer contract");
        validate_contract(&contract).expect("bundled layer contract should be valid");

        let names = required_names(&contract);
        assert!(names.contains("eye_white_l"));
        assert!(names.contains("hair_back_r2"));
        assert!(names.contains("bow_tail_l"));
        assert!(names.contains("skirt_front_c"));
    }
}
