#include "sena_spine_bridge.h"

#include <spine/AnimationState.h>
#include <spine/AnimationStateData.h>
#include <spine/Atlas.h>
#include <spine/Bone.h>
#include <spine/Skeleton.h>
#include <spine/SkeletonBinary.h>
#include <spine/SkeletonData.h>
#include <spine/SkeletonJson.h>
#include <spine/extension.h>

#include <stdlib.h>
#include <string.h>

struct SenaSpineRuntime {
    spAtlas* atlas;
    spSkeletonData* skeleton_data;
    spSkeleton* skeleton;
    spAnimationStateData* animation_state_data;
    spAnimationState* animation_state;
};

static void copy_error(char* buffer, size_t size, const char* message) {
    if (!buffer || size == 0) return;
    if (!message) message = "unknown Spine runtime error";

#if defined(_MSC_VER)
    strncpy_s(buffer, size, message, _TRUNCATE);
#else
    strncpy(buffer, message, size - 1);
    buffer[size - 1] = '\0';
#endif
}

/*
 * R1 does not create a texture atlas yet. spine-c still requires these host
 * callbacks to exist at link time because Atlas.c references them. R2 replaces
 * these no-op hooks with D3D11 texture ownership.
 */
void _spAtlasPage_createTexture(spAtlasPage* self, const char* path) {
    (void)path;
    if (!self) return;
    self->rendererObject = 0;
}

void _spAtlasPage_disposeTexture(spAtlasPage* self) {
    if (!self) return;
    self->rendererObject = 0;
}

char* _spUtil_readFile(const char* path, int* length) {
    return _spReadFile(path, length);
}

static SenaSpineRuntime* create_runtime(
    spAtlas* atlas,
    spSkeletonData* skeleton_data,
    char* error_buffer,
    size_t error_buffer_size
) {
    spSkeleton* skeleton;
    spAnimationStateData* animation_state_data;
    spAnimationState* animation_state;
    SenaSpineRuntime* runtime;

    if (!skeleton_data) {
        if (atlas) spAtlas_dispose(atlas);
        copy_error(error_buffer, error_buffer_size, "Spine skeleton data is null");
        return 0;
    }

    skeleton = spSkeleton_create(skeleton_data);
    if (!skeleton) {
        spSkeletonData_dispose(skeleton_data);
        if (atlas) spAtlas_dispose(atlas);
        copy_error(error_buffer, error_buffer_size, "failed to create Spine skeleton");
        return 0;
    }

    animation_state_data = spAnimationStateData_create(skeleton_data);
    if (!animation_state_data) {
        spSkeleton_dispose(skeleton);
        spSkeletonData_dispose(skeleton_data);
        if (atlas) spAtlas_dispose(atlas);
        copy_error(error_buffer, error_buffer_size, "failed to create Spine animation state data");
        return 0;
    }

    animation_state_data->defaultMix = 0.12f;
    animation_state = spAnimationState_create(animation_state_data);
    if (!animation_state) {
        spAnimationStateData_dispose(animation_state_data);
        spSkeleton_dispose(skeleton);
        spSkeletonData_dispose(skeleton_data);
        if (atlas) spAtlas_dispose(atlas);
        copy_error(error_buffer, error_buffer_size, "failed to create Spine animation state");
        return 0;
    }

    runtime = (SenaSpineRuntime*)calloc(1, sizeof(SenaSpineRuntime));
    if (!runtime) {
        spAnimationState_dispose(animation_state);
        spAnimationStateData_dispose(animation_state_data);
        spSkeleton_dispose(skeleton);
        spSkeletonData_dispose(skeleton_data);
        if (atlas) spAtlas_dispose(atlas);
        copy_error(error_buffer, error_buffer_size, "failed to allocate Sena Spine runtime");
        return 0;
    }

    runtime->atlas = atlas;
    runtime->skeleton_data = skeleton_data;
    runtime->skeleton = skeleton;
    runtime->animation_state_data = animation_state_data;
    runtime->animation_state = animation_state;

    spSkeleton_updateWorldTransform(skeleton);
    return runtime;
}

static int ends_with_ignore_case(const char* value, const char* suffix) {
    size_t value_length;
    size_t suffix_length;
    size_t index;

    if (!value || !suffix) return 0;
    value_length = strlen(value);
    suffix_length = strlen(suffix);
    if (suffix_length > value_length) return 0;

    value += value_length - suffix_length;
    for (index = 0; index < suffix_length; ++index) {
        char left = value[index];
        char right = suffix[index];
        if (left >= 'A' && left <= 'Z') left = (char)(left - 'A' + 'a');
        if (right >= 'A' && right <= 'Z') right = (char)(right - 'A' + 'a');
        if (left != right) return 0;
    }
    return 1;
}

