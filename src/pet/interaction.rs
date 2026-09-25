use std::{cell::Cell, rc::Rc, time::Duration};

use slint::{ComponentHandle, Timer};

use crate::PetWindow;

const BUBBLE_DURATION: Duration = Duration::from_millis(2600);

const INTERACTION_LINES: &[&str] = &[
    "嗯？我在这里呀 ✦",
    "今天也一起待一会儿吧。",
    "别戳太快，会痒的……",
    "猫猫刚刚好像也看你了。",
    "要记得偶尔休息一下哦。",
    "星奈收到你的招呼啦 ♡",
];

pub fn install_interactions(window: &PetWindow) {
    let next_line = Rc::new(Cell::new(0usize));
    let bubble_generation = Rc::new(Cell::new(0u64));
    let weak_window = window.as_weak();

    window.on_interact({
        let next_line = Rc::clone(&next_line);
        let bubble_generation = Rc::clone(&bubble_generation);

        move || {
            let Some(window) = weak_window.upgrade() else {
                return;
            };

            let index = next_line.get();
            next_line.set(index.wrapping_add(1));
            window.set_interaction_bubble_text(interaction_line(index).into());
            window.set_interaction_bubble_visible(true);

            let token = bubble_generation.get().wrapping_add(1);
            bubble_generation.set(token);

            let weak_window = window.as_weak();
            let bubble_generation = Rc::clone(&bubble_generation);
            Timer::single_shot(BUBBLE_DURATION, move || {
                if bubble_generation.get() != token {
                    return;
                }

                if let Some(window) = weak_window.upgrade() {
                    window.set_interaction_bubble_visible(false);
                }
            });
        }
    });
}

fn interaction_line(index: usize) -> &'static str {
    INTERACTION_LINES[index % INTERACTION_LINES.len()]
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
    fn interaction_lines_are_non_empty() {
        assert!(INTERACTION_LINES.iter().all(|line| !line.trim().is_empty()));
    }
}
