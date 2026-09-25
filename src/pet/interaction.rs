use std::{
    cell::{Cell, RefCell},
    collections::VecDeque,
    rc::Rc,
    sync::{Arc, Mutex},
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};

use slint::{ComponentHandle, Timer};

use crate::{
    PetWindow,
    behavior::BehaviorEngine,
    context::{DayPhase, DesktopContext, MediaState, UserActivity},
    pet::InteractionAnimationKey,
    preferences::PreferencesStore,
    render,
};

const BUBBLE_DURATION: Duration = Duration::from_millis(2600);
const DOUBLE_CLICK_WINDOW: Duration = Duration::from_millis(280);

const INTERACTION_LINES: &[&str] = &[
    "嗯？我在这里呀 ✦",
    "今天也一起待一会儿吧。",
    "别戳太快，会痒的……",
    "猫猫刚刚好像也看你了。",
    "要记得偶尔休息一下哦。",
    "星奈收到你的招呼啦 ♡",
];

const PETTING_LINES: &[&str] = &[
    "欸嘿……摸摸头也可以啦 ♡",
    "再摸一下也不是不行……",
    "头发要被你揉乱啦～",
    "嗯……这个力度刚刚好。",
];

thread_local! {
    static SETTINGS_REFRESH: RefCell<Option<Box<dyn Fn()>>> = const { RefCell::new(None) };
}

const AUTONOMOUS_LINES: &[&str] = &[
    "唔——稍微伸个懒腰……",
    "猫猫现在在想什么呢？",
    "发会儿呆也不错……",
    "你忙你的，我会安静待着的。",
];

// Higher numbers make an action more likely before cooldown rules are applied.
// The user's frequency setting remains primary; local time only nudges the mix
// toward livelier daytime motion or quieter late-night companionship.
const MORNING_AUTONOMOUS_WEIGHTS: [u64; 4] = [6, 3, 1, 1];
const DAY_AUTONOMOUS_WEIGHTS: [u64; 4] = [4, 3, 2, 1];
const EVENING_AUTONOMOUS_WEIGHTS: [u64; 4] = [2, 3, 3, 2];
const LATE_NIGHT_AUTONOMOUS_WEIGHTS: [u64; 4] = [1, 2, 4, 4];
const AUTONOMOUS_HISTORY_LIMIT: usize = 2;

