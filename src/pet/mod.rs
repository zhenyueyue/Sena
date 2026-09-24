mod motion;
mod package;

pub use package::PetPackage;

#[cfg(target_os = "windows")]
pub use motion::{clamp_current_position, install_motion, restore_position};
