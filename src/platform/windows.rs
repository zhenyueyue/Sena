mod media;
mod session;

pub use media::MediaWatcher;
pub use session::SessionWatcher;

use std::{cell::RefCell, path::Path, time::Duration};

use raw_window_handle::{HasWindowHandle, RawWindowHandle};
use slint::PhysicalPosition;
use windows::{
    Win32::{
        Foundation::{CloseHandle, HWND, POINT},
        Graphics::Gdi::{
            CreateEllipticRgn, DeleteObject, GetMonitorInfoW, HGDIOBJ, MONITOR_DEFAULTTONEAREST,
            MONITORINFO, MonitorFromPoint, SetWindowRgn,
        },
        System::{
            SystemInformation::GetTickCount,
            Threading::{
                OpenProcess, PROCESS_NAME_WIN32, PROCESS_QUERY_LIMITED_INFORMATION,
                QueryFullProcessImageNameW,
            },
        },
        UI::{
            Accessibility::{HWINEVENTHOOK, SetWinEventHook, UnhookWinEvent},
            Input::KeyboardAndMouse::{GetLastInputInfo, LASTINPUTINFO},
            WindowsAndMessaging::{
                EVENT_SYSTEM_FOREGROUND, GetCursorPos, GetForegroundWindow,
                GetWindowThreadProcessId, WINEVENT_OUTOFCONTEXT,
            },
        },
    },
    core::PWSTR,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WorkArea {
    pub left: i32,
    pub top: i32,
    pub right: i32,
    pub bottom: i32,
}

type ForegroundCallback = Box<dyn FnMut(String)>;

thread_local! {
    static FOREGROUND_CALLBACK: RefCell<Option<ForegroundCallback>> = RefCell::new(None);
}

/// Owns a Windows foreground-event hook.
///
/// The hook is installed from the UI thread. WINEVENT_OUTOFCONTEXT events are
/// delivered through that thread's Windows message loop, which Slint already
/// runs for us. No polling timer or dedicated watcher thread is necessary.
pub struct ForegroundHook {
    handle: HWINEVENTHOOK,
}

impl Drop for ForegroundHook {
    fn drop(&mut self) {
        FOREGROUND_CALLBACK.with(|slot| {
            if let Ok(mut callback) = slot.try_borrow_mut() {
                callback.take();
            }
        });

        unsafe {
            let _ = UnhookWinEvent(self.handle);
        }
    }
}

pub fn install_foreground_hook(
    callback: impl FnMut(String) + 'static,
) -> windows::core::Result<ForegroundHook> {
    FOREGROUND_CALLBACK.with(|slot| {
        *slot.borrow_mut() = Some(Box::new(callback));
    });

    let handle = unsafe {
        SetWinEventHook(
            EVENT_SYSTEM_FOREGROUND,
            EVENT_SYSTEM_FOREGROUND,
            None,
            Some(win_event_proc),
            0,
            0,
            WINEVENT_OUTOFCONTEXT,
        )
    };

    if handle.0.is_null() {
        FOREGROUND_CALLBACK.with(|slot| {
            slot.borrow_mut().take();
        });
        return Err(windows::core::Error::from_thread());
    }

    Ok(ForegroundHook { handle })
}

pub fn current_foreground_process() -> Option<String> {
    let hwnd = unsafe { GetForegroundWindow() };
    process_name_from_window(hwnd)
}

pub fn idle_duration() -> Option<Duration> {
    let mut info = LASTINPUTINFO {
        cbSize: std::mem::size_of::<LASTINPUTINFO>() as u32,
        dwTime: 0,
    };

    if !unsafe { GetLastInputInfo(&mut info) }.as_bool() {
        return None;
    }

    let now = unsafe { GetTickCount() };
    Some(Duration::from_millis(now.wrapping_sub(info.dwTime) as u64))
}

pub fn cursor_position() -> Option<PhysicalPosition> {
    let mut point = POINT::default();
    unsafe { GetCursorPos(&mut point) }.ok()?;
    Some(PhysicalPosition::new(point.x, point.y))
}

pub fn work_area_for_point(point: PhysicalPosition) -> Option<WorkArea> {
    let monitor = unsafe {
        MonitorFromPoint(
            POINT {
                x: point.x,
                y: point.y,
            },
            MONITOR_DEFAULTTONEAREST,
        )
    };

    if monitor.0.is_null() {
        return None;
    }

    let mut info = MONITORINFO {
        cbSize: std::mem::size_of::<MONITORINFO>() as u32,
        ..Default::default()
    };

    if !unsafe { GetMonitorInfoW(monitor, &mut info) }.as_bool() {
        return None;
    }

    Some(WorkArea {
        left: info.rcWork.left,
        top: info.rcWork.top,
        right: info.rcWork.right,
        bottom: info.rcWork.bottom,
    })
}

/// Restricts the native window to the visible placeholder pet body.
///
/// Windows does not hit-test pixels outside the region, so transparent corner
/// areas pass mouse input to the desktop or application underneath.
pub fn apply_pet_window_region_if_available(window: &slint::Window, placeholder: bool) {
    let Some(hwnd) = hwnd_from_slint_window(window) else {
        return;
    };

    if !placeholder {
        unsafe {
            let _ = SetWindowRgn(hwnd, None, true);
        }
        return;
    }

    let scale = window.scale_factor();
    let left = (20.0 * scale).round() as i32;
    let top = (28.0 * scale).round() as i32;
    let right = (200.0 * scale).round() as i32;
    let bottom = (216.0 * scale).round() as i32;

    let region = unsafe { CreateEllipticRgn(left, top, right, bottom) };
    if region.0.is_null() {
        return;
    }

    let applied = unsafe { SetWindowRgn(hwnd, Some(region), true) };
    if applied == 0 {
        unsafe {
            let _ = DeleteObject(HGDIOBJ(region.0));
        }
    }
}

fn hwnd_from_slint_window(window: &slint::Window) -> Option<HWND> {
    let provider = window.window_handle();
    let handle = provider.window_handle().ok()?;

    match handle.as_raw() {
        RawWindowHandle::Win32(handle) => Some(HWND(handle.hwnd.get() as *mut core::ffi::c_void)),
        _ => None,
    }
}

unsafe extern "system" fn win_event_proc(
    _hook: HWINEVENTHOOK,
    event: u32,
    hwnd: HWND,
    _id_object: i32,
    _id_child: i32,
    _event_thread: u32,
    _event_time: u32,
) {
    if event != EVENT_SYSTEM_FOREGROUND || hwnd.0.is_null() {
        return;
    }

    let Some(process_name) = process_name_from_window(hwnd) else {
        return;
    };

    FOREGROUND_CALLBACK.with(|slot| {
        let Ok(mut callback) = slot.try_borrow_mut() else {
            return;
        };

        if let Some(callback) = callback.as_mut() {
            callback(process_name);
        }
    });
}

fn process_name_from_window(hwnd: HWND) -> Option<String> {
    if hwnd.0.is_null() {
        return None;
    }

    let mut process_id = 0u32;
    unsafe {
        GetWindowThreadProcessId(hwnd, Some(&mut process_id));
    }

    if process_id == 0 {
        return None;
    }

    process_name_from_id(process_id)
}

fn process_name_from_id(process_id: u32) -> Option<String> {
    let process =
        unsafe { OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, false, process_id) }.ok()?;

    let result = (|| {
        let mut buffer = vec![0u16; 32_768];
        let mut size = buffer.len() as u32;

        unsafe {
            QueryFullProcessImageNameW(
                process,
                PROCESS_NAME_WIN32,
                PWSTR(buffer.as_mut_ptr()),
                &mut size,
            )
        }
        .ok()?;

        let full_path = String::from_utf16_lossy(&buffer[..size as usize]);
        Path::new(&full_path)
            .file_name()
            .and_then(|name| name.to_str())
            .map(str::to_owned)
    })();

    unsafe {
        let _ = CloseHandle(process);
    }

    result
}
