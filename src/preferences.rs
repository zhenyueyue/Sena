use std::{env, fs, path::PathBuf};

use serde::{Deserialize, Serialize};

const DEFAULT_SCALE: f32 = 1.0;
const MIN_SCALE: f32 = 0.6;
const MAX_SCALE: f32 = 1.4;
const DEFAULT_AUTONOMOUS_FREQUENCY: u8 = 1;
const MAX_AUTONOMOUS_FREQUENCY: u8 = 2;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(default)]
pub struct Preferences {
    pub window_x: Option<i32>,
    pub window_y: Option<i32>,
    pub scale: f32,
    pub always_on_top: bool,
    pub onboarding_completed: bool,
    pub autonomous_behavior_enabled: bool,
    pub speech_bubbles_enabled: bool,
    pub autonomous_frequency: u8,
}

impl Default for Preferences {
    fn default() -> Self {
        Self {
            window_x: None,
            window_y: None,
            scale: DEFAULT_SCALE,
            always_on_top: true,
            onboarding_completed: false,
            autonomous_behavior_enabled: true,
            speech_bubbles_enabled: true,
            autonomous_frequency: DEFAULT_AUTONOMOUS_FREQUENCY,
        }
    }
}

impl Preferences {
    pub fn normalized(mut self) -> Self {
        if !self.scale.is_finite() {
            self.scale = DEFAULT_SCALE;
        }
        self.scale = self.scale.clamp(MIN_SCALE, MAX_SCALE);
        self.autonomous_frequency = self.autonomous_frequency.min(MAX_AUTONOMOUS_FREQUENCY);
        self
    }

    pub fn position(&self) -> Option<(i32, i32)> {
        Some((self.window_x?, self.window_y?))
    }
}

#[derive(Debug)]
pub struct PreferencesStore {
    path: PathBuf,
    value: Preferences,
}

impl PreferencesStore {
    pub fn load_default() -> Self {
        Self::load_from_path(default_preferences_path())
    }

    pub fn load_from_path(path: PathBuf) -> Self {
        let value = fs::read_to_string(&path)
            .ok()
            .and_then(|json| serde_json::from_str::<Preferences>(&json).ok())
            .unwrap_or_default()
            .normalized();

        Self { path, value }
    }

    pub fn value(&self) -> &Preferences {
        &self.value
    }

    pub fn set_position(&mut self, x: i32, y: i32) {
        self.value.window_x = Some(x);
        self.value.window_y = Some(y);
    }

    pub fn set_scale(&mut self, scale: f32) {
        self.value.scale = if scale.is_finite() {
            scale.clamp(MIN_SCALE, MAX_SCALE)
        } else {
            DEFAULT_SCALE
        };
    }

    pub fn set_always_on_top(&mut self, enabled: bool) {
        self.value.always_on_top = enabled;
    }

    pub fn set_onboarding_completed(&mut self, completed: bool) {
        self.value.onboarding_completed = completed;
    }

    pub fn set_autonomous_behavior_enabled(&mut self, enabled: bool) {
        self.value.autonomous_behavior_enabled = enabled;
    }

    pub fn set_speech_bubbles_enabled(&mut self, enabled: bool) {
        self.value.speech_bubbles_enabled = enabled;
    }

    pub fn set_autonomous_frequency(&mut self, frequency: u8) {
        self.value.autonomous_frequency = frequency.min(MAX_AUTONOMOUS_FREQUENCY);
    }

    pub fn save(&self) -> std::io::Result<()> {
        if let Some(parent) = self.path.parent() {
            fs::create_dir_all(parent)?;
        }

        let json = serde_json::to_string_pretty(&self.value)
            .map_err(|error| std::io::Error::other(error.to_string()))?;
        fs::write(&self.path, json)
    }
}

fn default_preferences_path() -> PathBuf {
    env::var_os("APPDATA")
        .map(PathBuf::from)
        .unwrap_or_else(|| env::current_dir().unwrap_or_else(|_| PathBuf::from(".")))
        .join("Sena")
        .join("preferences.json")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_path(name: &str) -> PathBuf {
        std::env::temp_dir().join(format!(
            "sena-preferences-{name}-{}-{}.json",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("system time should be after epoch")
                .as_nanos()
        ))
    }

    #[test]
    fn missing_preferences_use_safe_defaults() {
        let path = temp_path("missing");
        let store = PreferencesStore::load_from_path(path);

        assert_eq!(store.value(), &Preferences::default());
    }

    #[test]
    fn preferences_round_trip_position_and_scale() {
        let path = temp_path("roundtrip");
        let mut store = PreferencesStore::load_from_path(path.clone());
        store.set_position(321, 654);
        store.set_scale(1.2);
        store.set_always_on_top(false);
        store.set_onboarding_completed(true);
        store.set_autonomous_behavior_enabled(false);
        store.set_speech_bubbles_enabled(false);
        store.set_autonomous_frequency(2);
        store.save().expect("preferences should save");

        let loaded = PreferencesStore::load_from_path(path.clone());
        let _ = fs::remove_file(path);

        assert_eq!(loaded.value().position(), Some((321, 654)));
        assert_eq!(loaded.value().scale, 1.2);
        assert!(!loaded.value().always_on_top);
        assert!(loaded.value().onboarding_completed);
        assert!(!loaded.value().autonomous_behavior_enabled);
        assert!(!loaded.value().speech_bubbles_enabled);
        assert_eq!(loaded.value().autonomous_frequency, 2);
    }

    #[test]
    fn invalid_scale_is_clamped() {
        let path = temp_path("clamp");
        fs::write(&path, r#"{"window_x":1,"window_y":2,"scale":9.0}"#)
            .expect("fixture should write");

        let loaded = PreferencesStore::load_from_path(path.clone());
        let _ = fs::remove_file(path);

        assert_eq!(loaded.value().scale, MAX_SCALE);
        assert!(loaded.value().always_on_top);
        assert!(!loaded.value().onboarding_completed);
        assert!(loaded.value().autonomous_behavior_enabled);
        assert!(loaded.value().speech_bubbles_enabled);
        assert_eq!(
            loaded.value().autonomous_frequency,
            DEFAULT_AUTONOMOUS_FREQUENCY
        );
    }

    #[test]
    fn invalid_autonomous_frequency_is_clamped() {
        let path = temp_path("frequency-clamp");
        fs::write(&path, r#"{"autonomous_frequency":99}"#).expect("fixture should write");

        let loaded = PreferencesStore::load_from_path(path.clone());
        let _ = fs::remove_file(path);

        assert_eq!(
            loaded.value().autonomous_frequency,
            MAX_AUTONOMOUS_FREQUENCY
        );
    }
}
