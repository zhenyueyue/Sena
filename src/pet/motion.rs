use std::{cell::RefCell, rc::Rc, time::Duration};

use slint::{ComponentHandle, PhysicalPosition, Timer};

use crate::{PetWindow, platform::windows};

#[derive(Debug, Default)]
struct MotionState {
    dragging: bool,
    drag_offset_x: i32,
    drag_offset_y: i32,
}

pub fn install_motion(window: &PetWindow) {
    let state = Rc::new(RefCell::new(MotionState::default()));

    {
        let state = Rc::clone(&state);
        let window = window.as_weak();

        window
            .upgrade()
            .expect("pet window must be alive")
            .on_drag_start(move || {
                let Some(window) = window.upgrade() else {
                    return;
                };
                let Some(cursor) = windows::cursor_position() else {
                    return;
                };

                let position = window.window().position();
                let mut state = state.borrow_mut();
                state.dragging = true;
                state.drag_offset_x = cursor.x - position.x;
                state.drag_offset_y = cursor.y - position.y;
            });
    }

    {
        let state = Rc::clone(&state);
        let window = window.as_weak();

        window
            .upgrade()
            .expect("pet window must be alive")
            .on_drag_move(move || {
                let Some(window) = window.upgrade() else {
                    return;
                };
                let Some(cursor) = windows::cursor_position() else {
                    return;
                };

                let state = state.borrow();
                if !state.dragging {
                    return;
                }

                let mut x = cursor.x - state.drag_offset_x;
                let mut y = cursor.y - state.drag_offset_y;

                if let Some(work_area) = windows::work_area_for_point(cursor) {
                    let size = window.window().size();
                    x = x.clamp(work_area.left, work_area.right - size.width as i32);
                    y = y.clamp(work_area.top, work_area.bottom - size.height as i32);
                }

                window.window().set_position(PhysicalPosition::new(x, y));
            });
    }

    {
        let state = Rc::clone(&state);

        window.on_drag_end(move || {
            state.borrow_mut().dragging = false;
        });
    }

    // Window handles are typically available after the first event-loop turn.
    // This is a one-shot initialization only; there is no permanent timer.
    let window = window.as_weak();
    Timer::single_shot(Duration::from_millis(100), move || {
        if let Some(window) = window.upgrade() {
            windows::apply_pet_window_region_if_available(
                &window.window(),
                !window.get_use_sprite(),
            );
        }
    });
}
