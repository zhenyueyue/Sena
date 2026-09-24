use std::{
    sync::{Arc, Mutex, mpsc},
    thread::{self, JoinHandle},
};

use windows::{
    Foundation::TypedEventHandler,
    Media::Control::{
        CurrentSessionChangedEventArgs, GlobalSystemMediaTransportControlsSession,
        GlobalSystemMediaTransportControlsSessionManager,
        GlobalSystemMediaTransportControlsSessionPlaybackStatus, PlaybackInfoChangedEventArgs,
    },
    Win32::System::WinRT::{RO_INIT_MULTITHREADED, RoInitialize, RoUninitialize},
};

use crate::context::MediaState;

type MediaCallback = Arc<dyn Fn(MediaState) + Send + Sync + 'static>;

struct Runtime {
    manager: GlobalSystemMediaTransportControlsSessionManager,
    session: Option<GlobalSystemMediaTransportControlsSession>,
    playback_token: Option<i64>,
    callback: MediaCallback,
}

/// Event-driven Windows system media observer.
///
/// The worker thread only exists to own the WinRT apartment and subscriptions.
/// It blocks on a channel while idle; playback updates arrive through WinRT
/// events, so there is no polling loop.
pub struct MediaWatcher {
    stop_tx: Option<mpsc::Sender<()>>,
    thread: Option<JoinHandle<()>>,
}

impl MediaWatcher {
    pub fn start(callback: impl Fn(MediaState) + Send + Sync + 'static) -> std::io::Result<Self> {
        let (stop_tx, stop_rx) = mpsc::channel();
        let callback: MediaCallback = Arc::new(callback);

        let thread = thread::Builder::new()
            .name("sena-media".into())
            .spawn(move || run_media_thread(callback, stop_rx))?;

        Ok(Self {
            stop_tx: Some(stop_tx),
            thread: Some(thread),
        })
    }
}

impl Drop for MediaWatcher {
    fn drop(&mut self) {
        if let Some(stop_tx) = self.stop_tx.take() {
            let _ = stop_tx.send(());
        }

        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
    }
}

fn run_media_thread(callback: MediaCallback, stop_rx: mpsc::Receiver<()>) {
    if unsafe { RoInitialize(RO_INIT_MULTITHREADED) }.is_err() {
        return;
    }

    let result = run_media_runtime(callback, stop_rx);

    if let Err(error) = result {
        eprintln!("media watcher unavailable: {error}");
    }

    unsafe {
        RoUninitialize();
    }
}

fn run_media_runtime(
    callback: MediaCallback,
    stop_rx: mpsc::Receiver<()>,
) -> windows::core::Result<()> {
    let manager = GlobalSystemMediaTransportControlsSessionManager::RequestAsync()?.join()?;

    let runtime = Arc::new(Mutex::new(Runtime {
        manager: manager.clone(),
        session: None,
        playback_token: None,
        callback,
    }));

    let weak_runtime = Arc::downgrade(&runtime);
    let manager_handler = TypedEventHandler::<
        GlobalSystemMediaTransportControlsSessionManager,
        CurrentSessionChangedEventArgs,
    >::new(move |_, _| {
        if let Some(runtime) = weak_runtime.upgrade() {
            let _ = refresh_current_session(&runtime);
        }

        Ok(())
    });

    let manager_token = manager.CurrentSessionChanged(&manager_handler)?;
    refresh_current_session(&runtime)?;

    // No timeout: this thread consumes no periodic CPU while the media state is
    // unchanged. WinRT raises callbacks independently.
    let _ = stop_rx.recv();

    let (session, playback_token) = {
        let mut runtime = runtime.lock().expect("media runtime lock poisoned");
        (runtime.session.take(), runtime.playback_token.take())
    };

    if let (Some(session), Some(token)) = (session, playback_token) {
        let _ = session.RemovePlaybackInfoChanged(token);
    }

    let _ = manager.RemoveCurrentSessionChanged(manager_token);

    Ok(())
}

fn refresh_current_session(runtime: &Arc<Mutex<Runtime>>) -> windows::core::Result<()> {
    let (manager, old_session, old_token, callback) = {
        let mut runtime = runtime.lock().expect("media runtime lock poisoned");
        (
            runtime.manager.clone(),
            runtime.session.take(),
            runtime.playback_token.take(),
            runtime.callback.clone(),
        )
    };

    if let (Some(session), Some(token)) = (old_session, old_token) {
        let _ = session.RemovePlaybackInfoChanged(token);
    }

    let session = manager.GetCurrentSession().ok();
    let state = session
        .as_ref()
        .map(playback_state)
        .unwrap_or(MediaState::Stopped);

    let playback_token = if let Some(session) = session.as_ref() {
        let callback = callback.clone();
        let handler = TypedEventHandler::<
            GlobalSystemMediaTransportControlsSession,
            PlaybackInfoChangedEventArgs,
        >::new(move |sender, _| {
            let state = sender
                .as_ref()
                .map(playback_state)
                .unwrap_or(MediaState::Stopped);

            callback(state);
            Ok(())
        });

        Some(session.PlaybackInfoChanged(&handler)?)
    } else {
        None
    };

    {
        let mut runtime = runtime.lock().expect("media runtime lock poisoned");
        runtime.session = session;
        runtime.playback_token = playback_token;
    }

    callback(state);

    Ok(())
}

fn playback_state(session: &GlobalSystemMediaTransportControlsSession) -> MediaState {
    let status = session
        .GetPlaybackInfo()
        .and_then(|info| info.PlaybackStatus());

    match status {
        Ok(GlobalSystemMediaTransportControlsSessionPlaybackStatus::Playing) => MediaState::Playing,
        _ => MediaState::Stopped,
    }
}
