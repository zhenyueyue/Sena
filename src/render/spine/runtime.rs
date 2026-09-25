use std::{
    ffi::{CStr, CString},
    path::Path,
    ptr::NonNull,
    sync::{Mutex, MutexGuard},
};

use super::ffi;

const ERROR_BUFFER_SIZE: usize = 1024;
static SPINE_RUNTIME_LOCK: Mutex<()> = Mutex::new(());

fn runtime_lock() -> MutexGuard<'static, ()> {
    SPINE_RUNTIME_LOCK
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BoneWorldTransform {
    pub x: f32,
    pub y: f32,
    pub rotation_degrees: f32,
}

#[derive(Debug)]
pub struct SpineRuntime {
    raw: NonNull<ffi::SenaSpineRuntimeOpaque>,
}

impl SpineRuntime {
    pub fn from_json(json: &str) -> Result<Self, String> {
        let json = CString::new(json).map_err(|_| "Spine JSON contains a NUL byte".to_string())?;
        let mut error = [0_i8; ERROR_BUFFER_SIZE];

        let _guard = runtime_lock();
        let raw = unsafe {
            ffi::sena_spine_runtime_create_json(json.as_ptr(), error.as_mut_ptr(), error.len())
        };

        NonNull::new(raw)
            .map(|raw| Self { raw })
            .ok_or_else(|| error_message(&error))
    }

    pub fn from_files(skeleton_path: &Path, atlas_path: &Path, scale: f32) -> Result<Self, String> {
        if !scale.is_finite() || scale <= 0.0 {
            return Err("Spine scale must be finite and greater than zero".into());
        }

        let skeleton = path_to_c_string(skeleton_path, "skeleton")?;
        let atlas = path_to_c_string(atlas_path, "atlas")?;
        let mut error = [0_i8; ERROR_BUFFER_SIZE];

        let _guard = runtime_lock();
        let raw = unsafe {
            ffi::sena_spine_runtime_create_files(
                skeleton.as_ptr(),
                atlas.as_ptr(),
                scale,
                error.as_mut_ptr(),
                error.len(),
            )
        };

        NonNull::new(raw)
            .map(|raw| Self { raw })
            .ok_or_else(|| error_message(&error))
    }

    pub fn set_animation(
        &mut self,
        track_index: i32,
        animation_name: &str,
        looping: bool,
    ) -> Result<(), String> {
        let name = CString::new(animation_name)
            .map_err(|_| "Spine animation name contains a NUL byte".to_string())?;
        let _guard = runtime_lock();
        let found = unsafe {
            ffi::sena_spine_runtime_set_animation(
                self.raw.as_ptr(),
                track_index,
                name.as_ptr(),
                i32::from(looping),
            )
        };

        if found != 0 {
            Ok(())
        } else {
            Err(format!("Spine animation not found: {animation_name}"))
        }
    }

    pub fn update(&mut self, delta_seconds: f32) {
        if delta_seconds.is_finite() && delta_seconds >= 0.0 {
            let _guard = runtime_lock();
            unsafe {
                ffi::sena_spine_runtime_update(self.raw.as_ptr(), delta_seconds);
            }
        }
    }

    pub fn bone_world_transform(&self, bone_name: &str) -> Option<BoneWorldTransform> {
        let name = CString::new(bone_name).ok()?;
        let mut x = 0.0;
        let mut y = 0.0;
        let mut rotation_degrees = 0.0;

        let _guard = runtime_lock();
        let found = unsafe {
            ffi::sena_spine_runtime_bone_world_transform(
                self.raw.as_ptr(),
                name.as_ptr(),
                &mut x,
                &mut y,
                &mut rotation_degrees,
            )
        };

        (found != 0).then_some(BoneWorldTransform {
            x,
            y,
            rotation_degrees,
        })
    }
}

