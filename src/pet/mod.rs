mod interaction;
mod motion;
mod package;

pub use interaction::{install_interactions, refresh_interaction_settings};
pub use package::{InteractionAnimationKey, PetPackage};

#[cfg(target_os = "windows")]
pub use motion::{
    clamp_current_position, install_motion, place_default_position, restore_position,
};
