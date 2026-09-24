use std::{
    ffi::c_void,
    sync::{Arc, Mutex, OnceLock, mpsc},
    thread::{self, JoinHandle},
    time::Duration,
};

use windows::{
    Win32::{
        Foundation::{HINSTANCE, HWND, LPARAM, LRESULT, POINT, WPARAM},
        System::LibraryLoader::GetModuleHandleW,
        UI::{
            Shell::{
                NIF_ICON, NIF_MESSAGE, NIF_TIP, NIM_ADD, NIM_DELETE, NOTIFYICONDATAW,
                Shell_NotifyIconW,
            },
            WindowsAndMessaging::{
                AppendMenuW, CreatePopupMenu, CreateWindowExW, DefWindowProcW, DestroyMenu,
                DestroyWindow, DispatchMessageW, GetCursorPos, GetMessageW, IDI_APPLICATION,
                LoadIconW, MF_SEPARATOR, MF_STRING, MSG, PostMessageW, PostQuitMessage,
                RegisterClassW, SetForegroundWindow, TPM_LEFTALIGN, TPM_RETURNCMD, TPM_RIGHTBUTTON,
                TrackPopupMenu, TranslateMessage, WINDOW_EX_STYLE, WINDOW_STYLE, WM_APP, WM_CLOSE,
                WM_CONTEXTMENU, WM_DESTROY, WM_LBUTTONDBLCLK, WM_LBUTTONUP, WM_RBUTTONUP,
                WNDCLASSW,
            },
        },
    },
    core::{PCWSTR, w},
};

const TRAY_ICON_ID: u32 = 1;
const TRAY_CALLBACK_MESSAGE: u32 = WM_APP + 41;

const CMD_SETTINGS: usize = 1000;
const CMD_SHOW: usize = 1001;
const CMD_HIDE: usize = 1002;
const CMD_SCALE_80: usize = 1010;
const CMD_SCALE_100: usize = 1011;
const CMD_SCALE_120: usize = 1012;
const CMD_EXIT: usize = 1099;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum TrayAction {
    Settings,
    Show,
    Hide,
    SetScale(f32),
    Exit,
}

type TrayCallback = Arc<dyn Fn(TrayAction) + Send + Sync + 'static>;

static TRAY_CALLBACK: OnceLock<Mutex<Option<TrayCallback>>> = OnceLock::new();

pub struct TrayIcon {
    hwnd: isize,
    thread: Option<JoinHandle<()>>,
}

impl TrayIcon {
    pub fn start(callback: impl Fn(TrayAction) + Send + Sync + 'static) -> std::io::Result<Self> {
        let callback: TrayCallback = Arc::new(callback);
        let (ready_tx, ready_rx) = mpsc::sync_channel(1);

        let thread = thread::Builder::new()
            .name("sena-tray".into())
            .spawn(move || run_tray_thread(callback, ready_tx))?;

        let hwnd = ready_rx
            .recv_timeout(Duration::from_secs(2))
            .map_err(|error| std::io::Error::other(error.to_string()))?;

        if hwnd == 0 {
            let _ = thread.join();
            return Err(std::io::Error::other(
                "failed to create Sena notification-area icon",
            ));
        }

        Ok(Self {
            hwnd,
            thread: Some(thread),
        })
    }
}

impl Drop for TrayIcon {
    fn drop(&mut self) {
        if self.hwnd != 0 {
            let hwnd = HWND(self.hwnd as *mut c_void);
            unsafe {
                let _ = PostMessageW(Some(hwnd), WM_CLOSE, WPARAM(0), LPARAM(0));
            }
        }

        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
    }
}

