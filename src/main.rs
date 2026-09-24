mod behavior;
mod context;
mod pet;
mod platform;
mod preferences;
mod render;

use std::{
    sync::{
        Arc, Mutex,
        atomic::{AtomicU64, Ordering},
    },
    time::Duration,
};

use behavior::BehaviorEngine;
use context::{DesktopContext, MediaState, UserActivity};
use preferences::PreferencesStore;
use slint::ComponentHandle;

slint::include_modules!();

fn main() -> Result<(), slint::PlatformError> {
    let window = PetWindow::new()?;

    // No timer is started here. Slint's event loop sleeps while Sena
    // is idle and wakes only when the window system has work to process.
    let context = Arc::new(Mutex::new(DesktopContext::default()));
    let preferences = Arc::new(Mutex::new(PreferencesStore::load_default()));

    let initial_preferences = preferences
        .lock()
        .expect("preferences lock poisoned")
        .value()
        .clone();
    render::set_user_scale(initial_preferences.scale);

    #[cfg(target_os = "windows")]
    let typing_generation = Arc::new(AtomicU64::new(0));
    #[cfg(target_os = "windows")]
    let music_generation = Arc::new(AtomicU64::new(0));
    #[cfg(target_os = "windows")]
    let drowsy_generation = Arc::new(AtomicU64::new(0));
    #[cfg(target_os = "windows")]
    let sleeping_generation = Arc::new(AtomicU64::new(0));

    render::install(&window);

    #[cfg(target_os = "windows")]
    {
        if let Some(process) = platform::windows::current_foreground_process() {
            context
                .lock()
                .expect("desktop context lock poisoned")
                .foreground_process = Some(process);
        }

        refresh_user_activity(&context);
    }

    render_current_context(&window, &context);

    #[cfg(target_os = "windows")]
    if let Some((x, y)) = initial_preferences.position() {
        let window = window.as_weak();
        let preferences = Arc::clone(&preferences);
        slint::Timer::single_shot(Duration::from_millis(250), move || {
            let Some(window) = window.upgrade() else {
                return;
            };

            pet::restore_position(&window, slint::PhysicalPosition::new(x, y));
            let position = window.window().position();

            if position.x != x || position.y != y {
                let mut preferences = preferences.lock().expect("preferences lock poisoned");
                preferences.set_position(position.x, position.y);
                if let Err(error) = preferences.save() {
                    eprintln!("failed to save clamped Sena position: {error}");
                }
            }
        });
    }

    #[cfg(target_os = "windows")]
    {
        let preferences = Arc::clone(&preferences);
        pet::install_motion(&window, move |position| {
            let mut preferences = preferences.lock().expect("preferences lock poisoned");
            preferences.set_position(position.x, position.y);
            if let Err(error) = preferences.save() {
                eprintln!("failed to save Sena preferences: {error}");
            }
        });
    }

    #[cfg(target_os = "windows")]
    let _tray_icon = {
        let window = window.as_weak();
        let context = Arc::clone(&context);
        let preferences = Arc::clone(&preferences);

        match platform::windows::TrayIcon::start(move |action| {
            let window = window.clone();
            let context = Arc::clone(&context);
            let preferences = Arc::clone(&preferences);

            let _ = slint::invoke_from_event_loop(move || match action {
                platform::windows::TrayAction::Show => {
                    if let Some(window) = window.upgrade() {
                        let _ = window.show();
                    }
                }
                platform::windows::TrayAction::Hide => {
                    if let Some(window) = window.upgrade() {
                        let _ = window.hide();
                    }
                }
                platform::windows::TrayAction::SetScale(scale) => {
                    render::set_user_scale(scale);

                    if let Some(window) = window.upgrade() {
                        render_current_context(&window, &context);
                        let position = pet::clamp_current_position(&window);

                        let mut preferences =
                            preferences.lock().expect("preferences lock poisoned");
                        preferences.set_scale(scale);
                        preferences.set_position(position.x, position.y);
                        if let Err(error) = preferences.save() {
                            eprintln!("failed to save Sena preferences: {error}");
                        }
                    }
                }
                platform::windows::TrayAction::Exit => {
                    let _ = slint::quit_event_loop();
                }
            });
        }) {
            Ok(tray) => Some(tray),
            Err(error) => {
                eprintln!("system tray unavailable: {error}");
                None
            }
        }
    };

    #[cfg(target_os = "windows")]
    {
        let window = window.as_weak();
        let context = Arc::clone(&context);
        let drowsy_generation = Arc::clone(&drowsy_generation);
        let sleeping_generation = Arc::clone(&sleeping_generation);
        slint::Timer::single_shot(Duration::from_millis(1), move || {
            restart_drowsy_motion(window.clone(), Arc::clone(&context), drowsy_generation);
            restart_sleeping_motion(window, context, sleeping_generation);
        });
    }

    #[cfg(target_os = "windows")]
    let _foreground_hook = {
        let window = window.as_weak();
        let context = Arc::clone(&context);
        let typing_generation = Arc::clone(&typing_generation);

        platform::windows::install_foreground_hook(move |process| {
            let typing_was_cleared = {
                let mut context = context.lock().expect("desktop context lock poisoned");
                context.foreground_process = Some(process);

                if !context.is_coding() && context.typing_active {
                    context.typing_active = false;
                    true
                } else {
                    false
                }
            };

            if typing_was_cleared {
                typing_generation.fetch_add(1, Ordering::Release);
            }

            if let Some(window) = window.upgrade() {
                render_current_context(&window, &context);
            }
        })
        .ok()
    };

    #[cfg(target_os = "windows")]
    let _idle_timer = {
        let timer = slint::Timer::default();
        let window = window.as_weak();
        let context = Arc::clone(&context);
        let drowsy_generation = Arc::clone(&drowsy_generation);
        let sleeping_generation = Arc::clone(&sleeping_generation);

        timer.start(
            slint::TimerMode::Repeated,
            Duration::from_secs(5),
            move || {
                let changed = refresh_user_activity(&context);
                if changed {
                    restart_drowsy_motion(
                        window.clone(),
                        Arc::clone(&context),
                        Arc::clone(&drowsy_generation),
                    );
                    restart_sleeping_motion(
                        window.clone(),
                        Arc::clone(&context),
                        Arc::clone(&sleeping_generation),
                    );
                }
            },
        );

        timer
    };

    #[cfg(target_os = "windows")]
    let _keyboard_watcher = {
        let window = window.as_weak();
        let context = Arc::clone(&context);
        let typing_generation = Arc::clone(&typing_generation);

        match platform::windows::KeyboardWatcher::start(move || {
            let should_handle = {
                let context = context.lock().expect("desktop context lock poisoned");
                context.is_coding() && !context.session_locked
            };

            if !should_handle {
                return;
            }

            let generation = typing_generation.fetch_add(1, Ordering::AcqRel) + 1;
            let window = window.clone();
            let context = Arc::clone(&context);
            let typing_generation = Arc::clone(&typing_generation);

            let _ = slint::invoke_from_event_loop(move || {
                let (valid, changed) = {
                    let mut context = context.lock().expect("desktop context lock poisoned");
                    if !context.is_coding() || context.session_locked {
                        (false, false)
                    } else {
                        let changed =
                            !context.typing_active || context.user_activity != UserActivity::Active;
                        context.typing_active = true;
                        context.user_activity = UserActivity::Active;
                        (true, changed)
                    }
                };

                if !valid {
                    return;
                }

                if changed && let Some(window) = window.upgrade() {
                    render_current_context(&window, &context);
                }

                let timeout_window = window.clone();
                let timeout_context = Arc::clone(&context);
                let timeout_generation = Arc::clone(&typing_generation);
                slint::Timer::single_shot(Duration::from_millis(650), move || {
                    if timeout_generation.load(Ordering::Acquire) != generation {
                        return;
                    }

                    let changed = {
                        let mut context = timeout_context
                            .lock()
                            .expect("desktop context lock poisoned");
                        if context.typing_active {
                            context.typing_active = false;
                            true
                        } else {
                            false
                        }
                    };

                    if changed && let Some(window) = timeout_window.upgrade() {
                        render_current_context(&window, &timeout_context);
                    }
                });
            });
        }) {
            Ok(watcher) => Some(watcher),
            Err(error) => {
                eprintln!("keyboard activity watcher unavailable: {error}");
                None
            }
        }
    };

    #[cfg(target_os = "windows")]
    let _session_watcher = {
        let window = window.as_weak();
        let context = Arc::clone(&context);
        let typing_generation = Arc::clone(&typing_generation);
        let music_generation = Arc::clone(&music_generation);
        let drowsy_generation = Arc::clone(&drowsy_generation);
        let sleeping_generation = Arc::clone(&sleeping_generation);

        platform::windows::SessionWatcher::start(move |locked| {
            let (changed, typing_was_cleared, music_was_cleared, media_playing) = {
                let mut context = context.lock().expect("desktop context lock poisoned");
                let changed = context.session_locked != locked;
                context.session_locked = locked;

                let typing_was_cleared = locked && context.typing_active;
                if typing_was_cleared {
                    context.typing_active = false;
                }

                let music_was_cleared = locked && context.music_motion_active;
                if music_was_cleared {
                    context.music_motion_active = false;
                }

                (
                    changed,
                    typing_was_cleared,
                    music_was_cleared,
                    context.media == MediaState::Playing,
                )
            };

            if typing_was_cleared {
                typing_generation.fetch_add(1, Ordering::Release);
            }

            let changed = changed || typing_was_cleared || music_was_cleared;

            if !changed {
                return;
            }

            let music_token = music_generation.fetch_add(1, Ordering::AcqRel) + 1;
            let window = window.clone();
            let context = Arc::clone(&context);
            let music_generation = Arc::clone(&music_generation);
            let drowsy_generation = Arc::clone(&drowsy_generation);
            let sleeping_generation = Arc::clone(&sleeping_generation);
            let _ = slint::invoke_from_event_loop(move || {
                restart_drowsy_motion(window.clone(), Arc::clone(&context), drowsy_generation);
                restart_sleeping_motion(window.clone(), Arc::clone(&context), sleeping_generation);

                if !locked && media_playing {
                    schedule_music_motion(window, context, music_generation, music_token, 0);
                }
            });
        })
        .ok()
    };

    #[cfg(target_os = "windows")]
    let _media_watcher = {
        let window = window.as_weak();
        let context = Arc::clone(&context);
        let music_generation = Arc::clone(&music_generation);
        let drowsy_generation = Arc::clone(&drowsy_generation);
        let sleeping_generation = Arc::clone(&sleeping_generation);

        platform::windows::MediaWatcher::start(move |media| {
            let token = music_generation.fetch_add(1, Ordering::AcqRel) + 1;
            let should_schedule = {
                let mut context = context.lock().expect("desktop context lock poisoned");
                context.media = media;
                context.music_motion_active = false;
                media == MediaState::Playing && !context.session_locked
            };

            let window = window.clone();
            let context = Arc::clone(&context);
            let music_generation = Arc::clone(&music_generation);
            let drowsy_generation = Arc::clone(&drowsy_generation);
            let sleeping_generation = Arc::clone(&sleeping_generation);
            let _ = slint::invoke_from_event_loop(move || {
                restart_drowsy_motion(window.clone(), Arc::clone(&context), drowsy_generation);
                restart_sleeping_motion(window.clone(), Arc::clone(&context), sleeping_generation);

                if should_schedule {
                    schedule_music_motion(window, context, music_generation, token, 0);
                }
            });
        })
        .ok()
    };

    window.run()
}

