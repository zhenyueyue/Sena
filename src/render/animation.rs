use std::time::Duration;

use crate::{behavior::Behavior, pet::PetPackage};

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
    pub looping: bool,
}

impl AnimationClip {
    pub const fn from_i32(value: i32) -> Option<Self> {
        match value {
            0 => Some(Self::Static),
            1 => Some(Self::Coding),
            2 => Some(Self::Listening),
            3 => Some(Self::CodingWithMusic),
            4 => Some(Self::Drowsy),
            5 => Some(Self::Sleeping),
            _ => None,
        }
    }

    pub const fn behavior(self) -> Behavior {
        match self {
            Self::Static => Behavior::Idle,
            Self::Coding => Behavior::Coding,
            Self::Listening => Behavior::ListeningMusic,
            Self::CodingWithMusic => Behavior::CodingWithMusic,
            Self::Drowsy => Behavior::Drowsy,
            Self::Sleeping => Behavior::Sleeping,
        }
    }
}

impl AnimationSpec {
    pub fn for_runtime(
        behavior: Behavior,
        package: &PetPackage,
        typing_active: bool,
        music_motion_active: bool,
        drowsy_motion_active: bool,
    ) -> Self {
        let mut spec = Self::for_behavior(behavior, package);
        let resolved = package
            .animation_with_idle_fallback(behavior)
            .map(|(resolved, _)| resolved)
            .unwrap_or(behavior);

        let should_animate = match behavior {
            Behavior::Coding => typing_active,
            Behavior::ListeningMusic => resolved == Behavior::ListeningMusic && music_motion_active,
            Behavior::CodingWithMusic => match resolved {
                Behavior::Coding => typing_active,
                Behavior::ListeningMusic => music_motion_active,
                Behavior::CodingWithMusic => typing_active || music_motion_active,
                _ => true,
            },
            Behavior::Drowsy => resolved == Behavior::Drowsy && drowsy_motion_active,
            _ => true,
        };

        if !should_animate {
            spec.frame_count = 1;
            spec.interval = None;
            spec.looping = false;
        }

        spec
    }

    pub fn for_behavior(behavior: Behavior, package: &PetPackage) -> Self {
        let fallback = Self::fallback_for_behavior(behavior);

        let Some((_, definition)) = package.animation_with_idle_fallback(behavior) else {
            return fallback;
        };

        let frame_count = definition.effective_frame_count().min(i32::MAX as usize) as i32;
        let interval = definition.frame_duration_ms(0).map(Duration::from_millis);

        Self {
            clip: fallback.clip,
            frame_count,
            interval,
            looping: definition.looping,
        }
    }

