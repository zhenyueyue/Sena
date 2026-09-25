#ifndef SENA_SPINE_BRIDGE_H
#define SENA_SPINE_BRIDGE_H

#include <stddef.h>

#ifdef __cplusplus
extern "C" {
#endif

typedef struct SenaSpineRuntime SenaSpineRuntime;

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

void sena_spine_runtime_update(SenaSpineRuntime* runtime, float delta_seconds);

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
