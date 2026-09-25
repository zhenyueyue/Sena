use std::{
    fs,
    path::{Path, PathBuf},
};

fn main() {
    compile_spine_runtime();
    slint_build::compile("ui/pet.slint").expect("failed to compile Slint UI");

    println!("cargo:rerun-if-changed=assets/icons/app.ico");
    println!("cargo:rerun-if-changed=assets/icons/tray.ico");

    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() != Ok("windows") {
        return;
    }

    let app_icon = Path::new("assets/icons/app.ico");
    let tray_icon = Path::new("assets/icons/tray.ico");

    if !app_icon.exists() && !tray_icon.exists() {
        println!(
            "cargo:warning=Sena icon assets are missing; using the Windows default icon for this build"
        );
        return;
    }

    let mut resource = winresource::WindowsResource::new();

    if app_icon.exists() {
        resource.set_icon_with_id("assets/icons/app.ico", "1");
    }

    if tray_icon.exists() {
        resource.set_icon_with_id("assets/icons/tray.ico", "2");
    }

    resource
        .compile()
        .expect("failed to compile Sena Windows icon resources");
}

fn compile_spine_runtime() {
    let runtime_root = Path::new("third_party/spine-runtimes/spine-c/spine-c");
    let source_dir = runtime_root.join("src").join("spine");
    let include_dir = runtime_root.join("include");
    let bridge = Path::new("native/spine_bridge/sena_spine_bridge.c");

    if !source_dir.is_dir() {
        panic!("Spine runtime submodule is missing. Run: git submodule update --init --recursive");
    }

    let mut sources = fs::read_dir(&source_dir)
        .expect("failed to enumerate spine-c sources")
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|path| path.extension().and_then(|ext| ext.to_str()) == Some("c"))
        .collect::<Vec<PathBuf>>();
    sources.sort();

    println!("cargo:rerun-if-changed={}", source_dir.display());
    println!("cargo:rerun-if-changed={}", include_dir.display());
    println!("cargo:rerun-if-changed={}", bridge.display());

    let mut build = cc::Build::new();
    build
        .include(include_dir)
        .files(sources)
        .file(bridge)
        .define("_CRT_SECURE_NO_WARNINGS", None)
        .warnings(false);

    build.compile("sena_spine_runtime");
}
