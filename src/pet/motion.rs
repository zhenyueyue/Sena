use std::{cell::RefCell, rc::Rc, time::Duration};

use slint::{ComponentHandle, PhysicalPosition, Timer};

use crate::{PetWindow, platform::windows};

#[derive(Debug, Default)]
struct MotionState {
    dragging: bool,
    drag_offset_x: i32,
    drag_offset_y: i32,
}

pub fn install_motion(
    window: &PetWindow,
    on_position_changed: impl Fn(PhysicalPosition) + 'static,
) {
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
        let window = window.as_weak();

        window
            .upgrade()
            .expect("pet window must be alive")
            .on_drag_end(move || {
                state.borrow_mut().dragging = false;

                if let Some(window) = window.upgrade() {
                    on_position_changed(window.window().position());
                }
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

pub fn place_default_position(window: &PetWindow) -> Option<PhysicalPosition> {
    const EDGE_MARGIN: i32 = 28;

    let cursor = windows::cursor_position()?;
    let work_area = windows::work_area_for_point(cursor)?;
    let size = window.window().size();
    let position = default_position_in_work_area(
        work_area,
        size.width as i32,
        size.height as i32,
        EDGE_MARGIN,
    );

    window.window().set_position(position);
    Some(position)
}

fn default_position_in_work_area(
    work_area: windows::WorkArea,
    width: i32,
    height: i32,
    margin: i32,
) -> PhysicalPosition {
    let max_x = (work_area.right - width).max(work_area.left);
    let max_y = (work_area.bottom - height).max(work_area.top);
    let x = (max_x - margin).max(work_area.left);
    let y = (max_y - margin).max(work_area.top);

    PhysicalPosition::new(x, y)
}

pub fn restore_position(window: &PetWindow, position: PhysicalPosition) {
    let size = window.window().size();
    let mut x = position.x;
    let mut y = position.y;

    if let Some(work_area) = windows::work_area_for_point(position) {
        let max_x = (work_area.right - size.width as i32).max(work_area.left);
        let max_y = (work_area.bottom - size.height as i32).max(work_area.top);
        x = x.clamp(work_area.left, max_x);
        y = y.clamp(work_area.top, max_y);
    }

    window.window().set_position(PhysicalPosition::new(x, y));
}

pub fn clamp_current_position(window: &PetWindow) -> PhysicalPosition {
    let current = window.window().position();
    restore_position(window, current);
    window.window().position()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_position_uses_bottom_right_margin() {
        let work_area = windows::WorkArea {
            left: 0,
            top: 0,
            right: 1920,
            bottom: 1040,
        };

        let position = default_position_in_work_area(work_area, 220, 240, 28);

        assert_eq!(position, PhysicalPosition::new(1672, 772));
    }

    #[test]
    fn default_position_stays_inside_small_work_area() {
        let work_area = windows::WorkArea {
            left: 100,
            top: 50,
            right: 260,
            bottom: 200,
        };

        let position = default_position_in_work_area(work_area, 220, 240, 28);

        assert_eq!(position, PhysicalPosition::new(100, 50));
    }
}
