use std::ffi::{c_char, c_float, c_int, c_void};

pub type SenaSpineRuntimeOpaque = c_void;

unsafe extern "C" {
    pub fn sena_spine_runtime_create_json(
        json_text: *const c_char,
        error_buffer: *mut c_char,
        error_buffer_size: usize,
    ) -> *mut SenaSpineRuntimeOpaque;

    pub fn sena_spine_runtime_create_files(
        skeleton_path: *const c_char,
        atlas_path: *const c_char,
        scale: c_float,
        error_buffer: *mut c_char,
        error_buffer_size: usize,
    ) -> *mut SenaSpineRuntimeOpaque;

    pub fn sena_spine_runtime_dispose(runtime: *mut SenaSpineRuntimeOpaque);

    pub fn sena_spine_runtime_set_animation(
        runtime: *mut SenaSpineRuntimeOpaque,
        track_index: c_int,
        animation_name: *const c_char,
        loop_animation: c_int,
    ) -> c_int;

    pub fn sena_spine_runtime_update(runtime: *mut SenaSpineRuntimeOpaque, delta_seconds: c_float);

    pub fn sena_spine_runtime_bone_world_transform(
        runtime: *mut SenaSpineRuntimeOpaque,
        bone_name: *const c_char,
        world_x: *mut c_float,
        world_y: *mut c_float,
        world_rotation_degrees: *mut c_float,
    ) -> c_int;
}