pub fn install_interactions(
    window: &PetWindow,
    context: Arc<Mutex<DesktopContext>>,
    preferences: Arc<Mutex<PreferencesStore>>,
) {
    let next_line = Rc::new(Cell::new(0usize));
    let next_petting_line = Rc::new(Cell::new(0usize));
    let bubble_generation = Rc::new(Cell::new(0u64));
    let click_generation = Rc::new(Cell::new(0u64));
    let last_click_at = Rc::new(Cell::new(None::<Instant>));
    let last_pet_activity = Rc::new(Cell::new(Instant::now()));
    let random_state = Rc::new(Cell::new(initial_random_seed()));
    let recent_autonomous_actions = Rc::new(RefCell::new(VecDeque::<usize>::new()));
    let schedule_generation = Rc::new(Cell::new(1u64));
    let weak_window = window.as_weak();

    window.on_pet_activity({
        let last_pet_activity = Rc::clone(&last_pet_activity);
        let weak_window = window.as_weak();
        let context = Arc::clone(&context);
        move || {
            last_pet_activity.set(Instant::now());
            if let Some(window) = weak_window.upgrade() {
                render::cancel_interaction_animation(&window);
                window.set_autonomous_reaction_active(false);
                window.set_autonomous_reaction_phase(0);
                restore_context(&window, &context);
            }
        }
    });

    window.on_interact({
        let next_line = Rc::clone(&next_line);
        let next_petting_line = Rc::clone(&next_petting_line);
        let bubble_generation = Rc::clone(&bubble_generation);
        let click_generation = Rc::clone(&click_generation);
        let last_click_at = Rc::clone(&last_click_at);
        let last_pet_activity = Rc::clone(&last_pet_activity);
        let preferences = Arc::clone(&preferences);
        let context = Arc::clone(&context);

        move || {
            let Some(window) = weak_window.upgrade() else {
                return;
            };

            let now = Instant::now();
            let is_double_click = last_click_at
                .get()
                .is_some_and(|previous| is_double_click_interval(now.duration_since(previous)));

            if is_double_click {
                last_click_at.set(None);
                last_pet_activity.set(now);
                click_generation.set(click_generation.get().wrapping_add(1));

                let index = next_petting_line.get();
                next_petting_line.set(index.wrapping_add(1));
                if speech_bubbles_enabled(&preferences) {
                    show_bubble(&window, petting_line(index), Rc::clone(&bubble_generation));
                }
                let _ = play_dedicated_interaction(
                    &window,
                    InteractionAnimationKey::Petting,
                    Arc::clone(&context),
                );
                window.set_interaction_reaction_phase(0);
                window.set_interaction_reaction_active(true);
                return;
            }

            last_click_at.set(Some(now));
            last_pet_activity.set(now);
            let token = click_generation.get().wrapping_add(1);
            click_generation.set(token);

            let weak_window = window.as_weak();
            let next_line = Rc::clone(&next_line);
            let bubble_generation = Rc::clone(&bubble_generation);
            let click_generation = Rc::clone(&click_generation);
            let last_click_at = Rc::clone(&last_click_at);
            let preferences = Arc::clone(&preferences);
            Timer::single_shot(DOUBLE_CLICK_WINDOW, move || {
                if click_generation.get() != token {
                    return;
                }

                last_click_at.set(None);
                let Some(window) = weak_window.upgrade() else {
                    return;
                };

                let index = next_line.get();
                next_line.set(index.wrapping_add(1));
                if speech_bubbles_enabled(&preferences) {
                    show_bubble(
                        &window,
                        interaction_line(index),
                        Rc::clone(&bubble_generation),
                    );
                }
            });
        }
    });

    let initial_frequency = preferences
        .lock()
        .expect("preferences lock poisoned")
        .value()
        .autonomous_frequency;
    let initial_delay =
        next_autonomous_delay(&random_state, initial_frequency, current_day_phase());

    schedule_autonomous_behavior(
        window.as_weak(),
        Arc::clone(&context),
        Arc::clone(&preferences),
        Rc::clone(&last_pet_activity),
        Rc::clone(&bubble_generation),
        Rc::clone(&random_state),
        Rc::clone(&recent_autonomous_actions),
        Rc::clone(&schedule_generation),
        schedule_generation.get(),
        initial_delay,
    );

    SETTINGS_REFRESH.with(|slot| {
        let weak_window = window.as_weak();
        let context = Arc::clone(&context);
        let preferences = Arc::clone(&preferences);
        let last_pet_activity = Rc::clone(&last_pet_activity);
        let bubble_generation = Rc::clone(&bubble_generation);
        let random_state = Rc::clone(&random_state);
        let recent_autonomous_actions = Rc::clone(&recent_autonomous_actions);
        let schedule_generation = Rc::clone(&schedule_generation);

        *slot.borrow_mut() = Some(Box::new(move || {
            let token = schedule_generation.get().wrapping_add(1);
            schedule_generation.set(token);

            let Some(window) = weak_window.upgrade() else {
                return;
            };
            let snapshot = preferences
                .lock()
                .expect("preferences lock poisoned")
                .value()
                .clone();

            if !snapshot.speech_bubbles_enabled {
                window.set_interaction_bubble_visible(false);
                bubble_generation.set(bubble_generation.get().wrapping_add(1));
            }

            if !snapshot.autonomous_behavior_enabled {
                render::cancel_interaction_animation(&window);
                window.set_autonomous_reaction_active(false);
                window.set_autonomous_reaction_phase(0);
                restore_context(&window, &context);
                return;
            }

            last_pet_activity.set(Instant::now());
            let delay = next_autonomous_delay(
                &random_state,
                snapshot.autonomous_frequency,
                current_day_phase(),
            );
            schedule_autonomous_behavior(
                window.as_weak(),
                Arc::clone(&context),
                Arc::clone(&preferences),
                Rc::clone(&last_pet_activity),
                Rc::clone(&bubble_generation),
                Rc::clone(&random_state),
                Rc::clone(&recent_autonomous_actions),
                Rc::clone(&schedule_generation),
                token,
                delay,
            );
        }));
    });
}

