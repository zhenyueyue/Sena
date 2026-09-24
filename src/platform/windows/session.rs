use std::{
    ffi::c_void,
    sync::{Arc, Mutex, OnceLock, mpsc},
    thread::{self, JoinHandle},
    time::Duration,
};

use windows::{
    Win32::{
        Foundation::{HINSTANCE, HWND, LPARAM, LRESULT, WPARAM},
        System::{
            LibraryLoader::GetModuleHandleW,
            RemoteDesktop::{
                NOTIFY_FOR_THIS_SESSION, WTSRegisterSessionNotification,
                WTSUnRegisterSessionNotification,
            },
        },
        UI::WindowsAndMessaging::{
            CreateWindowExW, DefWindowProcW, DestroyWindow, DispatchMessageW, GetMessageW,
            HWND_MESSAGE, MSG, PostMessageW, PostQuitMessage, RegisterClassW, TranslateMessage,
            WINDOW_EX_STYLE, WINDOW_STYLE, WM_CLOSE, WM_DESTROY, WM_WTSSESSION_CHANGE, WNDCLASSW,
            WTS_SESSION_LOCK, WTS_SESSION_UNLOCK,
        },
    },
    core::w,
};

type SessionCallback = Arc<dyn Fn(bool) + Send + Sync + 'static>;

static SESSION_CALLBACK: OnceLock<Mutex<Option<SessionCallback>>> = OnceLock::new();

pub struct SessionWatcher {
    hwnd: isize,
    thread: Option<JoinHandle<()>>,
}

impl SessionWatcher {
    pub fn start(callback: impl Fn(bool) + Send + Sync + 'static) -> std::io::Result<Self> {
        let callback: SessionCallback = Arc::new(callback);
        let (ready_tx, ready_rx) = mpsc::sync_channel(1);

        let thread = thread::Builder::new()
            .name("sena-session".into())
            .spawn(move || run_session_thread(callback, ready_tx))?;

        let hwnd = ready_rx
            .recv_timeout(Duration::from_secs(2))
            .map_err(|error| std::io::Error::other(error.to_string()))?;

        if hwnd == 0 {
            let _ = thread.join();
            return Err(std::io::Error::other(
                "failed to create Windows session notification window",
            ));
        }

        Ok(Self {
            hwnd,
            thread: Some(thread),
        })
    }
}

impl Drop for SessionWatcher {
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

fn run_session_thread(callback: SessionCallback, ready_tx: mpsc::SyncSender<isize>) {
    let slot = SESSION_CALLBACK.get_or_init(|| Mutex::new(None));
    *slot.lock().expect("session callback lock poisoned") = Some(callback);

    let result = unsafe { create_message_window() };
    let hwnd = match result {
        Ok(hwnd) => hwnd,
        Err(_) => {
            let _ = ready_tx.send(0);
            clear_callback();
            return;
        }
    };

    if unsafe { WTSRegisterSessionNotification(hwnd, NOTIFY_FOR_THIS_SESSION) }.is_err() {
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

unsafe fn create_message_window() -> windows::core::Result<HWND> {
    let module = unsafe { GetModuleHandleW(None) }?;
    let instance = HINSTANCE(module.0);
    let class_name = w!("SenaSessionWatcher");

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
            w!(""),
            WINDOW_STYLE(0),
            0,
            0,
            0,
            0,
            Some(HWND_MESSAGE),
            None,
            Some(instance),
            None,
        )
    }
}

unsafe extern "system" fn window_proc(
    hwnd: HWND,
    message: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    match message {
        WM_WTSSESSION_CHANGE => {
            match wparam.0 as u32 {
                WTS_SESSION_LOCK => notify(true),
                WTS_SESSION_UNLOCK => notify(false),
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
                let _ = WTSUnRegisterSessionNotification(hwnd);
                PostQuitMessage(0);
            }
            LRESULT(0)
        }
        _ => unsafe { DefWindowProcW(hwnd, message, wparam, lparam) },
    }
}

fn notify(locked: bool) {
    let Some(slot) = SESSION_CALLBACK.get() else {
        return;
    };

    let callback = slot.lock().expect("session callback lock poisoned").clone();

    if let Some(callback) = callback {
        callback(locked);
    }
}

fn clear_callback() {
    if let Some(slot) = SESSION_CALLBACK.get() {
        slot.lock().expect("session callback lock poisoned").take();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn receives_lock_and_unlock_session_messages() {
        let (tx, rx) = mpsc::channel();
        let watcher = SessionWatcher::start(move |locked| {
            let _ = tx.send(locked);
        })
        .expect("session watcher should start");

        let hwnd = HWND(watcher.hwnd as *mut c_void);

        unsafe {
            PostMessageW(
                Some(hwnd),
                WM_WTSSESSION_CHANGE,
                WPARAM(WTS_SESSION_LOCK as usize),
                LPARAM(0),
            )
            .expect("lock message should post");
        }

        assert_eq!(
            rx.recv_timeout(Duration::from_secs(1)),
            Ok(true),
            "lock event should be delivered"
        );

        unsafe {
            PostMessageW(
                Some(hwnd),
                WM_WTSSESSION_CHANGE,
                WPARAM(WTS_SESSION_UNLOCK as usize),
                LPARAM(0),
            )
            .expect("unlock message should post");
        }

        assert_eq!(
            rx.recv_timeout(Duration::from_secs(1)),
            Ok(false),
            "unlock event should be delivered"
        );
    }
}
