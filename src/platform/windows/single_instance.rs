use std::{
    ffi::c_void,
    io,
    sync::Arc,
    thread::{self, JoinHandle},
};

use windows::{
    Win32::{
        Foundation::{CloseHandle, ERROR_ALREADY_EXISTS, GetLastError, HANDLE, WAIT_OBJECT_0},
        System::Threading::{
            CreateEventW, CreateMutexW, INFINITE, SetEvent, WaitForMultipleObjects,
        },
    },
    core::PCWSTR,
};

const MUTEX_NAME: &str = r"Local\Sena.DesktopPet.Instance.v1";
const SHOW_EVENT_NAME: &str = r"Local\Sena.DesktopPet.Show.v1";

pub enum SingleInstanceAcquire {
    Primary(SingleInstance),
    ExistingNotified,
}

pub struct SingleInstance {
    mutex: HANDLE,
    show_event: HANDLE,
    stop_event: Option<HANDLE>,
    thread: Option<JoinHandle<()>>,
}

impl SingleInstance {
    pub fn acquire() -> io::Result<SingleInstanceAcquire> {
        Self::acquire_named(MUTEX_NAME, SHOW_EVENT_NAME)
    }

    pub fn start_show_listener(
        &mut self,
        callback: impl Fn() + Send + Sync + 'static,
    ) -> io::Result<()> {
        if self.thread.is_some() {
            return Ok(());
        }

        let stop_event = unsafe { CreateEventW(None, false, false, None) }
            .map_err(|error| io::Error::other(error.to_string()))?;

        let show_event = self.show_event.0 as isize;
        let stop_event_raw = stop_event.0 as isize;
        let callback: Arc<dyn Fn() + Send + Sync + 'static> = Arc::new(callback);

        let thread = thread::Builder::new()
            .name("sena-instance".into())
            .spawn(move || {
                let show_event = HANDLE(show_event as *mut c_void);
                let stop_event = HANDLE(stop_event_raw as *mut c_void);
                let handles = [show_event, stop_event];

                loop {
                    let result = unsafe { WaitForMultipleObjects(&handles, false, INFINITE) };
                    if result == WAIT_OBJECT_0 {
                        callback();
                    } else if result.0 == WAIT_OBJECT_0.0 + 1 {
                        break;
                    } else {
                        break;
                    }
                }
            })?;

        self.stop_event = Some(stop_event);
        self.thread = Some(thread);
        Ok(())
    }

    fn acquire_named(mutex_name: &str, show_event_name: &str) -> io::Result<SingleInstanceAcquire> {
        let mutex_name = wide_null(mutex_name);
        let show_event_name = wide_null(show_event_name);

        // Create/open the show event first. This guarantees that a secondary
        // process can signal it even while the primary is still initializing.
        let show_event =
            unsafe { CreateEventW(None, false, false, PCWSTR(show_event_name.as_ptr())) }
                .map_err(|error| io::Error::other(error.to_string()))?;

        let mutex = match unsafe { CreateMutexW(None, false, PCWSTR(mutex_name.as_ptr())) } {
            Ok(mutex) => mutex,
            Err(error) => {
                unsafe {
                    let _ = CloseHandle(show_event);
                }
                return Err(io::Error::other(error.to_string()));
            }
        };

        let already_exists = unsafe { GetLastError() } == ERROR_ALREADY_EXISTS;
        if already_exists {
            let signal_result = unsafe { SetEvent(show_event) };

            unsafe {
                let _ = CloseHandle(mutex);
                let _ = CloseHandle(show_event);
            }

            signal_result.map_err(|error| io::Error::other(error.to_string()))?;
            Ok(SingleInstanceAcquire::ExistingNotified)
        } else {
            Ok(SingleInstanceAcquire::Primary(Self {
                mutex,
                show_event,
                stop_event: None,
                thread: None,
            }))
        }
    }
}

impl Drop for SingleInstance {
    fn drop(&mut self) {
        if let Some(stop_event) = self.stop_event {
            unsafe {
                let _ = SetEvent(stop_event);
            }
        }

        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }

        if let Some(stop_event) = self.stop_event.take() {
            unsafe {
                let _ = CloseHandle(stop_event);
            }
        }

        unsafe {
            let _ = CloseHandle(self.show_event);
            let _ = CloseHandle(self.mutex);
        }
    }
}

fn wide_null(value: &str) -> Vec<u16> {
    value.encode_utf16().chain(std::iter::once(0)).collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{
        sync::mpsc,
        time::{Duration, SystemTime, UNIX_EPOCH},
    };

    fn unique_names(label: &str) -> (String, String) {
        let suffix = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system time should be after epoch")
            .as_nanos();

        (
            format!(
                r"Local\Sena.Test.{label}.Instance.{}.{}",
                std::process::id(),
                suffix
            ),
            format!(
                r"Local\Sena.Test.{label}.Show.{}.{}",
                std::process::id(),
                suffix
            ),
        )
    }

    fn acquire_primary(mutex_name: &str, event_name: &str) -> SingleInstance {
        match SingleInstance::acquire_named(mutex_name, event_name)
            .expect("first acquire should succeed")
        {
            SingleInstanceAcquire::Primary(primary) => primary,
            SingleInstanceAcquire::ExistingNotified => {
                panic!("first acquire must become primary")
            }
        }
    }

    #[test]
    fn second_acquire_notifies_primary_listener() {
        let (mutex_name, event_name) = unique_names("live");
        let mut primary = acquire_primary(&mutex_name, &event_name);

        let (tx, rx) = mpsc::sync_channel(1);
        primary
            .start_show_listener(move || {
                let _ = tx.send(());
            })
            .expect("listener should start");

        assert!(matches!(
            SingleInstance::acquire_named(&mutex_name, &event_name)
                .expect("second acquire should succeed"),
            SingleInstanceAcquire::ExistingNotified
        ));

        rx.recv_timeout(Duration::from_secs(2))
            .expect("primary should receive show request");
    }

    #[test]
    fn show_signal_survives_until_primary_listener_starts() {
        let (mutex_name, event_name) = unique_names("early");
        let mut primary = acquire_primary(&mutex_name, &event_name);

        assert!(matches!(
            SingleInstance::acquire_named(&mutex_name, &event_name)
                .expect("early second acquire should succeed"),
            SingleInstanceAcquire::ExistingNotified
        ));

        let (tx, rx) = mpsc::sync_channel(1);
        primary
            .start_show_listener(move || {
                let _ = tx.send(());
            })
            .expect("listener should start after signal");

        rx.recv_timeout(Duration::from_secs(2))
            .expect("pending show signal should be delivered");
    }
}