fn sleeping_context_allows_motion(context: &DesktopContext) -> bool {
    context.user_activity == UserActivity::Idle
        && context.media != MediaState::Playing
        && !context.session_locked
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sleeping_motion_requires_unlocked_idle_without_media() {
        let mut context = DesktopContext {
            user_activity: UserActivity::Idle,
            ..DesktopContext::default()
        };

        assert!(sleeping_context_allows_motion(&context));

        context.session_locked = true;
        assert!(!sleeping_context_allows_motion(&context));

        context.session_locked = false;
        context.media = MediaState::Playing;
        assert!(!sleeping_context_allows_motion(&context));

        context.media = MediaState::Stopped;
        context.user_activity = UserActivity::Drowsy;
        assert!(!sleeping_context_allows_motion(&context));
    }
}

#[cfg(target_os = "windows")]
fn restart_sleeping_motion(
    window: slint::Weak<PetWindow>,
    context: Arc<Mutex<DesktopContext>>,
    generation: Arc<AtomicU64>,
) {
    let token = generation.fetch_add(1, Ordering::AcqRel) + 1;
    let should_schedule = {
        let mut context = context.lock().expect("desktop context lock poisoned");
        context.sleeping_motion_active = false;

        sleeping_context_allows_motion(&context)
            && render::has_dedicated_animation(behavior::Behavior::Sleeping)
    };

    if let Some(strong_window) = window.upgrade() {
        render_current_context(&strong_window, &context);
    }

    if should_schedule {
        schedule_sleeping_motion(window, context, generation, token, 0);
    }
}