    const fn fallback_for_behavior(behavior: Behavior) -> Self {
        match behavior {
            // Idle deliberately has no timer. A future idle blink can be scheduled
            // as an occasional one-shot instead of keeping a permanent frame loop.
            Behavior::Idle => Self {
                clip: AnimationClip::Static,
                frame_count: 1,
                interval: None,
                looping: true,
            },
            Behavior::Coding => Self {
                clip: AnimationClip::Coding,
                frame_count: 2,
                interval: Some(Duration::from_millis(160)),
                looping: true,
            },
            Behavior::ListeningMusic => Self {
                clip: AnimationClip::Listening,
                frame_count: 4,
                interval: Some(Duration::from_millis(240)),
                looping: true,
            },
            Behavior::CodingWithMusic => Self {
                clip: AnimationClip::CodingWithMusic,
                frame_count: 4,
                interval: Some(Duration::from_millis(160)),
                looping: true,
            },
            Behavior::Drowsy => Self {
                clip: AnimationClip::Drowsy,
                frame_count: 2,
                interval: Some(Duration::from_millis(900)),
                looping: true,
            },
            Behavior::Sleeping => Self {
                clip: AnimationClip::Sleeping,
                frame_count: 2,
                interval: Some(Duration::from_millis(1500)),
                looping: true,
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
        let package = PetPackage::builtin_placeholder();
        let spec = AnimationSpec::for_behavior(Behavior::Idle, &package);
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
            let package = PetPackage::builtin_placeholder();
            let spec = AnimationSpec::for_behavior(behavior, &package);
            let interval = spec.interval.expect("animated behavior needs interval");
            assert!(interval >= Duration::from_millis(160));
        }
    }

    #[test]
    fn sleeping_uses_a_very_low_frequency_animation() {
        let package = PetPackage::builtin_placeholder();
        let spec = AnimationSpec::for_behavior(Behavior::Sleeping, &package);
        assert_eq!(spec.interval, Some(Duration::from_millis(1500)));
    }

    #[test]
    fn placeholder_package_drives_animation_timing() {
        let package = PetPackage::load_from_dir(
            std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                .join("pets")
                .join("default"),
        )
        .expect("placeholder package should load");
        let spec = AnimationSpec::for_behavior(Behavior::ListeningMusic, &package);

        assert_eq!(spec.frame_count, 4);
        assert_eq!(spec.interval, Some(Duration::from_millis(240)));
    }

    #[test]
    fn official_package_uses_coding_animation_when_available() {
        let package = PetPackage::load_default().expect("official Sena package should load");
        let spec = AnimationSpec::for_behavior(Behavior::Coding, &package);

        assert_eq!(package.manifest().id, "sena.official");
        assert_eq!(spec.frame_count, 4);
        assert_eq!(spec.interval, Some(Duration::from_millis(220)));
        assert!(spec.running());
    }

    #[test]
    fn coding_stays_static_until_keyboard_activity() {
        let package = PetPackage::load_default().expect("official Sena package should load");
        let spec = AnimationSpec::for_runtime(Behavior::Coding, &package, false, false, false);

        assert_eq!(spec.clip, AnimationClip::Coding);
        assert_eq!(spec.frame_count, 1);
        assert_eq!(spec.interval, None);
        assert!(!spec.running());
    }

    #[test]
    fn coding_animates_during_keyboard_activity() {
        let package = PetPackage::load_default().expect("official Sena package should load");
        let spec = AnimationSpec::for_runtime(Behavior::Coding, &package, true, false, false);

        assert_eq!(spec.frame_count, 4);
        assert_eq!(spec.interval, Some(Duration::from_millis(220)));
        assert!(spec.running());
    }

    #[test]
    fn dedicated_listening_animation_can_sleep_between_motion_bursts() {
        let package = PetPackage::builtin_placeholder();
        let quiet =
            AnimationSpec::for_runtime(Behavior::ListeningMusic, &package, false, false, false);
        let moving =
            AnimationSpec::for_runtime(Behavior::ListeningMusic, &package, false, true, false);

        assert!(!quiet.running());
        assert_eq!(quiet.frame_count, 1);
        assert!(moving.running());
        assert_eq!(moving.frame_count, 4);
    }

    #[test]
    fn coding_with_music_reuses_coding_when_combined_assets_are_missing() {
        let package = PetPackage::load_default().expect("official Sena package should load");
        let quiet =
            AnimationSpec::for_runtime(Behavior::CodingWithMusic, &package, false, false, false);
        let typing =
            AnimationSpec::for_runtime(Behavior::CodingWithMusic, &package, true, false, false);

        assert!(!quiet.running());
        assert_eq!(quiet.frame_count, 1);
        assert!(typing.running());
        assert_eq!(typing.frame_count, 4);
    }

    #[test]
    fn drowsy_animation_can_sleep_between_motion_bursts() {
        let package = PetPackage::builtin_placeholder();
        let quiet = AnimationSpec::for_runtime(Behavior::Drowsy, &package, false, false, false);
        let motion = AnimationSpec::for_runtime(Behavior::Drowsy, &package, false, false, true);

        assert!(!quiet.running());
        assert_eq!(quiet.frame_count, 1);
        assert!(motion.running());
        assert_eq!(motion.frame_count, 2);
    }

    #[test]
    fn official_drowsy_fallback_stays_static_until_assets_exist() {
        let package = PetPackage::load_default().expect("official Sena package should load");
        let quiet = AnimationSpec::for_runtime(Behavior::Drowsy, &package, false, false, false);
        let motion = AnimationSpec::for_runtime(Behavior::Drowsy, &package, false, false, true);

        assert!(!quiet.running());
        assert_eq!(quiet.frame_count, 1);
        assert!(!motion.running());
        assert_eq!(motion.frame_count, 1);
    }

    #[test]
    fn official_listening_assets_sleep_between_motion_bursts() {
        let package = PetPackage::load_default().expect("official Sena package should load");
        let quiet =
            AnimationSpec::for_runtime(Behavior::ListeningMusic, &package, false, false, false);
        let motion =
            AnimationSpec::for_runtime(Behavior::ListeningMusic, &package, false, true, false);

        assert_eq!(package.manifest().id, "sena.official");
        assert!(!quiet.running());
        assert_eq!(quiet.frame_count, 1);
        assert!(motion.running());
        assert_eq!(motion.frame_count, 4);
        assert_eq!(motion.interval, Some(Duration::from_millis(240)));
    }
}