fn run_tray_thread(callback: TrayCallback, ready_tx: mpsc::SyncSender<isize>) {
    let slot = TRAY_CALLBACK.get_or_init(|| Mutex::new(None));
    *slot.lock().expect("tray callback lock poisoned") = Some(callback);

    let hwnd = match unsafe { create_hidden_window() } {
        Ok(hwnd) => hwnd,
        Err(_) => {
            let _ = ready_tx.send(0);
            clear_callback();
            return;
        }
    };

    if !unsafe { add_tray_icon(hwnd) } {
        let _ = ready_tx.send(0);
        unsafe {
            let _ = DestroyWindow(hwnd);
        }
        clear_callback();
        return;
    }

    let _ = ready_tx.send(hwnd.0 as isize);

    let mut message = MSG::default();
    while unsafe { GetMessageW(&mut message, None, 0, 0) }.0 > 0 {
        unsafe {
            let _ = TranslateMessage(&message);
            DispatchMessageW(&message);
        }
    }

    clear_callback();
}

unsafe fn create_hidden_window() -> windows::core::Result<HWND> {
    let module = unsafe { GetModuleHandleW(None) }?;
    let instance = HINSTANCE(module.0);
    let class_name = w!("SenaTrayWindow");

    let class = WNDCLASSW {
        lpfnWndProc: Some(window_proc),
        hInstance: instance,
        lpszClassName: class_name,
        ..Default::default()
    };

    let atom = unsafe { RegisterClassW(&class) };
    if atom == 0 {
        return Err(windows::core::Error::from_thread());
    }

    unsafe {
        CreateWindowExW(
            WINDOW_EX_STYLE(0),
            class_name,
            w!("Sena Tray"),
            WINDOW_STYLE(0),
            0,
            0,
            0,
            0,
            None,
            None,
            Some(instance),
            None,
        )
    }
}

unsafe fn add_tray_icon(hwnd: HWND) -> bool {
    let icon = match unsafe { LoadIconW(None, IDI_APPLICATION) } {
        Ok(icon) => icon,
        Err(_) => return false,
    };

    let mut data = NOTIFYICONDATAW {
        cbSize: std::mem::size_of::<NOTIFYICONDATAW>() as u32,
        hWnd: hwnd,
        uID: TRAY_ICON_ID,
        uFlags: NIF_MESSAGE | NIF_ICON | NIF_TIP,
        uCallbackMessage: TRAY_CALLBACK_MESSAGE,
        hIcon: icon,
        ..Default::default()
    };
    write_utf16(&mut data.szTip, "Sena（星奈）");

    unsafe { Shell_NotifyIconW(NIM_ADD, &data) }.as_bool()
}

unsafe fn remove_tray_icon(hwnd: HWND) {
    let data = NOTIFYICONDATAW {
        cbSize: std::mem::size_of::<NOTIFYICONDATAW>() as u32,
        hWnd: hwnd,
        uID: TRAY_ICON_ID,
        ..Default::default()
    };

    unsafe {
        let _ = Shell_NotifyIconW(NIM_DELETE, &data);
    }
}

unsafe fn show_context_menu(hwnd: HWND) {
    let menu = match unsafe { CreatePopupMenu() } {
        Ok(menu) => menu,
        Err(_) => return,
    };

    unsafe {
        let _ = AppendMenuW(menu, MF_STRING, CMD_SETTINGS, w!("设置..."));
        let _ = AppendMenuW(menu, MF_SEPARATOR, 0, PCWSTR::null());
        let _ = AppendMenuW(menu, MF_STRING, CMD_SHOW, w!("显示 Sena"));
        let _ = AppendMenuW(menu, MF_STRING, CMD_HIDE, w!("隐藏 Sena"));
        let _ = AppendMenuW(menu, MF_SEPARATOR, 0, PCWSTR::null());
        let _ = AppendMenuW(menu, MF_STRING, CMD_SCALE_80, w!("大小 80%"));
        let _ = AppendMenuW(menu, MF_STRING, CMD_SCALE_100, w!("大小 100%"));
        let _ = AppendMenuW(menu, MF_STRING, CMD_SCALE_120, w!("大小 120%"));
        let _ = AppendMenuW(menu, MF_SEPARATOR, 0, PCWSTR::null());
        let _ = AppendMenuW(menu, MF_STRING, CMD_EXIT, w!("退出"));
    }

    let mut point = POINT::default();
    if unsafe { GetCursorPos(&mut point) }.is_err() {
        unsafe {
            let _ = DestroyMenu(menu);
        }
        return;
    }

    unsafe {
        let _ = SetForegroundWindow(hwnd);
    }

    let command = unsafe {
        TrackPopupMenu(
            menu,
            TPM_LEFTALIGN | TPM_RIGHTBUTTON | TPM_RETURNCMD,
            point.x,
            point.y,
            None,
            hwnd,
            None,
        )
    };

    unsafe {
        let _ = DestroyMenu(menu);
    }

    if let Some(action) = action_for_command(command.0 as usize) {
        notify(action);
    }
}

