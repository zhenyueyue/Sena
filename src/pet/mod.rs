mod motion;
mod package;

pub use package::PetPackage;

#[cfg(target_os = "windows")]
pub use motion::install_motion;
