mod behavior;
mod context;
mod pet;
mod platform;
mod preferences;
mod render;

use std::{
    cell::RefCell,
    rc::Rc,
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, AtomicU64, Ordering},
    },
    time::Duration,
};

use behavior::BehaviorEngine;
use context::{DayPhase, DesktopContext, MediaState, UserActivity};
use preferences::PreferencesStore;
use slint::ComponentHandle;

slint::include_modules!();

thread_local! {
    static SETTINGS_WINDOW: RefCell<Option<SettingsWindow>> = const { RefCell::new(None) };
    static WELCOME_WINDOW: RefCell<Option<WelcomeWindow>> = const { RefCell::new(None) };
}

fn main() -> Result<(), slint::PlatformError> {
    #[cfg(target_os = "windows")]
    let mut single_instance = match platform::windows::SingleInstance::acquire() {
        Ok(platform::windows::SingleInstanceAcquire::Primary(instance)) => Some(instance),
        Ok(platform::windows::SingleInstanceAcquire::ExistingNotified) => return Ok(()),
        Err(error) => {
            eprintln!("single-instance protection unavailable: {error}");
            None
        }
    };

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
    window.set_keep_on_top(initial_preferences.always_on_top);

    let pet_visible = Arc::new(AtomicBool::new(true));

    #[cfg(target_os = "windows")]
    platform::windows::set_tray_menu_state(true, initial_preferences.always_on_top);

    #[cfg(target_os = "windows")]
    if let Some(instance) = single_instance.as_mut() {
        let window = window.as_weak();
        let preferences = Arc::clone(&preferences);
        let pet_visible = Arc::clone(&pet_visible);

        if let Err(error) = instance.start_show_listener(move || {
            let window = window.clone();
            let preferences = Arc::clone(&preferences);
            let pet_visible = Arc::clone(&pet_visible);

            let _ = slint::invoke_from_event_loop(move || {
                pet_visible.store(true, Ordering::Release);

                if let Some(window) = window.upgrade() {
                    let _ = window.show();
                    platform::windows::ensure_window_visible(&window.window());
                }

                with_settings_window(|settings| {
                    settings.set_pet_visible(true);
                });
                sync_tray_menu_state(&preferences, &pet_visible);
            });
        }) {
            eprintln!("single-instance show listener unavailable: {error}");
        }
    }

    #[cfg(target_os = "windows")]
    let typing_generation = Arc::new(AtomicU64::new(0));
    #[cfg(target_os = "windows")]
    let music_generation = Arc::new(AtomicU64::new(0));
    #[cfg(target_os = "windows")]
    let drowsy_generation = Arc::new(AtomicU64::new(0));
    #[cfg(target_os = "windows")]
    let sleeping_generation = Arc::new(AtomicU64::new(0));

    render::install(&window);
    pet::install_interactions(&window, Arc::clone(&context), Arc::clone(&preferences));

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
    {
        let saved_position = initial_preferences.position();
        let window = window.as_weak();
        let preferences = Arc::clone(&preferences);
        slint::Timer::single_shot(Duration::from_millis(250), move || {
            let Some(window) = window.upgrade() else {
                return;
            };

            let position = if let Some((x, y)) = saved_position {
                pet::restore_position(&window, slint::PhysicalPosition::new(x, y));
                window.window().position()
            } else {
                match pet::place_default_position(&window) {
                    Some(position) => position,
                    None => return,
                }
            };

            let should_save =
                saved_position.is_none_or(|(x, y)| position.x != x || position.y != y);

            if should_save {
                let mut preferences = preferences.lock().expect("preferences lock poisoned");
                preferences.set_position(position.x, position.y);
                if let Err(error) = preferences.save() {
                    eprintln!("failed to save Sena position: {error}");
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
    let _pet_context_menu_hook = {
        let hook = Rc::new(RefCell::new(None));
        let hook_slot = Rc::clone(&hook);
        let window = window.as_weak();
        let context = Arc::clone(&context);
        let preferences = Arc::clone(&preferences);
        let pet_visible = Arc::clone(&pet_visible);

        slint::Timer::single_shot(Duration::from_millis(120), move || {
            let Some(strong_window) = window.upgrade() else {
                return;
            };

            let callback_window = window.clone();
            let callback_context = Arc::clone(&context);
            let callback_preferences = Arc::clone(&preferences);
            let callback_pet_visible = Arc::clone(&pet_visible);

            match platform::windows::PetContextMenuHook::install(
                &strong_window.window(),
                move || {
                    let Some(strong_window) = callback_window.upgrade() else {
                        return;
                    };

                    let snapshot = callback_preferences
                        .lock()
                        .expect("preferences lock poisoned")
                        .value()
                        .clone();

                    let Some(action) = platform::windows::show_pet_context_menu(
                        &strong_window.window(),
                        snapshot.scale,
                        snapshot.always_on_top,
                    ) else {
                        return;
                    };

                    handle_desktop_action(
                        action,
                        callback_window.clone(),
                        Arc::clone(&callback_context),
                        Arc::clone(&callback_preferences),
                        Arc::clone(&callback_pet_visible),
                    );
                },
            ) {
                Ok(installed) => {
                    *hook_slot.borrow_mut() = Some(installed);
                }
                Err(error) => {
                    eprintln!("pet context menu unavailable: {error}");
                }
            }
        });

        hook
    };

    #[cfg(target_os = "windows")]
    let _tray_icon = {
        let window = window.as_weak();
        let context = Arc::clone(&context);
        let preferences = Arc::clone(&preferences);
        let pet_visible = Arc::clone(&pet_visible);

        match platform::windows::TrayIcon::start(move |action| {
            let window = window.clone();
            let context = Arc::clone(&context);
            let preferences = Arc::clone(&preferences);
            let pet_visible = Arc::clone(&pet_visible);

            let _ = slint::invoke_from_event_loop(move || {
                handle_desktop_action(action, window, context, preferences, pet_visible);
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

    window.show()?;

    #[cfg(target_os = "windows")]
    if !initial_preferences.onboarding_completed {
        show_welcome_window(Arc::clone(&preferences));
    }

    slint::run_event_loop_until_quit()
}

#[cfg(target_os = "windows")]
fn handle_desktop_action(
    action: platform::windows::TrayAction,
    window: slint::Weak<PetWindow>,
    context: Arc<Mutex<DesktopContext>>,
    preferences: Arc<Mutex<PreferencesStore>>,
    pet_visible: Arc<AtomicBool>,
) {
    match action {
        platform::windows::TrayAction::Settings => {
            show_settings_window(window, context, preferences, pet_visible);
        }
        platform::windows::TrayAction::Show => {
            pet_visible.store(true, Ordering::Release);
            if let Some(window) = window.upgrade() {
                let _ = window.show();
                platform::windows::ensure_window_visible(&window.window());
            }
            with_settings_window(|settings| {
                settings.set_pet_visible(true);
            });
            sync_tray_menu_state(&preferences, &pet_visible);
        }
        platform::windows::TrayAction::Hide => {
            pet_visible.store(false, Ordering::Release);
            if let Some(window) = window.upgrade() {
                let _ = window.hide();
            }
            with_settings_window(|settings| {
                settings.set_pet_visible(false);
            });
            sync_tray_menu_state(&preferences, &pet_visible);
        }
        platform::windows::TrayAction::ToggleVisibility => {
            let visible = !pet_visible.load(Ordering::Acquire);
            pet_visible.store(visible, Ordering::Release);

            if let Some(window) = window.upgrade() {
                if visible {
                    let _ = window.show();
                    platform::windows::ensure_window_visible(&window.window());
                } else {
                    let _ = window.hide();
                }
            }

            with_settings_window(|settings| {
                settings.set_pet_visible(visible);
            });
            sync_tray_menu_state(&preferences, &pet_visible);
        }
        platform::windows::TrayAction::ResetPosition => {
            pet_visible.store(true, Ordering::Release);
            if let Some(window) = window.upgrade() {
                let _ = window.show();
                if let Some(position) = pet::place_default_position(&window) {
                    let mut preferences = preferences.lock().expect("preferences lock poisoned");
                    preferences.set_position(position.x, position.y);
                    if let Err(error) = preferences.save() {
                        eprintln!("failed to save reset Sena position: {error}");
                    }
                }
                platform::windows::ensure_window_visible(&window.window());
            }
            with_settings_window(|settings| {
                settings.set_pet_visible(true);
            });
            sync_tray_menu_state(&preferences, &pet_visible);
        }
        platform::windows::TrayAction::ToggleStartup => {
            let enabled = !platform::windows::startup_enabled();
            let result = platform::windows::set_startup_enabled(enabled);
            let actual = platform::windows::startup_enabled();

            with_settings_window(|settings| {
                settings.set_startup_enabled(actual);
                match &result {
                    Ok(()) => settings.set_status_message("".into()),
                    Err(error) => {
                        settings.set_status_message(format!("开机自启设置失败：{error}").into())
                    }
                }
            });

            if let Err(error) = result {
                eprintln!("failed to update Sena startup setting: {error}");
            }
        }
        platform::windows::TrayAction::SetScale(scale) => {
            render::set_user_scale(scale);

            if let Some(window) = window.upgrade() {
                render_current_context(&window, &context);
                let position = pet::clamp_current_position(&window);

                let mut preferences = preferences.lock().expect("preferences lock poisoned");
                preferences.set_scale(scale);
                preferences.set_position(position.x, position.y);
                if let Err(error) = preferences.save() {
                    eprintln!("failed to save Sena preferences: {error}");
                }
            }

            with_settings_window(|settings| {
                settings.set_scale_percent((scale * 100.0).round() as i32);
            });
        }
        platform::windows::TrayAction::ToggleAlwaysOnTop => {
            let enabled = {
                let mut preferences = preferences.lock().expect("preferences lock poisoned");
                let enabled = !preferences.value().always_on_top;
                preferences.set_always_on_top(enabled);
                if let Err(error) = preferences.save() {
                    eprintln!("failed to save Sena preferences: {error}");
                }
                enabled
            };

            if let Some(window) = window.upgrade() {
                window.set_keep_on_top(enabled);
            }

            with_settings_window(|settings| {
                settings.set_keep_on_top(enabled);
            });
            sync_tray_menu_state(&preferences, &pet_visible);
        }
        platform::windows::TrayAction::Exit => {
            let _ = slint::quit_event_loop();
        }
    }
}

#[cfg(target_os = "windows")]
fn show_welcome_window(preferences: Arc<Mutex<PreferencesStore>>) {
    WELCOME_WINDOW.with(|slot| {
        if slot.borrow().is_none() {
            let welcome = match WelcomeWindow::new() {
                Ok(welcome) => welcome,
                Err(error) => {
                    eprintln!("failed to create Sena welcome window: {error}");
                    return;
                }
            };

            let welcome_weak = welcome.as_weak();
            welcome.on_finish_onboarding(move || {
                {
                    let mut preferences = preferences.lock().expect("preferences lock poisoned");
                    preferences.set_onboarding_completed(true);
                    if let Err(error) = preferences.save() {
                        eprintln!("failed to save Sena onboarding state: {error}");
                    }
                }

                if let Some(welcome) = welcome_weak.upgrade() {
                    let _ = welcome.hide();
                }
            });

            *slot.borrow_mut() = Some(welcome);
        }

        if let Some(welcome) = slot.borrow().as_ref() {
            let _ = welcome.show();
        }
    });
}

#[cfg(target_os = "windows")]
fn sync_tray_menu_state(preferences: &Arc<Mutex<PreferencesStore>>, pet_visible: &Arc<AtomicBool>) {
    let always_on_top = preferences
        .lock()
        .expect("preferences lock poisoned")
        .value()
        .always_on_top;
    platform::windows::set_tray_menu_state(pet_visible.load(Ordering::Acquire), always_on_top);
}

#[cfg(target_os = "windows")]
fn with_settings_window(callback: impl FnOnce(&SettingsWindow)) {
    SETTINGS_WINDOW.with(|slot| {
        if let Some(settings) = slot.borrow().as_ref() {
            callback(settings);
        }
    });
}

#[cfg(target_os = "windows")]
fn show_settings_window(
    window: slint::Weak<PetWindow>,
    context: Arc<Mutex<DesktopContext>>,
    preferences: Arc<Mutex<PreferencesStore>>,
    pet_visible: Arc<AtomicBool>,
) {
    SETTINGS_WINDOW.with(|slot| {
        if slot.borrow().is_none() {
            let settings = match SettingsWindow::new() {
                Ok(settings) => settings,
                Err(error) => {
                    eprintln!("failed to create Sena settings window: {error}");
                    return;
                }
            };

            {
                let window = window.clone();
                let context = Arc::clone(&context);
                let preferences = Arc::clone(&preferences);
                let settings_weak = settings.as_weak();

                settings.on_set_scale(move |percent| {
                    let scale = (percent as f32 / 100.0).clamp(0.6, 1.4);
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
                            if let Some(settings) = settings_weak.upgrade() {
                                settings
                                    .set_status_message(format!("保存设置失败：{error}").into());
                            }
                        } else if let Some(settings) = settings_weak.upgrade() {
                            settings.set_status_message("".into());
                        }
                    }
                });
            }

            {
                let settings_weak = settings.as_weak();
                settings.on_set_startup(move |enabled| {
                    let result = platform::windows::set_startup_enabled(enabled);
                    let actual = platform::windows::startup_enabled();

                    if let Some(settings) = settings_weak.upgrade() {
                        settings.set_startup_enabled(actual);
                        match result {
                            Ok(()) => settings.set_status_message("".into()),
                            Err(error) => settings
                                .set_status_message(format!("开机自启设置失败：{error}").into()),
                        }
                    }
                });
            }

            {
                let window = window.clone();
                let preferences = Arc::clone(&preferences);
                let pet_visible = Arc::clone(&pet_visible);
                let settings_weak = settings.as_weak();

                settings.on_set_keep_on_top(move |enabled| {
                    if let Some(window) = window.upgrade() {
                        window.set_keep_on_top(enabled);
                    }

                    {
                        let mut store = preferences.lock().expect("preferences lock poisoned");
                        store.set_always_on_top(enabled);

                        if let Err(error) = store.save() {
                            eprintln!("failed to save Sena preferences: {error}");
                            if let Some(settings) = settings_weak.upgrade() {
                                settings
                                    .set_status_message(format!("保存设置失败：{error}").into());
                            }
                        } else if let Some(settings) = settings_weak.upgrade() {
                            settings.set_status_message("".into());
                        }
                    }

                    sync_tray_menu_state(&preferences, &pet_visible);
                });
            }

            {
                let window = window.clone();
                let preferences = Arc::clone(&preferences);
                let pet_visible = Arc::clone(&pet_visible);

                settings.on_set_visible(move |visible| {
                    pet_visible.store(visible, Ordering::Release);

                    if let Some(window) = window.upgrade() {
                        if visible {
                            let _ = window.show();
                            platform::windows::ensure_window_visible(&window.window());
                        } else {
                            let _ = window.hide();
                        }
                    }

                    sync_tray_menu_state(&preferences, &pet_visible);
                });
            }

            {
                let window = window.clone();
                let preferences = Arc::clone(&preferences);
                let settings_weak = settings.as_weak();

                settings.on_set_autonomous_behavior(move |enabled| {
                    if let Some(window) = window.upgrade()
                        && !enabled
                    {
                        window.set_autonomous_reaction_active(false);
                        window.set_autonomous_reaction_phase(0);
                    }

                    let result = {
                        let mut store = preferences.lock().expect("preferences lock poisoned");
                        store.set_autonomous_behavior_enabled(enabled);
                        store.save()
                    };

                    if result.is_ok() {
                        pet::refresh_interaction_settings();
                    }

                    if let Some(settings) = settings_weak.upgrade() {
                        match result {
                            Ok(()) => settings.set_status_message("".into()),
                            Err(error) => {
                                settings.set_status_message(format!("保存设置失败：{error}").into())
                            }
                        }
                    }
                });
            }

            {
                let window = window.clone();
                let preferences = Arc::clone(&preferences);
                let settings_weak = settings.as_weak();

                settings.on_set_speech_bubbles(move |enabled| {
                    if let Some(window) = window.upgrade()
                        && !enabled
                    {
                        window.set_interaction_bubble_visible(false);
                    }

                    let result = {
                        let mut store = preferences.lock().expect("preferences lock poisoned");
                        store.set_speech_bubbles_enabled(enabled);
                        store.save()
                    };

                    if result.is_ok() {
                        pet::refresh_interaction_settings();
                    }

                    if let Some(settings) = settings_weak.upgrade() {
                        match result {
                            Ok(()) => settings.set_status_message("".into()),
                            Err(error) => {
                                settings.set_status_message(format!("保存设置失败：{error}").into())
                            }
                        }
                    }
                });
            }

            {
                let preferences = Arc::clone(&preferences);
                let settings_weak = settings.as_weak();

                settings.on_set_autonomous_frequency(move |frequency| {
                    let result = {
                        let mut store = preferences.lock().expect("preferences lock poisoned");
                        store.set_autonomous_frequency(frequency.clamp(0, 2) as u8);
                        store.save()
                    };

                    if result.is_ok() {
                        pet::refresh_interaction_settings();
                    }

                    if let Some(settings) = settings_weak.upgrade() {
                        match result {
                            Ok(()) => settings.set_status_message("".into()),
                            Err(error) => {
                                settings.set_status_message(format!("保存设置失败：{error}").into())
                            }
                        }
                    }
                });
            }

            {
                let settings_weak = settings.as_weak();
                settings.on_close_settings(move || {
                    if let Some(settings) = settings_weak.upgrade() {
                        let _ = settings.hide();
                    }
                });
            }

            *slot.borrow_mut() = Some(settings);
        }

        let snapshot = preferences
            .lock()
            .expect("preferences lock poisoned")
            .value()
            .clone();

        if let Some(settings) = slot.borrow().as_ref() {
            settings.set_scale_percent((snapshot.scale * 100.0).round() as i32);
            settings.set_keep_on_top(snapshot.always_on_top);
            settings.set_pet_visible(pet_visible.load(Ordering::Acquire));
            settings.set_startup_enabled(platform::windows::startup_enabled());
            settings.set_autonomous_behavior_enabled(snapshot.autonomous_behavior_enabled);
            settings.set_speech_bubbles_enabled(snapshot.speech_bubbles_enabled);
            settings.set_autonomous_frequency(snapshot.autonomous_frequency as i32);
            settings.set_status_message("".into());
            let _ = settings.show();
        }
    });
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
            let (_, sleeping_after) = current_day_phase().idle_threshold_seconds();
            let idle_long_enough = platform::windows::idle_duration()
                .is_some_and(|duration| duration >= Duration::from_secs(sleeping_after));

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
            let (drowsy_after, _) = current_day_phase().idle_threshold_seconds();
            let idle_long_enough = platform::windows::idle_duration()
                .is_some_and(|duration| duration >= Duration::from_secs(drowsy_after));

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
fn current_day_phase() -> DayPhase {
    DayPhase::from_hour(platform::windows::local_hour())
}

#[cfg(target_os = "windows")]
fn refresh_user_activity(context: &Arc<Mutex<DesktopContext>>) -> bool {
    let Some(idle_for) = platform::windows::idle_duration() else {
        return false;
    };

    let (drowsy_after, sleeping_after) = current_day_phase().idle_threshold_seconds();
    let activity = if idle_for >= Duration::from_secs(sleeping_after) {
        UserActivity::Idle
    } else if idle_for >= Duration::from_secs(drowsy_after) {
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
