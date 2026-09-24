mod behavior;
mod context;
mod pet;
mod platform;
mod render;

use std::{
    sync::{
        Arc, Mutex,
        atomic::{AtomicU64, Ordering},
    },
    time::Duration,
};

use behavior::BehaviorEngine;
use context::{DesktopContext, UserActivity};

slint::include_modules!();

fn main() -> Result<(), slint::PlatformError> {
    let window = PetWindow::new()?;

    // No timer is started here. Slint's event loop sleeps while Sena
    // is idle and wakes only when the window system has work to process.
    let context = Arc::new(Mutex::new(DesktopContext::default()));

    #[cfg(target_os = "windows")]
    let typing_generation = Arc::new(AtomicU64::new(0));

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
    pet::install_motion(&window);

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

        timer.start(
            slint::TimerMode::Repeated,
            Duration::from_secs(5),
            move || {
                let changed = refresh_user_activity(&context);
                if changed {
                    if let Some(window) = window.upgrade() {
                        render_current_context(&window, &context);
                    }
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

        platform::windows::SessionWatcher::start(move |locked| {
            let (changed, typing_was_cleared) = {
                let mut context = context.lock().expect("desktop context lock poisoned");
                let changed = context.session_locked != locked;
                context.session_locked = locked;

                let typing_was_cleared = locked && context.typing_active;
                if typing_was_cleared {
                    context.typing_active = false;
                }

                (changed, typing_was_cleared)
            };

            if typing_was_cleared {
                typing_generation.fetch_add(1, Ordering::Release);
            }

            let changed = changed || typing_was_cleared;

            if !changed {
                return;
            }

            let window = window.clone();
            let context = Arc::clone(&context);
            let _ = slint::invoke_from_event_loop(move || {
                if let Some(window) = window.upgrade() {
                    render_current_context(&window, &context);
                }
            });
        })
        .ok()
    };

    #[cfg(target_os = "windows")]
    let _media_watcher = {
        let window = window.as_weak();
        let context = Arc::clone(&context);

        platform::windows::MediaWatcher::start(move |media| {
            {
                context.lock().expect("desktop context lock poisoned").media = media;
            }

            let window = window.clone();
            let context = Arc::clone(&context);
            let _ = slint::invoke_from_event_loop(move || {
                if let Some(window) = window.upgrade() {
                    render_current_context(&window, &context);
                }
            });
        })
        .ok()
    };

    window.run()
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
