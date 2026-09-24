//! Lightweight snapshot of the desktop state.
//!
//! Watchers will update this model from OS events. The model itself has no
//! polling loop, timers, or background thread.

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum MediaState {
    #[default]
    Stopped,
    Playing,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum UserActivity {
    #[default]
    Active,
    Drowsy,
    Idle,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct DesktopContext {
    pub foreground_process: Option<String>,
    pub media: MediaState,
    pub user_activity: UserActivity,
    pub session_locked: bool,
    pub typing_active: bool,
    pub music_motion_active: bool,
    pub drowsy_motion_active: bool,
}

impl DesktopContext {
    pub fn is_coding(&self) -> bool {
        self.foreground_process
            .as_deref()
            .is_some_and(is_known_coding_process)
    }
}

fn is_known_coding_process(process: &str) -> bool {
    const CODING_PROCESSES: &[&str] = &[
        "codex.exe",
        "code.exe",
        "cursor.exe",
        "idea64.exe",
        "clion64.exe",
        "rustrover64.exe",
    ];

    CODING_PROCESSES
        .iter()
        .any(|candidate| process.eq_ignore_ascii_case(candidate))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn recognizes_coding_process_case_insensitively() {
        let context = DesktopContext {
            foreground_process: Some("CoDeX.ExE".into()),
            ..DesktopContext::default()
        };

        assert!(context.is_coding());
    }
}
