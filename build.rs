use std::path::Path;

fn main() {
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
