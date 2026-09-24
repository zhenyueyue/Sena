mod animation;
mod sprite;

use std::sync::OnceLock;

use animation::{AnimationClip, AnimationSpec};
use slint::ComponentHandle;

use crate::{PetWindow, behavior::Behavior, context::DesktopContext, pet::PetPackage};

static PET_PACKAGE: OnceLock<PetPackage> = OnceLock::new();

fn active_package() -> &'static PetPackage {
    PET_PACKAGE.get_or_init(|| match PetPackage::load_default() {
        Ok(package) => {
            let manifest = package.manifest();
            eprintln!(
                "loaded pet package {} ({}) v{} by {} [{}] from {}",
                manifest.display_name.as_deref().unwrap_or(&manifest.name),
                manifest.id,
                manifest.version,
                manifest.author,
                manifest.license.as_deref().unwrap_or("unspecified license"),
                package.root().display(),
            );
            package
        }
        Err(error) => {
            eprintln!("pet package unavailable, using built-in placeholder: {error}");
            PetPackage::builtin_placeholder()
        }
    })
}

pub fn install(window: &PetWindow) {
    let window_weak = window.as_weak();

    window.on_animation_tick(move |clip, frame| {
        let Some(window) = window_weak.upgrade() else {
            return;
        };
        let Some(clip) = AnimationClip::from_i32(clip) else {
            return;
        };

        apply_sprite_frame(&window, clip.behavior(), frame.max(0) as usize);
    });
}

fn apply_sprite_frame(window: &PetWindow, behavior: Behavior, frame: usize) {
    let package = active_package();

    let Some(path) = package.sprite_frame_path(behavior, frame) else {
        window.set_use_sprite(false);
        return;
    };

    let Some(image) = sprite::load_cached(&path) else {
        window.set_use_sprite(false);
        return;
    };

    window.set_sprite_image(image);
    window.set_use_sprite(true);
}

/// Renderer boundary for the current Slint placeholder and Sprite backend.
///
/// The behavior engine only emits semantic states. Sprite and Live2D backends
/// can later map the same states to completely different visuals.
pub fn apply_context(window: &PetWindow, context: &DesktopContext, behavior: Behavior) {
    let label = match behavior {
        Behavior::Idle => "Idle",
        Behavior::Coding => "Coding",
        Behavior::ListeningMusic => "Listening",
        Behavior::CodingWithMusic => "Coding + Music",
        Behavior::Drowsy => "Drowsy",
        Behavior::Sleeping => "Sleeping",
    };

    let animation = AnimationSpec::for_behavior(behavior, active_package());
    let clip = animation.clip as i32;

    if window.get_animation_clip() != clip {
        window.set_animation_frame(0);
        window.set_animation_clip(clip);
    }

    window.set_animation_frame_count(animation.frame_count);
    window.set_animation_interval_ms(animation.interval_ms());
    window.set_animation_running(animation.running());
    window.set_animation_looping(animation.looping);

    apply_sprite_frame(
        window,
        behavior,
        window.get_animation_frame().max(0) as usize,
    );

    window.set_activity_label(label.into());
    window.set_is_drowsy(matches!(behavior, Behavior::Drowsy));
    window.set_is_sleeping(matches!(behavior, Behavior::Sleeping));
    window.set_show_laptop(matches!(
        behavior,
        Behavior::Coding | Behavior::CodingWithMusic
    ));
    window.set_show_headphones(matches!(
        behavior,
        Behavior::ListeningMusic | Behavior::CodingWithMusic
    ));
    window.set_foreground_label(
        context
            .foreground_process
            .as_deref()
            .unwrap_or("Desktop")
            .into(),
    );
}
