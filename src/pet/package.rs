use std::{
    collections::BTreeMap,
    env, fmt, fs,
    path::{Component, Path, PathBuf},
};

use serde::Deserialize;

use crate::behavior::Behavior;

const SUPPORTED_SCHEMA_VERSION: u32 = 1;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RendererKind {
    Placeholder,
    Sprite,
    Live2d,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AnimationKey {
    Idle,
    Coding,
    ListeningMusic,
    CodingWithMusic,
    Drowsy,
    Sleeping,
}

impl From<Behavior> for AnimationKey {
    fn from(value: Behavior) -> Self {
        match value {
            Behavior::Idle => Self::Idle,
            Behavior::Coding => Self::Coding,
            Behavior::ListeningMusic => Self::ListeningMusic,
            Behavior::CodingWithMusic => Self::CodingWithMusic,
            Behavior::Drowsy => Self::Drowsy,
            Behavior::Sleeping => Self::Sleeping,
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct AnimationDefinition {
    #[serde(default)]
    pub frames: Vec<String>,
    #[serde(default)]
    pub frame_count: Option<u32>,
    #[serde(default)]
    pub interval_ms: Option<u64>,
    #[serde(default)]
    pub frame_durations_ms: Vec<u64>,
    #[serde(default = "default_looping")]
    pub looping: bool,
}

impl AnimationDefinition {
    pub fn effective_frame_count(&self) -> usize {
        if self.frames.is_empty() {
            self.frame_count.unwrap_or(1).max(1) as usize
        } else {
            self.frames.len()
        }
    }

    pub fn frame_duration_ms(&self, frame: usize) -> Option<u64> {
        self.frame_durations_ms
            .get(frame)
            .copied()
            .or(self.interval_ms)
            .filter(|milliseconds| *milliseconds > 0)
    }
}

const fn default_looping() -> bool {
    true
}

#[derive(Debug, Clone, Deserialize)]
pub struct SpriteSettings {
    #[serde(default = "default_sprite_scale")]
    pub scale: f32,
    #[serde(default = "default_alpha_threshold")]
    pub alpha_threshold: u8,
}

impl Default for SpriteSettings {
    fn default() -> Self {
        Self {
            scale: default_sprite_scale(),
            alpha_threshold: default_alpha_threshold(),
        }
    }
}

const fn default_sprite_scale() -> f32 {
    1.0
}

const fn default_alpha_threshold() -> u8 {
    8
}

#[derive(Debug, Clone, Deserialize)]
pub struct PetManifest {
    pub schema_version: u32,
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub display_name: Option<String>,
    pub version: String,
    pub author: String,
    pub renderer: RendererKind,
    #[serde(default)]
    pub sprite: SpriteSettings,
    #[serde(default)]
    pub license: Option<String>,
    #[serde(default)]
    pub animations: BTreeMap<AnimationKey, AnimationDefinition>,
}

#[derive(Debug, Clone)]
pub struct PetPackage {
    root: PathBuf,
    manifest: PetManifest,
}

impl PetPackage {
    pub fn builtin_placeholder() -> Self {
        let animations = [
            (
                AnimationKey::Idle,
                AnimationDefinition {
                    frames: Vec::new(),
                    frame_count: Some(1),
                    interval_ms: None,
                    frame_durations_ms: Vec::new(),
                    looping: true,
                },
            ),
            (
                AnimationKey::Coding,
                AnimationDefinition {
                    frames: Vec::new(),
                    frame_count: Some(2),
                    interval_ms: Some(160),
                    frame_durations_ms: Vec::new(),
                    looping: true,
                },
            ),
            (
                AnimationKey::ListeningMusic,
                AnimationDefinition {
                    frames: Vec::new(),
                    frame_count: Some(4),
                    interval_ms: Some(240),
                    frame_durations_ms: Vec::new(),
                    looping: true,
                },
            ),
            (
                AnimationKey::CodingWithMusic,
                AnimationDefinition {
                    frames: Vec::new(),
                    frame_count: Some(4),
                    interval_ms: Some(160),
                    frame_durations_ms: Vec::new(),
                    looping: true,
                },
            ),
            (
                AnimationKey::Drowsy,
                AnimationDefinition {
                    frames: Vec::new(),
                    frame_count: Some(2),
                    interval_ms: Some(900),
                    frame_durations_ms: Vec::new(),
                    looping: true,
                },
            ),
            (
                AnimationKey::Sleeping,
                AnimationDefinition {
                    frames: Vec::new(),
                    frame_count: Some(2),
                    interval_ms: Some(1500),
                    frame_durations_ms: Vec::new(),
                    looping: true,
                },
            ),
        ]
        .into_iter()
        .collect();

        Self {
            root: PathBuf::new(),
            manifest: PetManifest {
                schema_version: SUPPORTED_SCHEMA_VERSION,
                id: "sena.builtin".into(),
                name: "Sena".into(),
                display_name: Some("星奈".into()),
                version: env!("CARGO_PKG_VERSION").into(),
                author: "Sena Project".into(),
                renderer: RendererKind::Placeholder,
                sprite: SpriteSettings::default(),
                license: Some("Apache-2.0".into()),
                animations,
            },
        }
    }

    pub fn load_default() -> Result<Self, PackageError> {
        let candidates = default_package_candidates();
        let mut first_error = None;

        for candidate in &candidates {
            if !candidate.join("pet.json").is_file() {
                continue;
            }

            match Self::load_from_dir(candidate) {
                Ok(package) => return Ok(package),
                Err(error) => {
                    if first_error.is_none() {
                        first_error = Some(error);
                    }
                }
            }
        }

        match first_error {
            Some(error) => Err(error),
            None => Err(PackageError::NotFound(candidates)),
        }
    }

    pub fn load_from_dir(root: impl AsRef<Path>) -> Result<Self, PackageError> {
        let root = root.as_ref().to_path_buf();
        let manifest_path = root.join("pet.json");
        let json =
            fs::read_to_string(&manifest_path).map_err(|source| PackageError::ReadManifest {
                path: manifest_path.clone(),
                source,
            })?;
        let manifest: PetManifest =
            serde_json::from_str(&json).map_err(|source| PackageError::ParseManifest {
                path: manifest_path.clone(),
                source,
            })?;

        validate_manifest(&root, &manifest)?;

        Ok(Self { root, manifest })
    }

    pub fn manifest(&self) -> &PetManifest {
        &self.manifest
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    pub fn animation(&self, behavior: Behavior) -> Option<&AnimationDefinition> {
        self.manifest.animations.get(&AnimationKey::from(behavior))
    }

    pub fn animation_with_idle_fallback(
        &self,
        behavior: Behavior,
    ) -> Option<(Behavior, &AnimationDefinition)> {
        if let Some(definition) = self.animation(behavior)
            && self.animation_is_renderable(definition)
        {
            return Some((behavior, definition));
        }

        if behavior != Behavior::Idle
            && let Some(idle) = self.animation(Behavior::Idle)
            && self.animation_is_renderable(idle)
        {
            return Some((Behavior::Idle, idle));
        }

        None
    }

    fn animation_is_renderable(&self, definition: &AnimationDefinition) -> bool {
        !self.is_sprite() || !definition.frames.is_empty()
    }

    pub fn is_sprite(&self) -> bool {
        self.manifest.renderer == RendererKind::Sprite
    }

    pub fn sprite_settings(&self) -> &SpriteSettings {
        &self.manifest.sprite
    }

    pub fn sprite_frame_path(&self, behavior: Behavior, frame: usize) -> Option<PathBuf> {
        if !self.is_sprite() {
            return None;
        }

        let (_, definition) = self.animation_with_idle_fallback(behavior)?;
        let relative = definition
            .frames
            .get(frame)
            .or_else(|| definition.frames.first())?;
        Some(self.root.join(relative))
    }

    pub fn animation_frame_duration_ms(&self, behavior: Behavior, frame: usize) -> Option<u64> {
        let (_, definition) = self.animation_with_idle_fallback(behavior)?;
        definition.frame_duration_ms(frame)
    }
}

fn default_package_candidates() -> Vec<PathBuf> {
    let mut candidates = Vec::new();

    if let Some(explicit) = env::var_os("SENA_PET_PACKAGE") {
        candidates.push(PathBuf::from(explicit));
    }

    if let Ok(executable) = env::current_exe()
        && let Some(directory) = executable.parent()
    {
        candidates.push(directory.join("pets").join("sena"));
    }

    candidates.push(
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("pets")
            .join("sena"),
    );

    if let Ok(executable) = env::current_exe()
        && let Some(directory) = executable.parent()
    {
        candidates.push(directory.join("pets").join("default"));
    }

    candidates.push(
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("pets")
            .join("default"),
    );

    candidates
}

fn validate_manifest(root: &Path, manifest: &PetManifest) -> Result<(), PackageError> {
    if manifest.schema_version != SUPPORTED_SCHEMA_VERSION {
        return Err(PackageError::UnsupportedSchema {
            found: manifest.schema_version,
            supported: SUPPORTED_SCHEMA_VERSION,
        });
    }

    if manifest.id.trim().is_empty() || manifest.name.trim().is_empty() {
        return Err(PackageError::InvalidManifest(
            "id and name must not be empty".into(),
        ));
    }

    if manifest.renderer == RendererKind::Sprite
        && (!manifest.sprite.scale.is_finite() || !(0.1..=4.0).contains(&manifest.sprite.scale))
    {
        return Err(PackageError::InvalidManifest(
            "sprite.scale must be between 0.1 and 4.0".into(),
        ));
    }

    for (animation, definition) in &manifest.animations {
        let frame_count = definition.effective_frame_count();
        if frame_count == 0 {
            return Err(PackageError::InvalidManifest(format!(
                "{animation:?} must contain at least one frame"
            )));
        }

        if !definition.frame_durations_ms.is_empty() {
            if definition.frame_durations_ms.len() != frame_count {
                return Err(PackageError::InvalidManifest(format!(
                    "{animation:?} frame_durations_ms must have exactly {frame_count} entries"
                )));
            }

            if definition
                .frame_durations_ms
                .iter()
                .any(|milliseconds| *milliseconds == 0)
            {
                return Err(PackageError::InvalidManifest(format!(
                    "{animation:?} frame durations must be greater than zero"
                )));
            }
        }

        for frame in &definition.frames {
            let relative = Path::new(frame);
            if !is_safe_relative_path(relative) {
                return Err(PackageError::UnsafeAssetPath(frame.clone()));
            }

            if manifest.renderer == RendererKind::Sprite {
                if !is_supported_sprite_asset(relative) {
                    return Err(PackageError::UnsupportedAssetFormat(frame.clone()));
                }

                if !root.join(relative).is_file() {
                    return Err(PackageError::MissingAsset(root.join(relative)));
                }
            }
        }
    }

    if manifest.renderer == RendererKind::Sprite
        && manifest
            .animations
            .values()
            .all(|animation| animation.frames.is_empty())
    {
        return Err(PackageError::InvalidManifest(
            "sprite packages must provide at least one frame asset".into(),
        ));
    }

    Ok(())
}

fn is_safe_relative_path(path: &Path) -> bool {
    !path.as_os_str().is_empty()
        && !path.is_absolute()
        && path
            .components()
            .all(|component| matches!(component, Component::Normal(_)))
}

fn is_supported_sprite_asset(path: &Path) -> bool {
    path.extension()
        .and_then(|extension| extension.to_str())
        .is_some_and(|extension| {
            extension.eq_ignore_ascii_case("png") || extension.eq_ignore_ascii_case("webp")
        })
}

#[derive(Debug)]
pub enum PackageError {
    NotFound(Vec<PathBuf>),
    ReadManifest {
        path: PathBuf,
        source: std::io::Error,
    },
    ParseManifest {
        path: PathBuf,
        source: serde_json::Error,
    },
    UnsupportedSchema {
        found: u32,
        supported: u32,
    },
    InvalidManifest(String),
    UnsafeAssetPath(String),
    UnsupportedAssetFormat(String),
    MissingAsset(PathBuf),
}

impl fmt::Display for PackageError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NotFound(paths) => write!(
                f,
                "no pet package found in {}",
                paths
                    .iter()
                    .map(|path| path.display().to_string())
                    .collect::<Vec<_>>()
                    .join(", ")
            ),
            Self::ReadManifest { path, source } => {
                write!(f, "failed to read {}: {source}", path.display())
            }
            Self::ParseManifest { path, source } => {
                write!(f, "failed to parse {}: {source}", path.display())
            }
            Self::UnsupportedSchema { found, supported } => write!(
                f,
                "pet package schema {found} is not supported; expected {supported}"
            ),
            Self::InvalidManifest(message) => write!(f, "invalid pet package: {message}"),
            Self::UnsafeAssetPath(path) => {
                write!(f, "pet package contains unsafe asset path: {path}")
            }
            Self::UnsupportedAssetFormat(path) => {
                write!(
                    f,
                    "unsupported sprite asset format: {path}; expected PNG or WebP"
                )
            }
            Self::MissingAsset(path) => {
                write!(f, "pet package asset does not exist: {}", path.display())
            }
        }
    }
}

impl std::error::Error for PackageError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bundled_default_package_loads() {
        let package = PetPackage::load_from_dir(
            PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                .join("pets")
                .join("default"),
        )
        .expect("bundled default package should load");

        assert_eq!(package.manifest().id, "sena.default");
        assert_eq!(package.manifest().renderer, RendererKind::Placeholder);
        assert_eq!(
            package
                .animation(Behavior::Sleeping)
                .expect("sleeping animation")
                .interval_ms,
            Some(1500)
        );
    }

    #[test]
    fn rejects_asset_paths_that_escape_package_root() {
        assert!(!is_safe_relative_path(Path::new("../outside.webp")));
        assert!(!is_safe_relative_path(Path::new("/absolute.webp")));
        assert!(is_safe_relative_path(Path::new("animations/idle/000.webp")));
    }

    #[test]
    fn animation_uses_real_frames_before_declared_placeholder_count() {
        let definition = AnimationDefinition {
            frames: vec!["a.webp".into(), "b.webp".into(), "c.webp".into()],
            frame_count: Some(99),
            interval_ms: Some(200),
            frame_durations_ms: Vec::new(),
            looping: true,
        };

        assert_eq!(definition.effective_frame_count(), 3);
    }

    #[test]
    fn sprite_assets_are_limited_to_png_and_webp() {
        assert!(is_supported_sprite_asset(Path::new("idle.PNG")));
        assert!(is_supported_sprite_asset(Path::new("idle.webp")));
        assert!(!is_supported_sprite_asset(Path::new("idle.jpg")));
        assert!(!is_supported_sprite_asset(Path::new("model.moc3")));
    }

    #[test]
    fn sprite_settings_have_safe_defaults() {
        let settings = SpriteSettings::default();
        assert_eq!(settings.scale, 1.0);
        assert_eq!(settings.alpha_threshold, 8);
    }

    #[test]
    fn official_sena_package_template_matches_current_schema() {
        let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("pets")
            .join("sena")
            .join("pet.template.json");
        let json = fs::read_to_string(path).expect("Sena pet template should be readable");
        let manifest: PetManifest =
            serde_json::from_str(&json).expect("Sena pet template should match schema");

        assert_eq!(manifest.id, "sena.official");
        assert_eq!(manifest.renderer, RendererKind::Sprite);
        assert_eq!(manifest.sprite.scale, 0.36);
        assert_eq!(manifest.animations.len(), 6);
        assert_eq!(
            manifest
                .animations
                .get(&AnimationKey::Idle)
                .expect("idle animation")
                .frames
                .len(),
            4
        );
    }

    #[test]
    fn sprite_behavior_without_frames_falls_back_to_idle() {
        let mut package = PetPackage::builtin_placeholder();
        package.manifest.renderer = RendererKind::Sprite;
        package.manifest.animations.clear();
        package.manifest.animations.insert(
            AnimationKey::Idle,
            AnimationDefinition {
                frames: vec!["animations/idle/000.webp".into()],
                frame_count: None,
                interval_ms: None,
                frame_durations_ms: Vec::new(),
                looping: true,
            },
        );
        package.manifest.animations.insert(
            AnimationKey::Coding,
            AnimationDefinition {
                frames: Vec::new(),
                frame_count: None,
                interval_ms: Some(180),
                frame_durations_ms: Vec::new(),
                looping: true,
            },
        );

        let (resolved, animation) = package
            .animation_with_idle_fallback(Behavior::Coding)
            .expect("coding should fall back to idle");

        assert_eq!(resolved, Behavior::Idle);
        assert_eq!(animation.frames, vec!["animations/idle/000.webp"]);
        assert_eq!(
            package.sprite_frame_path(Behavior::Coding, 0),
            Some(PathBuf::from("animations/idle/000.webp"))
        );
    }

    #[test]
    fn per_frame_duration_overrides_uniform_interval() {
        let definition = AnimationDefinition {
            frames: vec!["000.webp".into(), "001.webp".into()],
            frame_count: None,
            interval_ms: Some(650),
            frame_durations_ms: vec![1800, 120],
            looping: true,
        };

        assert_eq!(definition.frame_duration_ms(0), Some(1800));
        assert_eq!(definition.frame_duration_ms(1), Some(120));
    }

    #[test]
    fn uniform_interval_remains_backward_compatible() {
        let definition = AnimationDefinition {
            frames: vec!["000.webp".into(), "001.webp".into()],
            frame_count: None,
            interval_ms: Some(240),
            frame_durations_ms: Vec::new(),
            looping: true,
        };

        assert_eq!(definition.frame_duration_ms(0), Some(240));
        assert_eq!(definition.frame_duration_ms(1), Some(240));
    }

    #[test]
    fn official_sena_idle_keeps_staggered_blinks() {
        let package = PetPackage::load_default().expect("official Sena package should load");
        let idle = package
            .animation(Behavior::Idle)
            .expect("official Sena package should contain Idle");

        assert_eq!(
            idle.frames,
            vec![
                "animations/idle/000.webp",
                "animations/idle/001.webp",
                "animations/idle/002.webp",
                "animations/idle/003.webp",
            ]
        );
        assert_eq!(idle.frame_durations_ms, vec![1800, 120, 2300, 120]);
    }
}