impl Drop for SpineRuntime {
    fn drop(&mut self) {
        let _guard = runtime_lock();
        unsafe {
            ffi::sena_spine_runtime_dispose(self.raw.as_ptr());
        }
    }
}

fn path_to_c_string(path: &Path, kind: &str) -> Result<CString, String> {
    let value = path.to_string_lossy();
    CString::new(value.as_bytes())
        .map_err(|_| format!("Spine {kind} path contains a NUL byte: {}", path.display()))
}

fn error_message(buffer: &[i8]) -> String {
    let bytes = buffer.iter().map(|value| *value as u8).collect::<Vec<_>>();

    CStr::from_bytes_until_nul(&bytes)
        .ok()
        .and_then(|message| message.to_str().ok())
        .filter(|message| !message.is_empty())
        .unwrap_or("failed to initialize Spine runtime")
        .to_string()
}

#[cfg(test)]
mod tests {
    use std::fs;

    use super::*;

    const SMOKE_SKELETON: &str = r#"{
      "skeleton": { "hash": "", "spine": "3.8.75", "width": 0, "height": 0 },
      "bones": [
        { "name": "root" },
        { "name": "head", "parent": "root", "y": 10 }
      ],
      "animations": {
        "idle": {
          "bones": {
            "root": {
              "translate": [
                { "time": 0, "x": 0, "y": 0 },
                { "time": 0.5, "x": 4, "y": 2 },
                { "time": 1.0, "x": 0, "y": 0 }
              ],
              "rotate": [
                { "time": 0, "angle": 0 },
                { "time": 0.5, "angle": 12 },
                { "time": 1.0, "angle": 0 }
              ]
            }
          }
        }
      }
    }"#;

    #[test]
    fn spine_c_38_plays_idle_and_updates_world_transform() {
        let mut runtime =
            SpineRuntime::from_json(SMOKE_SKELETON).expect("Spine 3.8 smoke skeleton should parse");

        runtime
            .set_animation(0, "idle", true)
            .expect("idle animation should exist");

        let before = runtime
            .bone_world_transform("root")
            .expect("root bone should exist");
        runtime.update(0.5);
        let animated = runtime
            .bone_world_transform("root")
            .expect("root bone should still exist");

        assert!(animated.x > before.x + 3.0);
        assert!(animated.y > before.y + 1.0);
        assert!(animated.rotation_degrees > 8.0);
    }

    #[test]
    fn spine_c_38_loads_json_and_atlas_from_files() {
        let root = std::env::temp_dir().join(format!("sena-spine-r1-files-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(&root).expect("create Spine R1 fixture directory");

        let skeleton_path = root.join("sena.json");
        let atlas_path = root.join("sena.atlas");
        fs::write(&skeleton_path, SMOKE_SKELETON).expect("write Spine JSON fixture");
        fs::write(
            &atlas_path,
            "sena.png\nsize: 1,1\nformat: RGBA8888\nfilter: Linear,Linear\nrepeat: none\n",
        )
        .expect("write Spine atlas fixture");

        let mut runtime = SpineRuntime::from_files(&skeleton_path, &atlas_path, 1.0)
            .expect("Spine JSON + atlas should load from files");
        runtime
            .set_animation(0, "idle", true)
            .expect("idle animation should exist");
        runtime.update(0.5);

        let root_bone = runtime
            .bone_world_transform("root")
            .expect("root bone should exist");
        assert!(root_bone.x > 3.0);
        assert!(root_bone.y > 1.0);

        fs::remove_dir_all(root).expect("clean Spine R1 fixture directory");
    }

    #[test]
    fn missing_animation_and_bone_are_reported_without_crashing() {
        let mut runtime =
            SpineRuntime::from_json(SMOKE_SKELETON).expect("Spine 3.8 smoke skeleton should parse");

        assert!(runtime.set_animation(0, "missing", true).is_err());
        assert!(runtime.bone_world_transform("missing").is_none());
    }
}