#[cfg(target_os = "windows")]
fn schedule_sleeping_motion(
    window: slint::Weak<PetWindow>,
    context: Arc<Mutex<DesktopContext>>,
    generation: Arc<AtomicU64>,
    token: u64,
    cycle: u64,
) {
    let rest = match cycle % 4 {
        0 => Duration::from_secs(35),
        1 => Duration::from_secs(52),
        2 => Duration::from_secs(43),
        _ => Duration::from_secs(61),
    };

    slint::Timer::single_shot(rest, move || {
        if generation.load(Ordering::Acquire) != token {
            return;
        }

        let should_start = {
            let mut context = context.lock().expect("desktop context lock poisoned");
            let idle_long_enough = platform::windows::idle_duration()
                .is_some_and(|duration| duration >= Duration::from_secs(10 * 60));

            if !sleeping_context_allows_motion(&context) || !idle_long_enough {
                false
            } else {
                context.sleeping_motion_active = true;
                true
            }
        };

        if !should_start {
            return;
        }

        if let Some(strong_window) = window.upgrade() {
            render_current_context(&strong_window, &context);
        }

        let finish_window = window.clone();
        let finish_context = Arc::clone(&context);
        let finish_generation = Arc::clone(&generation);
        slint::Timer::single_shot(Duration::from_millis(2400), move || {
            if finish_generation.load(Ordering::Acquire) != token {
                return;
            }

            let still_sleeping = {
                let mut context = finish_context
                    .lock()
                    .expect("desktop context lock poisoned");
                context.sleeping_motion_active = false;
                sleeping_context_allows_motion(&context)
            };

            if let Some(strong_window) = finish_window.upgrade() {
                render_current_context(&strong_window, &finish_context);
            }

            if still_sleeping {
                schedule_sleeping_motion(
                    finish_window,
                    finish_context,
                    finish_generation,
                    token,
                    cycle + 1,
                );
            }
        });
    });
}

