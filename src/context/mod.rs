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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DayPhase {
    Morning,
    Day,
    Evening,
    LateNight,
}

impl DayPhase {
    pub const fn from_hour(hour: u8) -> Self {
        match hour % 24 {
            6..=10 => Self::Morning,
            11..=17 => Self::Day,
            18..=22 => Self::Evening,
            _ => Self::LateNight,
        }
    }

    pub const fn idle_threshold_seconds(self) -> (u64, u64) {
        match self {
            Self::Morning | Self::Day => (5 * 60, 10 * 60),
            Self::Evening => (4 * 60, 9 * 60),
            Self::LateNight => (3 * 60, 7 * 60),
        }
    }
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
    pub sleeping_motion_active: bool,
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
    fn day_phase_boundaries_match_local_clock_periods() {
        assert_eq!(DayPhase::from_hour(0), DayPhase::LateNight);
        assert_eq!(DayPhase::from_hour(5), DayPhase::LateNight);
        assert_eq!(DayPhase::from_hour(6), DayPhase::Morning);
        assert_eq!(DayPhase::from_hour(10), DayPhase::Morning);
        assert_eq!(DayPhase::from_hour(11), DayPhase::Day);
        assert_eq!(DayPhase::from_hour(17), DayPhase::Day);
        assert_eq!(DayPhase::from_hour(18), DayPhase::Evening);
        assert_eq!(DayPhase::from_hour(22), DayPhase::Evening);
        assert_eq!(DayPhase::from_hour(23), DayPhase::LateNight);
    }

    #[test]
    fn late_night_idle_thresholds_are_gentler_than_daytime() {
        assert_eq!(DayPhase::Day.idle_threshold_seconds(), (300, 600));
        assert_eq!(DayPhase::Evening.idle_threshold_seconds(), (240, 540));
        assert_eq!(DayPhase::LateNight.idle_threshold_seconds(), (180, 420));
    }

    #[test]
    fn recognizes_coding_process_case_insensitively() {
        let context = DesktopContext {
            foreground_process: Some("CoDeX.ExE".into()),
            ..DesktopContext::default()
        };

        assert!(context.is_coding());
    }
}
