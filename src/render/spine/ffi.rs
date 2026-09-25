use std::ffi::{c_char, c_float, c_int, c_ushort, c_void};

pub type SenaSpineRuntimeOpaque = c_void;

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct SenaSpineVertexRaw {
    pub x: c_float,
    pub y: c_float,
    pub u: c_float,
    pub v: c_float,
    pub r: c_float,
    pub g: c_float,
    pub b: c_float,
    pub a: c_float,
    pub dark_r: c_float,
    pub dark_g: c_float,
    pub dark_b: c_float,
}

pub type SenaSpineBatchCallback = unsafe extern "C" fn(
    user_data: *mut c_void,
    texture_page: *const c_char,
    slot_name: *const c_char,
    attachment_name: *const c_char,
    blend_mode: c_int,
    vertices: *const SenaSpineVertexRaw,
    vertex_count: c_int,
    indices: *const c_ushort,
    index_count: c_int,
) -> c_int;

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

    pub fn sena_spine_runtime_set_skin(
        runtime: *mut SenaSpineRuntimeOpaque,
        skin_name: *const c_char,
    ) -> c_int;

    pub fn sena_spine_runtime_version(runtime: *const SenaSpineRuntimeOpaque) -> *const c_char;

    pub fn sena_spine_runtime_skin_count(runtime: *const SenaSpineRuntimeOpaque) -> c_int;
    pub fn sena_spine_runtime_skin_name(
        runtime: *const SenaSpineRuntimeOpaque,
        index: c_int,
    ) -> *const c_char;

    pub fn sena_spine_runtime_animation_count(runtime: *const SenaSpineRuntimeOpaque) -> c_int;
    pub fn sena_spine_runtime_animation_name(
        runtime: *const SenaSpineRuntimeOpaque,
        index: c_int,
    ) -> *const c_char;
    pub fn sena_spine_runtime_animation_duration(
        runtime: *const SenaSpineRuntimeOpaque,
        index: c_int,
    ) -> c_float;

    pub fn sena_spine_runtime_atlas_page_count(runtime: *const SenaSpineRuntimeOpaque) -> c_int;
    pub fn sena_spine_runtime_atlas_page_name(
        runtime: *const SenaSpineRuntimeOpaque,
        index: c_int,
    ) -> *const c_char;

    pub fn sena_spine_runtime_bone_count(runtime: *const SenaSpineRuntimeOpaque) -> c_int;
    pub fn sena_spine_runtime_bone_name(
        runtime: *const SenaSpineRuntimeOpaque,
        index: c_int,
    ) -> *const c_char;
    pub fn sena_spine_runtime_bone_parent_name(
        runtime: *const SenaSpineRuntimeOpaque,
        index: c_int,
    ) -> *const c_char;

    pub fn sena_spine_runtime_slot_count(runtime: *const SenaSpineRuntimeOpaque) -> c_int;
    pub fn sena_spine_runtime_slot_name(
        runtime: *const SenaSpineRuntimeOpaque,
        index: c_int,
    ) -> *const c_char;
    pub fn sena_spine_runtime_slot_bone_name(
        runtime: *const SenaSpineRuntimeOpaque,
        index: c_int,
    ) -> *const c_char;
    pub fn sena_spine_runtime_slot_setup_attachment_name(
        runtime: *const SenaSpineRuntimeOpaque,
        index: c_int,
    ) -> *const c_char;
    pub fn sena_spine_runtime_slot_blend_mode(
        runtime: *const SenaSpineRuntimeOpaque,
        index: c_int,
    ) -> c_int;
    pub fn sena_spine_runtime_attachment_type(
        runtime: *mut SenaSpineRuntimeOpaque,
        slot_name: *const c_char,
        attachment_name: *const c_char,
    ) -> c_int;
    pub fn sena_spine_runtime_skin_attachment_type(
        runtime: *const SenaSpineRuntimeOpaque,
        skin_name: *const c_char,
        slot_name: *const c_char,
        attachment_name: *const c_char,
    ) -> c_int;

    pub fn sena_spine_runtime_animation_timeline_count(
        runtime: *const SenaSpineRuntimeOpaque,
        animation_name: *const c_char,
    ) -> c_int;
    pub fn sena_spine_runtime_animation_timeline_type(
        runtime: *const SenaSpineRuntimeOpaque,
        animation_name: *const c_char,
        timeline_index: c_int,
    ) -> c_int;
    pub fn sena_spine_runtime_animation_timeline_target_kind(
        runtime: *const SenaSpineRuntimeOpaque,
        animation_name: *const c_char,
        timeline_index: c_int,
    ) -> c_int;
    pub fn sena_spine_runtime_animation_timeline_target_name(
        runtime: *const SenaSpineRuntimeOpaque,
        animation_name: *const c_char,
        timeline_index: c_int,
    ) -> *const c_char;

    pub fn sena_spine_runtime_update(runtime: *mut SenaSpineRuntimeOpaque, delta_seconds: c_float);

    pub fn sena_spine_runtime_extract_frame(
        runtime: *mut SenaSpineRuntimeOpaque,
        callback: SenaSpineBatchCallback,
        user_data: *mut c_void,
    ) -> c_int;

    pub fn sena_spine_runtime_bone_world_transform(
        runtime: *mut SenaSpineRuntimeOpaque,
        bone_name: *const c_char,
        world_x: *mut c_float,
        world_y: *mut c_float,
        world_rotation_degrees: *mut c_float,
    ) -> c_int;
}
