use std::{
    cell::RefCell,
    sync::{Arc, Mutex, OnceLock, mpsc},
    thread::{self, JoinHandle},
    time::{Duration, Instant},
};

use windows::Win32::{
    Foundation::{HINSTANCE, LPARAM, LRESULT, WPARAM},
    System::{LibraryLoader::GetModuleHandleW, Threading::GetCurrentThreadId},
    UI::WindowsAndMessaging::{
        CallNextHookEx, DispatchMessageW, GetMessageW, MSG, PM_NOREMOVE, PeekMessageW,
        PostThreadMessageW, SetWindowsHookExW, TranslateMessage, UnhookWindowsHookEx,
        WH_KEYBOARD_LL, WM_KEYDOWN, WM_QUIT, WM_SYSKEYDOWN,
    },
};

const MIN_NOTIFY_INTERVAL: Duration = Duration::from_millis(50);

type KeyboardCallback = Arc<dyn Fn() + Send + Sync + 'static>;

static KEYBOARD_CALLBACK: OnceLock<Mutex<Option<KeyboardCallback>>> = OnceLock::new();

thread_local! {
    static LAST_NOTIFY: RefCell<Option<Instant>> = const { RefCell::new(None) };
}

/// Event-driven Windows keyboard activity watcher.
///
/// The hook deliberately does not inspect or retain virtual-key codes, scan
/// codes, text, or modifier state. It only emits a throttled "keyboard was
/// active" pulse on key-down messages.
pub struct KeyboardWatcher {
    thread_id: u32,
    thread: Option<JoinHandle<()>>,
}

impl KeyboardWatcher {
    pub fn start(callback: impl Fn() + Send + Sync + 'static) -> std::io::Result<Self> {
        let callback: KeyboardCallback = Arc::new(callback);
        let (ready_tx, ready_rx) = mpsc::sync_channel(1);

        let thread = thread::Builder::new()
            .name("sena-keyboard".into())
            .spawn(move || run_keyboard_thread(callback, ready_tx))?;

        let thread_id = ready_rx
            .recv_timeout(Duration::from_secs(2))
            .map_err(|error| std::io::Error::other(error.to_string()))?;

        if thread_id == 0 {
            let _ = thread.join();
            return Err(std::io::Error::other(
                "failed to install Windows keyboard activity hook",
            ));
        }

        Ok(Self {
            thread_id,
            thread: Some(thread),
        })
    }
}

impl Drop for KeyboardWatcher {
    fn drop(&mut self) {
        if self.thread_id != 0 {
            unsafe {
                let _ = PostThreadMessageW(self.thread_id, WM_QUIT, WPARAM(0), LPARAM(0));
            }
        }

        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
    }
}

fn run_keyboard_thread(callback: KeyboardCallback, ready_tx: mpsc::SyncSender<u32>) {
    let slot = KEYBOARD_CALLBACK.get_or_init(|| Mutex::new(None));
    *slot.lock().expect("keyboard callback lock poisoned") = Some(callback);

    let module = match unsafe { GetModuleHandleW(None) } {
        Ok(module) => module,
        Err(_) => {
            let _ = ready_tx.send(0);
            clear_callback();
            return;
        }
    };

    let hook = match unsafe {
        SetWindowsHookExW(
            WH_KEYBOARD_LL,
            Some(keyboard_proc),
            Some(HINSTANCE(module.0)),
            0,
        )
    } {
        Ok(hook) => hook,
        Err(_) => {
            let _ = ready_tx.send(0);
            clear_callback();
            return;
        }
    };

    // Ensure this thread owns a Windows message queue before exposing its id.
    let mut message = MSG::default();
    unsafe {
        let _ = PeekMessageW(&mut message, None, 0, 0, PM_NOREMOVE);
    }

    let _ = ready_tx.send(unsafe { GetCurrentThreadId() });

    while unsafe { GetMessageW(&mut message, None, 0, 0) }.0 > 0 {
        unsafe {
            let _ = TranslateMessage(&message);
            DispatchMessageW(&message);
        }
    }

    unsafe {
        let _ = UnhookWindowsHookEx(hook);
    }
    clear_callback();
}

unsafe extern "system" fn keyboard_proc(code: i32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    if code >= 0 && is_key_down_message(wparam.0 as u32) {
        notify_activity();
    }

    unsafe { CallNextHookEx(None, code, wparam, lparam) }
}

fn is_key_down_message(message: u32) -> bool {
    matches!(message, WM_KEYDOWN | WM_SYSKEYDOWN)
}

fn notify_activity() {
    let should_notify = LAST_NOTIFY.with(|last| {
        let mut last = last.borrow_mut();
        let now = Instant::now();

        if last.is_some_and(|previous| now.duration_since(previous) < MIN_NOTIFY_INTERVAL) {
            false
        } else {
            *last = Some(now);
            true
        }
    });

    if !should_notify {
        return;
    }

    let Some(slot) = KEYBOARD_CALLBACK.get() else {
        return;
    };

    let callback = slot
        .lock()
        .expect("keyboard callback lock poisoned")
        .clone();

    if let Some(callback) = callback {
        callback();
    }
}

fn clear_callback() {
    if let Some(slot) = KEYBOARD_CALLBACK.get() {
        slot.lock().expect("keyboard callback lock poisoned").take();
    }

    LAST_NOTIFY.with(|last| *last.borrow_mut() = None);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_key_down_messages_trigger_activity() {
        assert!(is_key_down_message(WM_KEYDOWN));
        assert!(is_key_down_message(WM_SYSKEYDOWN));
        assert!(!is_key_down_message(
            windows::Win32::UI::WindowsAndMessaging::WM_KEYUP
        ));
        assert!(!is_key_down_message(
            windows::Win32::UI::WindowsAndMessaging::WM_SYSKEYUP
        ));
    }
}
