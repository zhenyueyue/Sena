use std::time::Duration;

use crate::behavior::Behavior;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(i32)]
pub enum AnimationClip {
    Static = 0,
    Coding = 1,
    Listening = 2,
    CodingWithMusic = 3,
    Drowsy = 4,
    Sleeping = 5,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AnimationSpec {
    pub clip: AnimationClip,
    pub frame_count: i32,
    pub interval: Option<Duration>,
}

impl AnimationSpec {
    pub const fn for_behavior(behavior: Behavior) -> Self {
        match behavior {
            // Idle deliberately has no timer. A future idle blink can be scheduled
            // as an occasional one-shot instead of keeping a permanent frame loop.
            Behavior::Idle => Self {
                clip: AnimationClip::Static,
                frame_count: 1,
                interval: None,
            },
            Behavior::Coding => Self {
                clip: AnimationClip::Coding,
                frame_count: 2,
                interval: Some(Duration::from_millis(160)),
            },
            Behavior::ListeningMusic => Self {
                clip: AnimationClip::Listening,
                frame_count: 4,
                interval: Some(Duration::from_millis(240)),
            },
            Behavior::CodingWithMusic => Self {
                clip: AnimationClip::CodingWithMusic,
                frame_count: 4,
                interval: Some(Duration::from_millis(160)),
            },
            Behavior::Drowsy => Self {
                clip: AnimationClip::Drowsy,
                frame_count: 2,
                interval: Some(Duration::from_millis(900)),
            },
            Behavior::Sleeping => Self {
                clip: AnimationClip::Sleeping,
                frame_count: 2,
                interval: Some(Duration::from_millis(1500)),
            },
        }
    }

    pub fn interval_ms(self) -> i32 {
        self.interval
            .map(|duration| duration.as_millis().min(i32::MAX as u128) as i32)
            .unwrap_or(1000)
    }

    pub const fn running(self) -> bool {
        self.interval.is_some() && self.frame_count > 1
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn idle_has_no_permanent_animation_timer() {
        let spec = AnimationSpec::for_behavior(Behavior::Idle);
        assert!(!spec.running());
        assert_eq!(spec.frame_count, 1);
    }

    #[test]
    fn active_animation_rates_stay_well_below_sixty_fps() {
        for behavior in [
            Behavior::Coding,
            Behavior::ListeningMusic,
            Behavior::CodingWithMusic,
            Behavior::Drowsy,
            Behavior::Sleeping,
        ] {
            let spec = AnimationSpec::for_behavior(behavior);
            let interval = spec.interval.expect("animated behavior needs interval");
            assert!(interval >= Duration::from_millis(160));
        }
    }

    #[test]
    fn sleeping_uses_a_very_low_frequency_animation() {
        let spec = AnimationSpec::for_behavior(Behavior::Sleeping);
        assert_eq!(spec.interval, Some(Duration::from_millis(1500)));
    }
}
