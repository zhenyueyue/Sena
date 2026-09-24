use crate::context::{DesktopContext, MediaState, UserActivity};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Behavior {
    #[default]
    Idle,
    Coding,
    ListeningMusic,
    CodingWithMusic,
    Drowsy,
    Sleeping,
}

#[derive(Debug, Default)]
pub struct BehaviorEngine;

impl BehaviorEngine {
    pub fn resolve(&self, context: &DesktopContext) -> Behavior {
        if context.session_locked {
            return Behavior::Sleeping;
        }

        if context.media != MediaState::Playing {
            match context.user_activity {
                UserActivity::Idle => return Behavior::Sleeping,
                UserActivity::Drowsy => return Behavior::Drowsy,
                UserActivity::Active => {}
            }
        }

        match (context.is_coding(), context.media == MediaState::Playing) {
            (true, true) => Behavior::CodingWithMusic,
            (true, false) => Behavior::Coding,
            (false, true) => Behavior::ListeningMusic,
            (false, false) => Behavior::Idle,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn combines_coding_and_music_without_needing_a_separate_context_rule() {
        let context = DesktopContext {
            foreground_process: Some("codex.exe".into()),
            media: MediaState::Playing,
            user_activity: UserActivity::Active,
            session_locked: false,
            typing_active: false,
            music_motion_active: false,
            drowsy_motion_active: false,
        };

        assert_eq!(BehaviorEngine.resolve(&context), Behavior::CodingWithMusic);
    }

    #[test]
    fn locked_session_has_priority() {
        let context = DesktopContext {
            foreground_process: Some("codex.exe".into()),
            media: MediaState::Playing,
            user_activity: UserActivity::Active,
            session_locked: true,
            typing_active: false,
            music_motion_active: false,
            drowsy_motion_active: false,
        };

        assert_eq!(BehaviorEngine.resolve(&context), Behavior::Sleeping);
    }

    #[test]
    fn inactive_user_gets_drowsy_before_sleeping() {
        let context = DesktopContext {
            user_activity: UserActivity::Drowsy,
            ..DesktopContext::default()
        };

        assert_eq!(BehaviorEngine.resolve(&context), Behavior::Drowsy);
    }

    #[test]
    fn active_media_prevents_input_idle_from_forcing_sleep() {
        let context = DesktopContext {
            media: MediaState::Playing,
            user_activity: UserActivity::Idle,
            ..DesktopContext::default()
        };

        assert_eq!(BehaviorEngine.resolve(&context), Behavior::ListeningMusic);
    }
}