#[cfg(target_os = "windows")]
fn restart_drowsy_motion(
    window: slint::Weak<PetWindow>,
    context: Arc<Mutex<DesktopContext>>,
    generation: Arc<AtomicU64>,
) {
    let token = generation.fetch_add(1, Ordering::AcqRel) + 1;
    let should_schedule = {
        let mut context = context.lock().expect("desktop context lock poisoned");
        context.drowsy_motion_active = false;

        context.user_activity == UserActivity::Drowsy
            && context.media != MediaState::Playing
            && !context.session_locked
            && render::has_dedicated_animation(behavior::Behavior::Drowsy)
    };

    if let Some(strong_window) = window.upgrade() {
        render_current_context(&strong_window, &context);
    }

    if should_schedule {
        schedule_drowsy_motion(window, context, generation, token, 0);
    }
}

#[cfg(target_os = "windows")]
fn schedule_drowsy_motion(
    window: slint::Weak<PetWindow>,
    context: Arc<Mutex<DesktopContext>>,
    generation: Arc<AtomicU64>,
    token: u64,
    cycle: u64,
) {
    let rest = match cycle % 4 {
        0 => Duration::from_secs(18),
        1 => Duration::from_secs(27),
        2 => Duration::from_secs(22),
        _ => Duration::from_secs(31),
    };

    slint::Timer::single_shot(rest, move || {
        if generation.load(Ordering::Acquire) != token {
            return;
        }

        let should_start = {
            let mut context = context.lock().expect("desktop context lock poisoned");
            let idle_long_enough = platform::windows::idle_duration()
                .is_some_and(|duration| duration >= Duration::from_secs(5 * 60));

            if context.user_activity != UserActivity::Drowsy
                || context.media == MediaState::Playing
                || context.session_locked
                || !idle_long_enough
            {
                false
            } else {
                context.drowsy_motion_active = true;
                true
            }
        };

        if !should_start {
            return;
        }

        if let Some(strong_window) = window.upgrade() {
            render_current_context(&strong_window, &context);
        }

        let finish_window = window.clone();
        let finish_context = Arc::clone(&context);
        let finish_generation = Arc::clone(&generation);
        slint::Timer::single_shot(Duration::from_millis(1600), move || {
            if finish_generation.load(Ordering::Acquire) != token {
                return;
            }

            let still_drowsy = {
                let mut context = finish_context
                    .lock()
                    .expect("desktop context lock poisoned");
                context.drowsy_motion_active = false;
                context.user_activity == UserActivity::Drowsy
                    && context.media != MediaState::Playing
                    && !context.session_locked
            };

            if let Some(strong_window) = finish_window.upgrade() {
                render_current_context(&strong_window, &finish_context);
            }

            if still_drowsy {
                schedule_drowsy_motion(
                    finish_window,
                    finish_context,
                    finish_generation,
                    token,
                    cycle + 1,
                );
            }
        });
    });
}