pub fn refresh_interaction_settings() {
    SETTINGS_REFRESH.with(|slot| {
        if let Some(refresh) = slot.borrow().as_ref() {
            refresh();
        }
    });
}

fn schedule_autonomous_behavior(
    weak_window: slint::Weak<PetWindow>,
    context: Arc<Mutex<DesktopContext>>,
    preferences: Arc<Mutex<PreferencesStore>>,
    last_pet_activity: Rc<Cell<Instant>>,
    bubble_generation: Rc<Cell<u64>>,
    random_state: Rc<Cell<u64>>,
    recent_autonomous_actions: Rc<RefCell<VecDeque<usize>>>,
    schedule_generation: Rc<Cell<u64>>,
    token: u64,
    delay: Duration,
) {
    Timer::single_shot(delay, move || {
        if schedule_generation.get() != token {
            return;
        }

        let Some(window) = weak_window.upgrade() else {
            return;
        };

        let elapsed = last_pet_activity.get().elapsed();
        let context_snapshot = context
            .lock()
            .expect("desktop context lock poisoned")
            .clone();
        let preference_snapshot = preferences
            .lock()
            .expect("preferences lock poisoned")
            .value()
            .clone();
        let day_phase = current_day_phase();
        let minimum_idle = autonomous_min_idle(preference_snapshot.autonomous_frequency, day_phase);

        if preference_snapshot.autonomous_behavior_enabled
            && elapsed >= minimum_idle
            && autonomous_behavior_allowed(&context_snapshot)
            && !window.get_interaction_animation_active()
            && !window.get_interaction_reaction_active()
            && !window.get_interaction_bubble_visible()
        {
            let action = {
                let recent = recent_autonomous_actions.borrow();
                select_autonomous_action(&random_state, &recent, day_phase)
            };
            remember_autonomous_action(&recent_autonomous_actions, action);
            let animation_key = autonomous_animation_key(action);
            let _ = play_dedicated_interaction(&window, animation_key, Arc::clone(&context));

            window.set_autonomous_action(action as i32);
            window.set_autonomous_reaction_phase(0);
            window.set_autonomous_reaction_active(true);
            if preference_snapshot.speech_bubbles_enabled {
                show_bubble(
                    &window,
                    autonomous_line(action),
                    Rc::clone(&bubble_generation),
                );
            }
            last_pet_activity.set(Instant::now());
        }

        let next_delay = next_autonomous_delay(
            &random_state,
            preference_snapshot.autonomous_frequency,
            current_day_phase(),
        );
        schedule_autonomous_behavior(
            window.as_weak(),
            context,
            preferences,
            last_pet_activity,
            bubble_generation,
            random_state,
            recent_autonomous_actions,
            schedule_generation,
            token,
            next_delay,
        );
    });
}

fn show_bubble(window: &PetWindow, text: &str, bubble_generation: Rc<Cell<u64>>) {
    window.set_interaction_bubble_text(text.into());
    window.set_interaction_bubble_visible(true);

    let token = bubble_generation.get().wrapping_add(1);
    bubble_generation.set(token);

    let weak_window = window.as_weak();
    Timer::single_shot(BUBBLE_DURATION, move || {
        if bubble_generation.get() != token {
            return;
        }

        if let Some(window) = weak_window.upgrade() {
            window.set_interaction_bubble_visible(false);
        }
    });
}

