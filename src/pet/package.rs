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
    Spine,
    Model3d,
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum InteractionAnimationKey {
    Petting,
    Stretch,
    LookAtCat,
    Daydream,
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

#[allow(dead_code)]
#[derive(Debug, Clone, Deserialize)]
pub struct Model3dSettings {
    pub source: String,
    #[serde(default = "default_model_scale")]
    pub scale: f32,
    #[serde(default)]
    pub motions: BTreeMap<String, String>,
    #[serde(default)]
    pub expressions: BTreeMap<String, String>,
    #[serde(default)]
    pub anchors: BTreeMap<String, String>,
}

const fn default_model_scale() -> f32 {
    1.0
}

#[allow(dead_code)]
#[derive(Debug, Clone, Deserialize)]
pub struct SpineSettings {
    pub skeleton: String,
    pub atlas: String,
    #[serde(default = "default_spine_scale")]
    pub scale: f32,
    #[serde(default = "default_spine_skin")]
    pub default_skin: String,
}

const fn default_spine_scale() -> f32 {
    1.0
}

fn default_spine_skin() -> String {
    "base".into()
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
    pub model3d: Option<Model3dSettings>,
    #[serde(default)]
    pub spine: Option<SpineSettings>,
    #[serde(default)]
    pub license: Option<String>,
    #[serde(default)]
    pub animations: BTreeMap<AnimationKey, AnimationDefinition>,
    #[serde(default)]
    pub interactions: BTreeMap<InteractionAnimationKey, AnimationDefinition>,
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
                model3d: None,
                spine: None,
                license: Some("Apache-2.0".into()),
                animations,
                interactions: BTreeMap::new(),
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

    pub fn interaction_animation(
        &self,
        key: InteractionAnimationKey,
    ) -> Option<&AnimationDefinition> {
        self.manifest.interactions.get(&key)
    }

    pub fn has_renderable_interaction(&self, key: InteractionAnimationKey) -> bool {
        self.interaction_animation(key)
            .is_some_and(|definition| self.animation_is_renderable(definition))
    }

    pub fn animation_with_idle_fallback(
        &self,
        behavior: Behavior,
    ) -> Option<(Behavior, &AnimationDefinition)> {
        let candidates: &[Behavior] = match behavior {
            Behavior::CodingWithMusic => &[
                Behavior::CodingWithMusic,
                Behavior::Coding,
                Behavior::ListeningMusic,
                Behavior::Idle,
            ],
            Behavior::Idle => &[Behavior::Idle],
            _ => &[behavior, Behavior::Idle],
        };

        candidates.iter().find_map(|candidate| {
            let definition = self.animation(*candidate)?;
            self.animation_is_renderable(definition)
                .then_some((*candidate, definition))
        })
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

    #[allow(dead_code)]
    pub fn is_spine(&self) -> bool {
        self.manifest.renderer == RendererKind::Spine
    }

    #[allow(dead_code)]
    pub fn spine_settings(&self) -> Option<&SpineSettings> {
        self.manifest.spine.as_ref()
    }

    #[allow(dead_code)]
    pub fn spine_skeleton_path(&self) -> Option<PathBuf> {
        let settings = self.spine_settings()?;
        Some(self.root.join(&settings.skeleton))
    }

    #[allow(dead_code)]
    pub fn spine_atlas_path(&self) -> Option<PathBuf> {
        let settings = self.spine_settings()?;
        Some(self.root.join(&settings.atlas))
    }

    #[allow(dead_code)]
    pub fn model3d_settings(&self) -> Option<&Model3dSettings> {
        self.manifest.model3d.as_ref()
    }

    #[allow(dead_code)]
    pub fn model3d_path(&self) -> Option<PathBuf> {
        let settings = self.model3d_settings()?;
        Some(self.root.join(&settings.source))
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

    pub fn interaction_frame_path(
        &self,
        key: InteractionAnimationKey,
        frame: usize,
    ) -> Option<PathBuf> {
        if !self.is_sprite() {
            return None;
        }

        let definition = self.interaction_animation(key)?;
        if !self.animation_is_renderable(definition) {
            return None;
        }
        let relative = definition
            .frames
            .get(frame)
            .or_else(|| definition.frames.first())?;
        Some(self.root.join(relative))
    }

    pub fn interaction_frame_duration_ms(
        &self,
        key: InteractionAnimationKey,
        frame: usize,
    ) -> Option<u64> {
        self.interaction_animation(key)?.frame_duration_ms(frame)
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

    if manifest.renderer == RendererKind::Spine {
        let spine = manifest.spine.as_ref().ok_or_else(|| {
            PackageError::InvalidManifest("spine renderer requires spine settings".into())
        })?;

        let skeleton = Path::new(&spine.skeleton);
        if !is_safe_relative_path(skeleton) {
            return Err(PackageError::UnsafeAssetPath(spine.skeleton.clone()));
        }
        if !is_supported_spine_skeleton(skeleton) {
            return Err(PackageError::UnsupportedAssetFormat(spine.skeleton.clone()));
        }

        let atlas = Path::new(&spine.atlas);
        if !is_safe_relative_path(atlas) {
            return Err(PackageError::UnsafeAssetPath(spine.atlas.clone()));
        }
        if !is_supported_spine_atlas(atlas) {
            return Err(PackageError::UnsupportedAssetFormat(spine.atlas.clone()));
        }

        if !spine.scale.is_finite() || !(0.05..=10.0).contains(&spine.scale) {
            return Err(PackageError::InvalidManifest(
                "spine.scale must be between 0.05 and 10.0".into(),
            ));
        }
        if spine.default_skin.trim().is_empty() {
            return Err(PackageError::InvalidManifest(
                "spine.default_skin must not be empty".into(),
            ));
        }

        for relative in [skeleton, atlas] {
            if !root.join(relative).is_file() {
                return Err(PackageError::MissingAsset(root.join(relative)));
            }
        }
    }

    if manifest.renderer == RendererKind::Model3d {
        let model = manifest.model3d.as_ref().ok_or_else(|| {
            PackageError::InvalidManifest("model3d renderer requires model3d settings".into())
        })?;
        let relative = Path::new(&model.source);
        if !is_safe_relative_path(relative) {
            return Err(PackageError::UnsafeAssetPath(model.source.clone()));
        }
        if !is_supported_model3d_asset(relative) {
            return Err(PackageError::UnsupportedAssetFormat(model.source.clone()));
        }
        if !model.scale.is_finite() || !(0.05..=10.0).contains(&model.scale) {
            return Err(PackageError::InvalidManifest(
                "model3d.scale must be between 0.05 and 10.0".into(),
            ));
        }
        if !root.join(relative).is_file() {
            return Err(PackageError::MissingAsset(root.join(relative)));
        }
    }

    for (animation, definition) in manifest
        .animations
        .iter()
        .map(|(key, definition)| (format!("{key:?}"), definition))
        .chain(
            manifest
                .interactions
                .iter()
                .map(|(key, definition)| (format!("interaction {key:?}"), definition)),
        )
    {
        let frame_count = definition.effective_frame_count();
        if frame_count == 0 {
            return Err(PackageError::InvalidManifest(format!(
                "{animation} must contain at least one frame"
            )));
        }

        if !definition.frame_durations_ms.is_empty() {
            if definition.frame_durations_ms.len() != frame_count {
                return Err(PackageError::InvalidManifest(format!(
                    "{animation} frame_durations_ms must have exactly {frame_count} entries"
                )));
            }

            if definition
                .frame_durations_ms
                .iter()
                .any(|milliseconds| *milliseconds == 0)
            {
                return Err(PackageError::InvalidManifest(format!(
                    "{animation} frame durations must be greater than zero"
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

fn is_supported_spine_skeleton(path: &Path) -> bool {
    path.extension()
        .and_then(|extension| extension.to_str())
        .is_some_and(|extension| {
            extension.eq_ignore_ascii_case("skel") || extension.eq_ignore_ascii_case("json")
        })
}

fn is_supported_spine_atlas(path: &Path) -> bool {
    path.extension()
        .and_then(|extension| extension.to_str())
        .is_some_and(|extension| extension.eq_ignore_ascii_case("atlas"))
}

fn is_supported_model3d_asset(path: &Path) -> bool {
    path.extension()
        .and_then(|extension| extension.to_str())
        .is_some_and(|extension| {
            extension.eq_ignore_ascii_case("vrm") || extension.eq_ignore_ascii_case("glb")
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
            Self::UnsupportedAssetFormat(path) => write!(
                f,
                "unsupported pet asset format: {path}; expected PNG/WebP for Sprite, SKEL/JSON + ATLAS for Spine, or VRM/GLB for Model3d"
            ),
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
    fn spine_assets_are_limited_to_skeleton_and_atlas_formats() {
        assert!(is_supported_spine_skeleton(Path::new("spine/sena.skel")));
        assert!(is_supported_spine_skeleton(Path::new("spine/sena.JSON")));
        assert!(!is_supported_spine_skeleton(Path::new("spine/sena.spine")));
        assert!(!is_supported_spine_skeleton(Path::new("spine/sena.png")));

        assert!(is_supported_spine_atlas(Path::new("spine/sena.atlas")));
        assert!(is_supported_spine_atlas(Path::new("spine/sena.ATLAS")));
        assert!(!is_supported_spine_atlas(Path::new("spine/sena.json")));
    }

    #[test]
    fn spine_package_loads_when_required_assets_exist() {
        let root = std::env::temp_dir().join(format!("sena-spine-package-{}", std::process::id()));
        let export = root.join("spine").join("export");
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(&export).expect("create Spine package fixture");

        fs::write(export.join("sena.skel"), []).expect("write skeleton fixture");
        fs::write(export.join("sena.atlas"), []).expect("write atlas fixture");
        fs::write(
            root.join("pet.json"),
            r#"{
                "schema_version": 1,
                "id": "test.spine",
                "name": "Spine Test",
                "version": "0.0.0",
                "author": "Sena Test",
                "renderer": "spine",
                "spine": {
                    "skeleton": "spine/export/sena.skel",
                    "atlas": "spine/export/sena.atlas",
                    "scale": 1.0,
                    "default_skin": "base"
                }
            }"#,
        )
        .expect("write manifest fixture");

        let package = PetPackage::load_from_dir(&root).expect("Spine package should load");
        assert!(package.is_spine());
        assert_eq!(
            package.spine_skeleton_path(),
            Some(export.join("sena.skel"))
        );
        assert_eq!(package.spine_atlas_path(), Some(export.join("sena.atlas")));
        assert_eq!(
            package
                .spine_settings()
                .map(|settings| settings.default_skin.as_str()),
            Some("base")
        );

        fs::remove_dir_all(root).expect("clean Spine package fixture");
    }

    #[test]
    fn model3d_assets_are_limited_to_vrm_and_glb() {
        assert!(is_supported_model3d_asset(Path::new("models/sena.vrm")));
        assert!(is_supported_model3d_asset(Path::new("models/sena.GLB")));
        assert!(!is_supported_model3d_asset(Path::new("models/sena.blend")));
        assert!(!is_supported_model3d_asset(Path::new("models/sena.fbx")));
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
        let spine = manifest.spine.as_ref().expect("Spine migration contract");
        assert_eq!(spine.skeleton, "spine/export/sena.skel");
        assert_eq!(spine.atlas, "spine/export/sena.atlas");
        assert_eq!(spine.default_skin, "base");
        let model = manifest
            .model3d
            .as_ref()
            .expect("archived 3D migration contract");
        assert_eq!(model.source, "models/sena.vrm");
        assert_eq!(model.motions.get("walk").map(String::as_str), Some("Walk"));
        assert_eq!(
            model.anchors.get("cat_carry").map(String::as_str),
            Some("CatCarry")
        );
        assert_eq!(manifest.animations.len(), 6);
        assert_eq!(manifest.interactions.len(), 4);
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
    fn official_interaction_slots_support_progressive_asset_rollout() {
        let package = PetPackage::load_default().expect("official Sena package should load");

        for key in [
            InteractionAnimationKey::Petting,
            InteractionAnimationKey::Stretch,
            InteractionAnimationKey::LookAtCat,
            InteractionAnimationKey::Daydream,
        ] {
            assert!(package.has_renderable_interaction(key));
            assert_eq!(
                package.interaction_frame_path(key, 0),
                Some(
                    package
                        .root()
                        .join("animations")
                        .join("idle")
                        .join("000.webp")
                )
            );
        }
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

    #[test]
    fn official_sena_coding_keeps_v1_frame_timing() {
        let package = PetPackage::load_default().expect("official Sena package should load");
        let coding = package
            .animation(Behavior::Coding)
            .expect("official Sena package should contain Coding");

        assert_eq!(
            coding.frames,
            vec![
                "animations/coding/000.webp",
                "animations/coding/001.webp",
                "animations/coding/002.webp",
                "animations/coding/003.webp",
            ]
        );
        assert_eq!(coding.frame_durations_ms, vec![220, 180, 900, 180]);
    }

    #[test]
    fn official_sena_listening_keeps_v1_frame_timing() {
        let package = PetPackage::load_default().expect("official Sena package should load");
        let listening = package
            .animation(Behavior::ListeningMusic)
            .expect("official Sena package should contain ListeningMusic");

        assert_eq!(
            listening.frames,
            vec![
                "animations/listening_music/000.webp",
                "animations/listening_music/001.webp",
                "animations/listening_music/002.webp",
                "animations/listening_music/003.webp",
            ]
        );
        assert_eq!(listening.frame_durations_ms, vec![240, 220, 240, 220]);
    }

    #[test]
    fn official_sena_drowsy_keeps_v1_frame_timing() {
        let package = PetPackage::load_default().expect("official Sena package should load");
        let drowsy = package
            .animation(Behavior::Drowsy)
            .expect("official Sena package should contain Drowsy");

        assert_eq!(
            drowsy.frames,
            vec![
                "animations/drowsy/000.webp",
                "animations/drowsy/001.webp",
                "animations/drowsy/002.webp",
            ]
        );
        assert_eq!(drowsy.frame_durations_ms, vec![420, 760, 420]);
    }

    #[test]
    fn official_sena_sleeping_keeps_v1_frame_timing() {
        let package = PetPackage::load_default().expect("official Sena package should load");
        let sleeping = package
            .animation(Behavior::Sleeping)
            .expect("official Sena package should contain Sleeping");

        assert_eq!(
            sleeping.frames,
            vec![
                "animations/sleeping/000.webp",
                "animations/sleeping/001.webp",
                "animations/sleeping/002.webp",
            ]
        );
        assert_eq!(sleeping.frame_durations_ms, vec![700, 1000, 700]);
    }

    #[test]
    fn official_sena_coding_with_music_keeps_v1_frame_timing() {
        let package = PetPackage::load_default().expect("official Sena package should load");
        let combined = package
            .animation(Behavior::CodingWithMusic)
            .expect("official Sena package should contain CodingWithMusic");

        assert_eq!(
            combined.frames,
            vec![
                "animations/coding_with_music/000.webp",
                "animations/coding_with_music/001.webp",
                "animations/coding_with_music/002.webp",
                "animations/coding_with_music/003.webp",
            ]
        );
        assert_eq!(combined.frame_durations_ms, vec![180, 160, 340, 160]);
    }
}