SenaSpineRuntime* sena_spine_runtime_create_json(
    const char* json_text,
    char* error_buffer,
    size_t error_buffer_size
) {
    spSkeletonJson* parser;
    spSkeletonData* skeleton_data;
    if (!json_text) {
        copy_error(error_buffer, error_buffer_size, "Spine JSON text is null");
        return 0;
    }

    /*
     * A null atlas is valid for the R1 skeleton-only smoke fixture because it
     * contains no region or mesh attachments.
     */
    parser = spSkeletonJson_create(0);
    if (!parser) {
        copy_error(error_buffer, error_buffer_size, "failed to create Spine JSON parser");
        return 0;
    }

    skeleton_data = spSkeletonJson_readSkeletonData(parser, json_text);
    if (!skeleton_data) {
        copy_error(error_buffer, error_buffer_size, parser->error);
        spSkeletonJson_dispose(parser);
        return 0;
    }
    spSkeletonJson_dispose(parser);

    return create_runtime(0, skeleton_data, error_buffer, error_buffer_size);
}

SenaSpineRuntime* sena_spine_runtime_create_files(
    const char* skeleton_path,
    const char* atlas_path,
    float scale,
    char* error_buffer,
    size_t error_buffer_size
) {
    spAtlas* atlas;
    spSkeletonData* skeleton_data = 0;

    if (!skeleton_path || !atlas_path) {
        copy_error(error_buffer, error_buffer_size, "Spine skeleton/atlas path is null");
        return 0;
    }
    if (scale <= 0.0f) {
        copy_error(error_buffer, error_buffer_size, "Spine scale must be greater than zero");
        return 0;
    }

    atlas = spAtlas_createFromFile(atlas_path, 0);
    if (!atlas) {
        copy_error(error_buffer, error_buffer_size, "failed to load Spine atlas");
        return 0;
    }

    if (ends_with_ignore_case(skeleton_path, ".json")) {
        spSkeletonJson* parser = spSkeletonJson_create(atlas);
        if (!parser) {
            spAtlas_dispose(atlas);
            copy_error(error_buffer, error_buffer_size, "failed to create Spine JSON parser");
            return 0;
        }
        parser->scale = scale;
        skeleton_data = spSkeletonJson_readSkeletonDataFile(parser, skeleton_path);
        if (!skeleton_data) {
            copy_error(error_buffer, error_buffer_size, parser->error);
            spSkeletonJson_dispose(parser);
            spAtlas_dispose(atlas);
            return 0;
        }
        spSkeletonJson_dispose(parser);
    } else if (ends_with_ignore_case(skeleton_path, ".skel")) {
        spSkeletonBinary* parser = spSkeletonBinary_create(atlas);
        if (!parser) {
            spAtlas_dispose(atlas);
            copy_error(error_buffer, error_buffer_size, "failed to create Spine binary parser");
            return 0;
        }
        parser->scale = scale;
        skeleton_data = spSkeletonBinary_readSkeletonDataFile(parser, skeleton_path);
        if (!skeleton_data) {
            copy_error(error_buffer, error_buffer_size, parser->error);
            spSkeletonBinary_dispose(parser);
            spAtlas_dispose(atlas);
            return 0;
        }
        spSkeletonBinary_dispose(parser);
    } else {
        spAtlas_dispose(atlas);
        copy_error(error_buffer, error_buffer_size, "Spine skeleton must use .json or .skel");
        return 0;
    }

    return create_runtime(atlas, skeleton_data, error_buffer, error_buffer_size);
}

void sena_spine_runtime_dispose(SenaSpineRuntime* runtime) {
    if (!runtime) return;

    spAnimationState_dispose(runtime->animation_state);
    spAnimationStateData_dispose(runtime->animation_state_data);
    spSkeleton_dispose(runtime->skeleton);
    spSkeletonData_dispose(runtime->skeleton_data);
    if (runtime->atlas) spAtlas_dispose(runtime->atlas);
    free(runtime);
}

int sena_spine_runtime_set_animation(
    SenaSpineRuntime* runtime,
    int track_index,
    const char* animation_name,
    int loop
) {
    spAnimation* animation;

    if (!runtime || !animation_name || track_index < 0) return 0;

    /*
     * spine-c 3.8's spAnimationState_setAnimationByName forwards a null
     * animation into spAnimationState_setAnimation when the name is missing,
     * which then dereferences it. Guard the legacy API at our bridge boundary.
     */
    animation = spSkeletonData_findAnimation(runtime->skeleton_data, animation_name);
    if (!animation) return 0;

    return spAnimationState_setAnimation(
        runtime->animation_state,
        track_index,
        animation,
        loop ? 1 : 0
    ) != 0;
}

void sena_spine_runtime_update(SenaSpineRuntime* runtime, float delta_seconds) {
    if (!runtime || delta_seconds < 0.0f) return;

    spSkeleton_update(runtime->skeleton, delta_seconds);
    spAnimationState_update(runtime->animation_state, delta_seconds);
    spAnimationState_apply(runtime->animation_state, runtime->skeleton);
    spSkeleton_updateWorldTransform(runtime->skeleton);
}

int sena_spine_runtime_bone_world_transform(
    SenaSpineRuntime* runtime,
    const char* bone_name,
    float* world_x,
    float* world_y,
    float* world_rotation_degrees
) {
    spBone* bone;

    if (!runtime || !bone_name) return 0;

    bone = spSkeleton_findBone(runtime->skeleton, bone_name);
    if (!bone) return 0;

    if (world_x) *world_x = bone->worldX;
    if (world_y) *world_y = bone->worldY;
    if (world_rotation_degrees) {
        *world_rotation_degrees = spBone_getWorldRotationX(bone);
    }
    return 1;
}