fn play_dedicated_interaction(
    window: &PetWindow,
    key: InteractionAnimationKey,
    context: Arc<Mutex<DesktopContext>>,
) -> bool {
    if !render::has_interaction_animation(key) {
        return false;
    }

    let weak_window = window.as_weak();
    render::play_interaction_animation(window, key, move || {
        if let Some(window) = weak_window.upgrade() {
            restore_context(&window, &context);
        }
    })
}

fn restore_context(window: &PetWindow, context: &Arc<Mutex<DesktopContext>>) {
    let snapshot = context
        .lock()
        .expect("desktop context lock poisoned")
        .clone();
    let behavior = BehaviorEngine.resolve(&snapshot);
    render::apply_context(window, &snapshot, behavior);
}

fn autonomous_animation_key(action: usize) -> InteractionAnimationKey {
    match action % AUTONOMOUS_LINES.len() {
        0 => InteractionAnimationKey::Stretch,
        1 => InteractionAnimationKey::LookAtCat,
        _ => InteractionAnimationKey::Daydream,
    }
}

fn autonomous_behavior_allowed(context: &DesktopContext) -> bool {
    !context.session_locked
        && context.media == MediaState::Stopped
        && context.user_activity == UserActivity::Active
        && !context.typing_active
        && !context.is_coding()
}

fn initial_random_seed() -> u64 {
    let time = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos() as u64;
    time ^ (std::process::id() as u64).rotate_left(17)
}

fn advance_random(state: &Rc<Cell<u64>>) -> u64 {
    let next = state
        .get()
        .wrapping_mul(6_364_136_223_846_793_005)
        .wrapping_add(1_442_695_040_888_963_407);
    state.set(next);
    next
}

fn select_autonomous_action(
    state: &Rc<Cell<u64>>,
    recent: &VecDeque<usize>,
    day_phase: DayPhase,
) -> usize {
    let total_weight: u64 = (0..AUTONOMOUS_LINES.len())
        .map(|action| autonomous_action_weight(action, recent, day_phase))
        .sum();

    if total_weight == 0 {
        return (advance_random(state) as usize) % AUTONOMOUS_LINES.len();
    }

    let mut roll = advance_random(state) % total_weight;
    for action in 0..AUTONOMOUS_LINES.len() {
        let weight = autonomous_action_weight(action, recent, day_phase);
        if roll < weight {
            return action;
        }
        roll -= weight;
    }

    0
}

fn autonomous_action_weight(action: usize, recent: &VecDeque<usize>, day_phase: DayPhase) -> u64 {
    let Some(base_weight) = autonomous_weights(day_phase).get(action).copied() else {
        return 0;
    };
    let family = autonomous_animation_key(action);

    if recent
        .back()
        .is_some_and(|last| autonomous_animation_key(*last) == family)
    {
        return 0;
    }

    if recent
        .iter()
        .rev()
        .nth(1)
        .is_some_and(|previous| autonomous_animation_key(*previous) == family)
    {
        return (base_weight / 2).max(1);
    }

    base_weight
}

fn remember_autonomous_action(history: &Rc<RefCell<VecDeque<usize>>>, action: usize) {
    let mut history = history.borrow_mut();
    history.push_back(action);
    while history.len() > AUTONOMOUS_HISTORY_LIMIT {
        history.pop_front();
    }
}

fn autonomous_weights(day_phase: DayPhase) -> &'static [u64; 4] {
    match day_phase {
        DayPhase::Morning => &MORNING_AUTONOMOUS_WEIGHTS,
        DayPhase::Day => &DAY_AUTONOMOUS_WEIGHTS,
        DayPhase::Evening => &EVENING_AUTONOMOUS_WEIGHTS,
        DayPhase::LateNight => &LATE_NIGHT_AUTONOMOUS_WEIGHTS,
    }
}