#[cfg(target_os = "windows")]
fn schedule_music_motion(
    window: slint::Weak<PetWindow>,
    context: Arc<Mutex<DesktopContext>>,
    generation: Arc<AtomicU64>,
    token: u64,
    cycle: u64,
) {
    let rest = match cycle % 4 {
        0 => Duration::from_secs(7),
        1 => Duration::from_secs(11),
        2 => Duration::from_secs(9),
        _ => Duration::from_secs(13),
    };

    slint::Timer::single_shot(rest, move || {
        if generation.load(Ordering::Acquire) != token {
            return;
        }

        let should_start = {
            let mut context = context.lock().expect("desktop context lock poisoned");
            if context.media != MediaState::Playing || context.session_locked {
                false
            } else {
                context.music_motion_active = true;
                true
            }
        };

        if !should_start {
            return;
        }

        if let Some(strong_window) = window.upgrade() {
            render_current_context(&strong_window, &context);
        }

        let finish_window = window.clone();
        let finish_context = Arc::clone(&context);
        let finish_generation = Arc::clone(&generation);
        slint::Timer::single_shot(Duration::from_millis(900), move || {
            if finish_generation.load(Ordering::Acquire) != token {
                return;
            }

            let still_playing = {
                let mut context = finish_context
                    .lock()
                    .expect("desktop context lock poisoned");
                context.music_motion_active = false;
                context.media == MediaState::Playing && !context.session_locked
            };

            if let Some(strong_window) = finish_window.upgrade() {
                render_current_context(&strong_window, &finish_context);
            }

            if still_playing {
                schedule_music_motion(
                    finish_window,
                    finish_context,
                    finish_generation,
                    token,
                    cycle + 1,
                );
            }
        });
    });
}

#[cfg(target_os = "windows")]
fn refresh_user_activity(context: &Arc<Mutex<DesktopContext>>) -> bool {
    let Some(idle_for) = platform::windows::idle_duration() else {
        return false;
    };

    let activity = if idle_for >= Duration::from_secs(10 * 60) {
        UserActivity::Idle
    } else if idle_for >= Duration::from_secs(5 * 60) {
        UserActivity::Drowsy
    } else {
        UserActivity::Active
    };

    let mut context = context.lock().expect("desktop context lock poisoned");

    if context.user_activity == activity {
        false
    } else {
        context.user_activity = activity;
        true
    }
}

fn render_current_context(window: &PetWindow, context: &Arc<Mutex<DesktopContext>>) {
    let context = context
        .lock()
        .expect("desktop context lock poisoned")
        .clone();

    let behavior = BehaviorEngine.resolve(&context);
    render::apply_context(window, &context, behavior);
}
