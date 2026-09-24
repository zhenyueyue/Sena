mod behavior;
mod context;
mod pet;
mod platform;
mod render;

use std::{
    sync::{Arc, Mutex},
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

        platform::windows::install_foreground_hook(move |process| {
            context
                .lock()
                .expect("desktop context lock poisoned")
                .foreground_process = Some(process);

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
    let _session_watcher = {
        let window = window.as_weak();
        let context = Arc::clone(&context);

        platform::windows::SessionWatcher::start(move |locked| {
            let changed = {
                let mut context = context.lock().expect("desktop context lock poisoned");

                if context.session_locked == locked {
                    false
                } else {
                    context.session_locked = locked;
                    true
                }
            };

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