fn autonomous_delay_bounds(frequency: u8, day_phase: DayPhase) -> (u64, u64) {
    let base = match frequency.min(2) {
        0 => (180, 300),
        2 => (40, 75),
        _ => (75, 135),
    };

    let scale = |seconds: u64| match day_phase {
        DayPhase::Morning => seconds * 9 / 10,
        DayPhase::Day => seconds,
        DayPhase::Evening => seconds * 6 / 5,
        DayPhase::LateNight => seconds * 8 / 5,
    };

    (scale(base.0).max(1), scale(base.1).max(1))
}

fn autonomous_min_idle(frequency: u8, day_phase: DayPhase) -> Duration {
    Duration::from_secs(autonomous_delay_bounds(frequency, day_phase).0)
}

fn next_autonomous_delay(state: &Rc<Cell<u64>>, frequency: u8, day_phase: DayPhase) -> Duration {
    let (minimum, maximum) = autonomous_delay_bounds(frequency, day_phase);
    let span = maximum - minimum + 1;
    Duration::from_secs(minimum + advance_random(state) % span)
}

fn current_day_phase() -> DayPhase {
    #[cfg(target_os = "windows")]
    {
        DayPhase::from_hour(crate::platform::windows::local_hour())
    }

    #[cfg(not(target_os = "windows"))]
    {
        DayPhase::Day
    }
}

fn speech_bubbles_enabled(preferences: &Arc<Mutex<PreferencesStore>>) -> bool {
    preferences
        .lock()
        .expect("preferences lock poisoned")
        .value()
        .speech_bubbles_enabled
}

fn is_double_click_interval(elapsed: Duration) -> bool {
    elapsed <= DOUBLE_CLICK_WINDOW
}

fn interaction_line(index: usize) -> &'static str {
    INTERACTION_LINES[index % INTERACTION_LINES.len()]
}

fn petting_line(index: usize) -> &'static str {
    PETTING_LINES[index % PETTING_LINES.len()]
}

