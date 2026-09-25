#ifndef SENA_SPINE_BRIDGE_H
#define SENA_SPINE_BRIDGE_H

#include <stddef.h>

#ifdef __cplusplus
extern "C" {
#endif

typedef struct SenaSpineRuntime SenaSpineRuntime;

typedef enum SenaSpineBlendMode {
    SENA_SPINE_BLEND_NORMAL = 0,
    SENA_SPINE_BLEND_ADDITIVE = 1,
    SENA_SPINE_BLEND_MULTIPLY = 2,
    SENA_SPINE_BLEND_SCREEN = 3
} SenaSpineBlendMode;

typedef struct SenaSpineVertex {
    float x;
    float y;
    float u;
    float v;
    float r;
    float g;
    float b;
    float a;
    float dark_r;
    float dark_g;
    float dark_b;
} SenaSpineVertex;

typedef int (*SenaSpineBatchCallback)(
    void* user_data,
    const char* texture_page,
    const char* slot_name,
    const char* attachment_name,
    int blend_mode,
    const SenaSpineVertex* vertices,
    int vertex_count,
    const unsigned short* indices,
    int index_count
);

SenaSpineRuntime* sena_spine_runtime_create_json(
    const char* json_text,
    char* error_buffer,
    size_t error_buffer_size
);

SenaSpineRuntime* sena_spine_runtime_create_files(
    const char* skeleton_path,
    const char* atlas_path,
    float scale,
    char* error_buffer,
    size_t error_buffer_size
);

void sena_spine_runtime_dispose(SenaSpineRuntime* runtime);

int sena_spine_runtime_set_animation(
    SenaSpineRuntime* runtime,
    int track_index,
    const char* animation_name,
    int loop
);

int sena_spine_runtime_set_skin(
    SenaSpineRuntime* runtime,
    const char* skin_name
);

void sena_spine_runtime_update(SenaSpineRuntime* runtime, float delta_seconds);

int sena_spine_runtime_extract_frame(
    SenaSpineRuntime* runtime,
    SenaSpineBatchCallback callback,
    void* user_data
);

int sena_spine_runtime_bone_world_transform(
    SenaSpineRuntime* runtime,
    const char* bone_name,
    float* world_x,
    float* world_y,
    float* world_rotation_degrees
);

#ifdef __cplusplus
}
#endif

#endif
