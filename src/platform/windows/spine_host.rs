use std::sync::OnceLock;

use raw_window_handle::{HasWindowHandle, RawWindowHandle};
use windows::{
    Win32::{
        Foundation::{HINSTANCE, HWND, LPARAM, LRESULT, RECT, WPARAM},
        System::LibraryLoader::GetModuleHandleW,
        UI::WindowsAndMessaging::{
            CreateWindowExW, DefWindowProcW, DestroyWindow, GetWindowRect, IsWindowVisible,
            RegisterClassW, SW_HIDE, SW_SHOWNOACTIVATE, SWP_NOACTIVATE, SetWindowPos, ShowWindow,
            WINDOW_EX_STYLE, WM_NCHITTEST, WNDCLASSW, WS_EX_NOACTIVATE, WS_EX_NOREDIRECTIONBITMAP,
            WS_EX_TOOLWINDOW, WS_POPUP,
        },
    },
    core::w,
};

static CLASS_REGISTERED: OnceLock<bool> = OnceLock::new();

pub(crate) struct SpineCompositionHost {
    hwnd: HWND,
    reference_hwnd: HWND,
    width: i32,
    height: i32,
}

impl SpineCompositionHost {
    pub(crate) fn new(window: &slint::Window, width: u32, height: u32) -> Result<Self, String> {
        let reference_hwnd =
            hwnd_from_slint_window(window).ok_or("native pet HWND is not available yet")?;
        ensure_window_class()?;

        let width = i32::try_from(width.max(1)).map_err(|_| "Spine host width is too large")?;
        let height = i32::try_from(height.max(1)).map_err(|_| "Spine host height is too large")?;

        let mut rect = RECT::default();
        unsafe { GetWindowRect(reference_hwnd, &mut rect) }
            .map_err(|error| format!("GetWindowRect failed: {error}"))?;

        let module = unsafe { GetModuleHandleW(None) }
            .map_err(|error| format!("GetModuleHandleW failed: {error}"))?;
        let instance = HINSTANCE(module.0);

        let hwnd = unsafe {
            CreateWindowExW(
                WINDOW_EX_STYLE(
                    WS_EX_TOOLWINDOW.0 | WS_EX_NOACTIVATE.0 | WS_EX_NOREDIRECTIONBITMAP.0,
                ),
                w!("SenaSpineCompositionHost"),
                w!(""),
                WS_POPUP,
                rect.left,
                rect.top,
                width,
                height,
                None,
                None,
                Some(instance),
                None,
            )
        }
        .map_err(|error| format!("failed to create Spine composition host: {error}"))?;

        let host = Self {
            hwnd,
            reference_hwnd,
            width,
            height,
        };
        host.sync()?;
        Ok(host)
    }

    pub(crate) fn hwnd(&self) -> HWND {
        self.hwnd
    }

    pub(crate) fn sync(&self) -> Result<(), String> {
        let mut rect = RECT::default();
        unsafe { GetWindowRect(self.reference_hwnd, &mut rect) }
            .map_err(|error| format!("GetWindowRect failed: {error}"))?;

        unsafe {
            SetWindowPos(
                self.hwnd,
                Some(self.reference_hwnd),
                rect.left,
                rect.top,
                self.width,
                self.height,
                SWP_NOACTIVATE,
            )
        }
        .map_err(|error| format!("SetWindowPos for Spine host failed: {error}"))?;

        let visible = unsafe { IsWindowVisible(self.reference_hwnd) }.as_bool();
        unsafe {
            let _ = ShowWindow(self.hwnd, if visible { SW_SHOWNOACTIVATE } else { SW_HIDE });
        }

        Ok(())
    }
}

impl Drop for SpineCompositionHost {
    fn drop(&mut self) {
        unsafe {
            let _ = DestroyWindow(self.hwnd);
        }
    }
}

fn ensure_window_class() -> Result<(), String> {
    let registered = *CLASS_REGISTERED.get_or_init(|| {
        let Ok(module) = (unsafe { GetModuleHandleW(None) }) else {
            return false;
        };
        let class = WNDCLASSW {
            lpfnWndProc: Some(window_proc),
            hInstance: HINSTANCE(module.0),
            lpszClassName: w!("SenaSpineCompositionHost"),
            ..Default::default()
        };
        (unsafe { RegisterClassW(&class) }) != 0
    });

    registered
        .then_some(())
        .ok_or_else(|| "failed to register Spine composition host class".into())
}

fn hwnd_from_slint_window(window: &slint::Window) -> Option<HWND> {
    let provider = window.window_handle();
    let handle = provider.window_handle().ok()?;

    match handle.as_raw() {
        RawWindowHandle::Win32(handle) => Some(HWND(handle.hwnd.get() as *mut core::ffi::c_void)),
        _ => None,
    }
}

unsafe extern "system" fn window_proc(
    hwnd: HWND,
    message: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    if message == WM_NCHITTEST {
        return LRESULT(-1);
    }

    unsafe { DefWindowProcW(hwnd, message, wparam, lparam) }
}