fn autonomous_line(index: usize) -> &'static str {
    AUTONOMOUS_LINES[index % AUTONOMOUS_LINES.len()]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn interaction_lines_cycle_after_last_entry() {
        assert_eq!(interaction_line(0), INTERACTION_LINES[0]);
        assert_eq!(
            interaction_line(INTERACTION_LINES.len()),
            INTERACTION_LINES[0]
        );
    }

    #[test]
    fn petting_lines_cycle_after_last_entry() {
        assert_eq!(petting_line(0), PETTING_LINES[0]);
        assert_eq!(petting_line(PETTING_LINES.len()), PETTING_LINES[0]);
    }

    #[test]
    fn interaction_lines_are_non_empty() {
        assert!(INTERACTION_LINES.iter().all(|line| !line.trim().is_empty()));
        assert!(PETTING_LINES.iter().all(|line| !line.trim().is_empty()));
    }

    #[test]
    fn double_click_window_accepts_fast_second_click() {
        assert!(is_double_click_interval(Duration::from_millis(180)));
        assert!(is_double_click_interval(DOUBLE_CLICK_WINDOW));
        assert!(!is_double_click_interval(Duration::from_millis(281)));
    }

    #[test]
    fn autonomous_behavior_only_runs_in_quiet_active_context() {
        let mut context = DesktopContext::default();
        assert!(autonomous_behavior_allowed(&context));

        context.media = MediaState::Playing;
        assert!(!autonomous_behavior_allowed(&context));

        context.media = MediaState::Stopped;
        context.user_activity = UserActivity::Drowsy;
        assert!(!autonomous_behavior_allowed(&context));

        context.user_activity = UserActivity::Active;
        context.session_locked = true;
        assert!(!autonomous_behavior_allowed(&context));
    }

    #[test]
    fn autonomous_delay_matches_frequency_ranges() {
        for (frequency, minimum, maximum) in [(0, 180, 300), (1, 75, 135), (2, 40, 75)] {
            let state = Rc::new(Cell::new(1234));
            for _ in 0..32 {
                let delay = next_autonomous_delay(&state, frequency, DayPhase::Day);
                assert!(delay >= Duration::from_secs(minimum));
                assert!(delay <= Duration::from_secs(maximum));
            }
        }
    }

    #[test]
    fn autonomous_delay_slows_down_toward_late_night() {
        assert_eq!(autonomous_delay_bounds(1, DayPhase::Morning), (67, 121));
        assert_eq!(autonomous_delay_bounds(1, DayPhase::Day), (75, 135));
        assert_eq!(autonomous_delay_bounds(1, DayPhase::Evening), (90, 162));
        assert_eq!(autonomous_delay_bounds(1, DayPhase::LateNight), (120, 216));
    }

    #[test]
    fn autonomous_weights_shift_from_motion_to_quiet_at_night() {
        assert_eq!(autonomous_weights(DayPhase::Morning), &[6, 3, 1, 1]);
        assert_eq!(autonomous_weights(DayPhase::Day), &[4, 3, 2, 1]);
        assert_eq!(autonomous_weights(DayPhase::Evening), &[2, 3, 3, 2]);
        assert_eq!(autonomous_weights(DayPhase::LateNight), &[1, 2, 4, 4]);
    }

    #[test]
    fn autonomous_lines_are_non_empty() {
        assert!(AUTONOMOUS_LINES.iter().all(|line| !line.trim().is_empty()));
        assert_eq!(autonomous_line(AUTONOMOUS_LINES.len()), AUTONOMOUS_LINES[0]);
    }

    #[test]
    fn autonomous_weights_prevent_immediate_family_repeats() {
        let mut history = VecDeque::new();
        history.push_back(2);

        assert_eq!(autonomous_action_weight(2, &history, DayPhase::Day), 0);
        assert_eq!(autonomous_action_weight(3, &history, DayPhase::Day), 0);
        assert!(autonomous_action_weight(0, &history, DayPhase::Day) > 0);
        assert!(autonomous_action_weight(1, &history, DayPhase::Day) > 0);
    }

    #[test]
    fn autonomous_weights_reduce_family_seen_two_actions_ago() {
        let mut history = VecDeque::new();
        history.push_back(0);
        history.push_back(1);

        assert_eq!(autonomous_action_weight(0, &history, DayPhase::Day), 2);
        assert_eq!(autonomous_action_weight(1, &history, DayPhase::Day), 0);
        assert_eq!(autonomous_action_weight(2, &history, DayPhase::Day), 2);
        assert_eq!(autonomous_action_weight(3, &history, DayPhase::Day), 1);
    }

    #[test]
    fn weighted_selection_never_repeats_the_last_animation_family() {
        let state = Rc::new(Cell::new(1234));
        let history = Rc::new(RefCell::new(VecDeque::new()));

        for _ in 0..128 {
            let action = {
                let recent = history.borrow();
                select_autonomous_action(&state, &recent, DayPhase::Day)
            };

            if let Some(previous) = history.borrow().back().copied() {
                assert_ne!(
                    autonomous_animation_key(action),
                    autonomous_animation_key(previous)
                );
            }

            remember_autonomous_action(&history, action);
        }
    }

    #[test]
    fn autonomous_history_keeps_only_two_recent_actions() {
        let history = Rc::new(RefCell::new(VecDeque::new()));
        remember_autonomous_action(&history, 0);
        remember_autonomous_action(&history, 1);
        remember_autonomous_action(&history, 2);

        assert_eq!(
            history.borrow().iter().copied().collect::<Vec<_>>(),
            vec![1, 2]
        );
    }

    #[test]
    fn autonomous_actions_map_to_expected_animation_slots() {
        assert_eq!(
            autonomous_animation_key(0),
            InteractionAnimationKey::Stretch
        );
        assert_eq!(
            autonomous_animation_key(1),
            InteractionAnimationKey::LookAtCat
        );
        assert_eq!(
            autonomous_animation_key(2),
            InteractionAnimationKey::Daydream
        );
        assert_eq!(
            autonomous_animation_key(3),
            InteractionAnimationKey::Daydream
        );
    }
}
