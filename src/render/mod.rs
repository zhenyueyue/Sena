mod animation;
mod sprite;

use std::{cell::RefCell, sync::OnceLock};

use animation::{AnimationClip, AnimationSpec};
use slint::{ComponentHandle, LogicalSize, Timer};

use crate::{PetWindow, behavior::Behavior, context::DesktopContext, pet::PetPackage};

static PET_PACKAGE: OnceLock<PetPackage> = OnceLock::new();

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct AlphaRegionKey {
    behavior: Behavior,
    source_width: u32,
    source_height: u32,
    target_width: u32,
    target_height: u32,
}

thread_local! {
    static ALPHA_REGION_KEY: RefCell<Option<AlphaRegionKey>> = const { RefCell::new(None) };
}

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

        let behavior = clip.behavior();
        let frame = frame.max(0) as usize;

        apply_sprite_frame(&window, behavior, frame);

        if let Some(milliseconds) = active_package().animation_frame_duration_ms(behavior, frame) {
            window.set_animation_interval_ms(milliseconds.min(i32::MAX as u64) as i32);
        }
    });

    // The native HWND may not exist during the initial context render. Re-apply
    // the first frame once after the event loop starts so its alpha region is
    // guaranteed to reach Win32 without adding a permanent timer.
    let window_weak = window.as_weak();
    Timer::single_shot(std::time::Duration::from_millis(150), move || {
        let Some(window) = window_weak.upgrade() else {
            return;
        };
        let Some(clip) = AnimationClip::from_i32(window.get_animation_clip()) else {
            return;
        };

        apply_sprite_frame(
            &window,
            clip.behavior(),
            window.get_animation_frame().max(0) as usize,
        );
    });
}

fn apply_sprite_frame(window: &PetWindow, behavior: Behavior, frame: usize) {
    let package = active_package();

    let Some(path) = package.sprite_frame_path(behavior, frame) else {
        use_placeholder(window);
        return;
    };

    let settings = package.sprite_settings();
    let scale_factor = window.window().scale_factor();
    let Some(sprite) = sprite::load_cached(
        &path,
        settings.alpha_threshold,
        settings.scale,
        scale_factor,
    ) else {
        use_placeholder(window);
        return;
    };

    let logical_width = sprite.source_width as f32 * settings.scale;
    let logical_height = sprite.source_height as f32 * settings.scale;
    let target_width = (logical_width * scale_factor).round().max(1.0) as u32;
    let target_height = (logical_height * scale_factor).round().max(1.0) as u32;
    let current_size = window.window().size();

    if current_size.width != target_width || current_size.height != target_height {
        window
            .window()
            .set_size(LogicalSize::new(logical_width, logical_height));
    }

    window.set_sprite_image(sprite.image.clone());
    window.set_use_sprite(true);

    #[cfg(target_os = "windows")]
    {
        use crate::platform::windows::{self, AlphaRegionRect};

        let region_key = AlphaRegionKey {
            behavior,
            source_width: sprite.width,
            source_height: sprite.height,
            target_width,
            target_height,
        };

        let should_update_region = ALPHA_REGION_KEY.with(|current| {
            let mut current = current.borrow_mut();
            if current.as_ref() == Some(&region_key) {
                false
            } else {
                *current = Some(region_key);
                true
            }
        });

        if should_update_region {
            let alpha_rects: Vec<AlphaRegionRect> = sprite
                .alpha_rects
                .iter()
                .map(|rect| AlphaRegionRect {
                    left: rect.left,
                    top: rect.top,
                    right: rect.right,
                    bottom: rect.bottom,
                })
                .collect();

            windows::apply_sprite_alpha_region_if_available(
                &window.window(),
                sprite.width,
                sprite.height,
                &alpha_rects,
            );
        }
    }
}

fn use_placeholder(window: &PetWindow) {
    window.set_use_sprite(false);
    ALPHA_REGION_KEY.with(|current| *current.borrow_mut() = None);

    let scale_factor = window.window().scale_factor();
    let current_size = window.window().size();
    let target_width = (220.0 * scale_factor).round() as u32;
    let target_height = (240.0 * scale_factor).round() as u32;

    if current_size.width != target_width || current_size.height != target_height {
        window.window().set_size(LogicalSize::new(220.0, 240.0));
    }

    #[cfg(target_os = "windows")]
    crate::platform::windows::apply_pet_window_region_if_available(&window.window(), true);
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

    let animation = AnimationSpec::for_runtime(behavior, active_package(), context.typing_active);
    let clip = animation.clip as i32;

    if window.get_animation_clip() != clip {
        window.set_animation_frame(0);
        window.set_animation_clip(clip);
    }

    let mut current_frame = window.get_animation_frame().max(0) as usize;
    if current_frame >= animation.frame_count.max(1) as usize {
        current_frame = 0;
        window.set_animation_frame(0);
    }
    let interval_ms = active_package()
        .animation_frame_duration_ms(behavior, current_frame)
        .map(|milliseconds| milliseconds.min(i32::MAX as u64) as i32)
        .unwrap_or_else(|| animation.interval_ms());

    window.set_animation_frame_count(animation.frame_count);
    window.set_animation_interval_ms(interval_ms);
    window.set_animation_running(animation.running());
    window.set_animation_looping(animation.looping);

    apply_sprite_frame(window, behavior, current_frame);

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
