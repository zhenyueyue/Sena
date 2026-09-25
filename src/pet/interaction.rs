use std::{
    cell::Cell,
    rc::Rc,
    sync::{Arc, Mutex},
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};

use slint::{ComponentHandle, Timer};

use crate::{
    PetWindow,
    context::{DesktopContext, MediaState, UserActivity},
};

const BUBBLE_DURATION: Duration = Duration::from_millis(2600);
const DOUBLE_CLICK_WINDOW: Duration = Duration::from_millis(280);
const AUTONOMOUS_MIN_IDLE: Duration = Duration::from_secs(75);
const AUTONOMOUS_MIN_DELAY_SECS: u64 = 75;
const AUTONOMOUS_DELAY_SPAN_SECS: u64 = 61;

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

const AUTONOMOUS_LINES: &[&str] = &[
    "唔——稍微伸个懒腰……",
    "猫猫现在在想什么呢？",
    "发会儿呆也不错……",
    "你忙你的，我会安静待着的。",
];

pub fn install_interactions(window: &PetWindow, context: Arc<Mutex<DesktopContext>>) {
    let next_line = Rc::new(Cell::new(0usize));
    let next_petting_line = Rc::new(Cell::new(0usize));
    let bubble_generation = Rc::new(Cell::new(0u64));
    let click_generation = Rc::new(Cell::new(0u64));
    let last_click_at = Rc::new(Cell::new(None::<Instant>));
    let last_pet_activity = Rc::new(Cell::new(Instant::now()));
    let random_state = Rc::new(Cell::new(initial_random_seed()));
    let weak_window = window.as_weak();

    window.on_pet_activity({
        let last_pet_activity = Rc::clone(&last_pet_activity);
        let weak_window = window.as_weak();
        move || {
            last_pet_activity.set(Instant::now());
            if let Some(window) = weak_window.upgrade() {
                window.set_autonomous_reaction_active(false);
                window.set_autonomous_reaction_phase(0);
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
                show_bubble(&window, petting_line(index), Rc::clone(&bubble_generation));
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
                show_bubble(
                    &window,
                    interaction_line(index),
                    Rc::clone(&bubble_generation),
                );
            });
        }
    });

    schedule_autonomous_behavior(
        window.as_weak(),
        context,
        last_pet_activity,
        bubble_generation,
        random_state,
        AUTONOMOUS_MIN_IDLE,
    );
}

fn schedule_autonomous_behavior(
    weak_window: slint::Weak<PetWindow>,
    context: Arc<Mutex<DesktopContext>>,
    last_pet_activity: Rc<Cell<Instant>>,
    bubble_generation: Rc<Cell<u64>>,
    random_state: Rc<Cell<u64>>,
    delay: Duration,
) {
    Timer::single_shot(delay, move || {
        let Some(window) = weak_window.upgrade() else {
            return;
        };

        let elapsed = last_pet_activity.get().elapsed();
        let context_snapshot = context
            .lock()
            .expect("desktop context lock poisoned")
            .clone();

        if elapsed >= AUTONOMOUS_MIN_IDLE
            && autonomous_behavior_allowed(&context_snapshot)
            && !window.get_interaction_reaction_active()
            && !window.get_interaction_bubble_visible()
        {
            let action = next_random_index(&random_state, AUTONOMOUS_LINES.len());
            window.set_autonomous_action(action as i32);
            window.set_autonomous_reaction_phase(0);
            window.set_autonomous_reaction_active(true);
            show_bubble(
                &window,
                autonomous_line(action),
                Rc::clone(&bubble_generation),
            );
            last_pet_activity.set(Instant::now());
        }

        let next_delay = next_autonomous_delay(&random_state);
        schedule_autonomous_behavior(
            window.as_weak(),
            context,
            last_pet_activity,
            bubble_generation,
            random_state,
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

fn next_random_index(state: &Rc<Cell<u64>>, len: usize) -> usize {
    if len == 0 {
        return 0;
    }
    (advance_random(state) as usize) % len
}

fn next_autonomous_delay(state: &Rc<Cell<u64>>) -> Duration {
    let extra = advance_random(state) % AUTONOMOUS_DELAY_SPAN_SECS;
    Duration::from_secs(AUTONOMOUS_MIN_DELAY_SECS + extra)
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
    fn autonomous_delay_stays_in_low_frequency_range() {
        let state = Rc::new(Cell::new(1234));
        for _ in 0..32 {
            let delay = next_autonomous_delay(&state);
            assert!(delay >= Duration::from_secs(75));
            assert!(delay <= Duration::from_secs(135));
        }
    }

    #[test]
    fn autonomous_lines_are_non_empty() {
        assert!(AUTONOMOUS_LINES.iter().all(|line| !line.trim().is_empty()));
        assert_eq!(autonomous_line(AUTONOMOUS_LINES.len()), AUTONOMOUS_LINES[0]);
    }
}