fn action_for_command(command: usize) -> Option<TrayAction> {
    match command {
        CMD_SETTINGS => Some(TrayAction::Settings),
        CMD_SHOW => Some(TrayAction::Show),
        CMD_HIDE => Some(TrayAction::Hide),
        CMD_SCALE_80 => Some(TrayAction::SetScale(0.8)),
        CMD_SCALE_100 => Some(TrayAction::SetScale(1.0)),
        CMD_SCALE_120 => Some(TrayAction::SetScale(1.2)),
        CMD_EXIT => Some(TrayAction::Exit),
        _ => None,
    }
}

unsafe extern "system" fn window_proc(
    hwnd: HWND,
    message: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    match message {
        TRAY_CALLBACK_MESSAGE => {
            match lparam.0 as u32 {
                WM_LBUTTONUP | WM_LBUTTONDBLCLK => notify(TrayAction::Show),
                WM_RBUTTONUP | WM_CONTEXTMENU => unsafe { show_context_menu(hwnd) },
                _ => {}
            }
            LRESULT(0)
        }
        WM_CLOSE => {
            unsafe {
                let _ = DestroyWindow(hwnd);
            }
            LRESULT(0)
        }
        WM_DESTROY => {
            unsafe {
                remove_tray_icon(hwnd);
                PostQuitMessage(0);
            }
            LRESULT(0)
        }
        _ => unsafe { DefWindowProcW(hwnd, message, wparam, lparam) },
    }
}

fn notify(action: TrayAction) {
    let Some(slot) = TRAY_CALLBACK.get() else {
        return;
    };

    let callback = slot.lock().expect("tray callback lock poisoned").clone();
    if let Some(callback) = callback {
        callback(action);
    }
}

fn clear_callback() {
    if let Some(slot) = TRAY_CALLBACK.get() {
        slot.lock().expect("tray callback lock poisoned").take();
    }
}

fn write_utf16(target: &mut [u16], value: &str) {
    target.fill(0);
    let capacity = target.len().saturating_sub(1);
    for (slot, unit) in target.iter_mut().take(capacity).zip(value.encode_utf16()) {
        *slot = unit;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tray_commands_map_to_expected_actions() {
        assert_eq!(action_for_command(CMD_SETTINGS), Some(TrayAction::Settings));
        assert_eq!(action_for_command(CMD_SHOW), Some(TrayAction::Show));
        assert_eq!(action_for_command(CMD_HIDE), Some(TrayAction::Hide));
        assert_eq!(
            action_for_command(CMD_SCALE_80),
            Some(TrayAction::SetScale(0.8))
        );
        assert_eq!(
            action_for_command(CMD_SCALE_100),
            Some(TrayAction::SetScale(1.0))
        );
        assert_eq!(
            action_for_command(CMD_SCALE_120),
            Some(TrayAction::SetScale(1.2))
        );
        assert_eq!(action_for_command(CMD_EXIT), Some(TrayAction::Exit));
        assert_eq!(action_for_command(9999), None);
    }
}
