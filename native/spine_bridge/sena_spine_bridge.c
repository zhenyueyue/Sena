#include "sena_spine_bridge.h"

#include <spine/Animation.h>
#include <spine/AnimationState.h>
#include <spine/BoneData.h>
#include <spine/AnimationStateData.h>
#include <spine/Atlas.h>
#include <spine/Bone.h>
#include <spine/ClippingAttachment.h>
#include <spine/MeshAttachment.h>
#include <spine/RegionAttachment.h>
#include <spine/Skeleton.h>
#include <spine/SkeletonBinary.h>
#include <spine/SkeletonClipping.h>
#include <spine/SkeletonData.h>
#include <spine/SkeletonJson.h>
#include <spine/Skin.h>
#include <spine/SlotData.h>
#include <spine/extension.h>

#include <stdlib.h>
#include <string.h>

struct SenaSpineRuntime {
    spAtlas* atlas;
    spSkeletonData* skeleton_data;
    spSkeleton* skeleton;
    spAnimationStateData* animation_state_data;
    spAnimationState* animation_state;
    spSkeletonClipping* clipper;
    float* world_vertices;
    int world_vertices_capacity;
    SenaSpineVertex* packed_vertices;
    int packed_vertices_capacity;
};

static const unsigned short QUAD_TRIANGLES[6] = {0, 1, 2, 2, 3, 0};

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
    runtime->clipper = spSkeletonClipping_create();

    if (!runtime->clipper) {
        spAnimationState_dispose(animation_state);
        spAnimationStateData_dispose(animation_state_data);
        spSkeleton_dispose(skeleton);
        spSkeletonData_dispose(skeleton_data);
        if (atlas) spAtlas_dispose(atlas);
        free(runtime);
        copy_error(error_buffer, error_buffer_size, "failed to create Spine clipper");
        return 0;
    }

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

    spSkeletonClipping_dispose(runtime->clipper);
    free(runtime->world_vertices);
    free(runtime->packed_vertices);
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

int sena_spine_runtime_set_skin(
    SenaSpineRuntime* runtime,
    const char* skin_name
) {
    if (!runtime || !skin_name || !skin_name[0]) return 0;
    if (!spSkeleton_setSkinByName(runtime->skeleton, skin_name)) return 0;

    spSkeleton_setSlotsToSetupPose(runtime->skeleton);
    spSkeleton_updateWorldTransform(runtime->skeleton);
    return 1;
}

const char* sena_spine_runtime_version(const SenaSpineRuntime* runtime) {
    if (!runtime || !runtime->skeleton_data) return 0;
    return runtime->skeleton_data->version;
}

int sena_spine_runtime_skin_count(const SenaSpineRuntime* runtime) {
    if (!runtime || !runtime->skeleton_data) return 0;
    return runtime->skeleton_data->skinsCount;
}

const char* sena_spine_runtime_skin_name(const SenaSpineRuntime* runtime, int index) {
    spSkin* skin;
    if (!runtime || !runtime->skeleton_data) return 0;
    if (index < 0 || index >= runtime->skeleton_data->skinsCount) return 0;

    skin = runtime->skeleton_data->skins[index];
    return skin ? skin->name : 0;
}

int sena_spine_runtime_animation_count(const SenaSpineRuntime* runtime) {
    if (!runtime || !runtime->skeleton_data) return 0;
    return runtime->skeleton_data->animationsCount;
}

const char* sena_spine_runtime_animation_name(const SenaSpineRuntime* runtime, int index) {
    spAnimation* animation;
    if (!runtime || !runtime->skeleton_data) return 0;
    if (index < 0 || index >= runtime->skeleton_data->animationsCount) return 0;

    animation = runtime->skeleton_data->animations[index];
    return animation ? animation->name : 0;
}

float sena_spine_runtime_animation_duration(const SenaSpineRuntime* runtime, int index) {
    spAnimation* animation;
    if (!runtime || !runtime->skeleton_data) return -1.0f;
    if (index < 0 || index >= runtime->skeleton_data->animationsCount) return -1.0f;

    animation = runtime->skeleton_data->animations[index];
    return animation ? animation->duration : -1.0f;
}

int sena_spine_runtime_atlas_page_count(const SenaSpineRuntime* runtime) {
    int count = 0;
    spAtlasPage* page;
    if (!runtime || !runtime->atlas) return 0;

    page = runtime->atlas->pages;
    while (page) {
        count += 1;
        page = page->next;
    }
    return count;
}

const char* sena_spine_runtime_atlas_page_name(const SenaSpineRuntime* runtime, int index) {
    int current = 0;
    spAtlasPage* page;
    if (!runtime || !runtime->atlas || index < 0) return 0;

    page = runtime->atlas->pages;
    while (page) {
        if (current == index) return page->name;
        current += 1;
        page = page->next;
    }
    return 0;
}

int sena_spine_runtime_bone_count(const SenaSpineRuntime* runtime) {
    if (!runtime || !runtime->skeleton_data) return 0;
    return runtime->skeleton_data->bonesCount;
}

const char* sena_spine_runtime_bone_name(const SenaSpineRuntime* runtime, int index) {
    spBoneData* bone;
    if (!runtime || !runtime->skeleton_data) return 0;
    if (index < 0 || index >= runtime->skeleton_data->bonesCount) return 0;

    bone = runtime->skeleton_data->bones[index];
    return bone ? bone->name : 0;
}

const char* sena_spine_runtime_bone_parent_name(const SenaSpineRuntime* runtime, int index) {
    spBoneData* bone;
    if (!runtime || !runtime->skeleton_data) return 0;
    if (index < 0 || index >= runtime->skeleton_data->bonesCount) return 0;

    bone = runtime->skeleton_data->bones[index];
    return bone && bone->parent ? bone->parent->name : 0;
}

int sena_spine_runtime_slot_count(const SenaSpineRuntime* runtime) {
    if (!runtime || !runtime->skeleton_data) return 0;
    return runtime->skeleton_data->slotsCount;
}

const char* sena_spine_runtime_slot_name(const SenaSpineRuntime* runtime, int index) {
    spSlotData* slot;
    if (!runtime || !runtime->skeleton_data) return 0;
    if (index < 0 || index >= runtime->skeleton_data->slotsCount) return 0;

    slot = runtime->skeleton_data->slots[index];
    return slot ? slot->name : 0;
}

const char* sena_spine_runtime_slot_bone_name(const SenaSpineRuntime* runtime, int index) {
    spSlotData* slot;
    if (!runtime || !runtime->skeleton_data) return 0;
    if (index < 0 || index >= runtime->skeleton_data->slotsCount) return 0;

    slot = runtime->skeleton_data->slots[index];
    return slot && slot->boneData ? slot->boneData->name : 0;
}

const char* sena_spine_runtime_slot_setup_attachment_name(const SenaSpineRuntime* runtime, int index) {
    spSlotData* slot;
    if (!runtime || !runtime->skeleton_data) return 0;
    if (index < 0 || index >= runtime->skeleton_data->slotsCount) return 0;

    slot = runtime->skeleton_data->slots[index];
    return slot ? slot->attachmentName : 0;
}

int sena_spine_runtime_slot_blend_mode(const SenaSpineRuntime* runtime, int index) {
    spSlotData* slot;
    if (!runtime || !runtime->skeleton_data) return -1;
    if (index < 0 || index >= runtime->skeleton_data->slotsCount) return -1;

    slot = runtime->skeleton_data->slots[index];
    return slot ? (int)slot->blendMode : -1;
}

int sena_spine_runtime_attachment_type(
    SenaSpineRuntime* runtime,
    const char* slot_name,
    const char* attachment_name
) {
    spAttachment* attachment;
    if (!runtime || !runtime->skeleton || !slot_name || !attachment_name) return -1;

    attachment = spSkeleton_getAttachmentForSlotName(
        runtime->skeleton,
        slot_name,
        attachment_name
    );
    return attachment ? (int)attachment->type : -1;
}

int sena_spine_runtime_skin_attachment_type(
    const SenaSpineRuntime* runtime,
    const char* skin_name,
    const char* slot_name,
    const char* attachment_name
) {
    int slot_index;
    spSkin* skin;
    spAttachment* attachment;

    if (!runtime || !runtime->skeleton_data || !skin_name || !slot_name || !attachment_name) return -1;

    skin = spSkeletonData_findSkin(runtime->skeleton_data, skin_name);
    if (!skin) return -1;

    slot_index = spSkeletonData_findSlotIndex(runtime->skeleton_data, slot_name);
    if (slot_index < 0) return -1;

    attachment = spSkin_getAttachment(skin, slot_index, attachment_name);
    return attachment ? (int)attachment->type : -1;
}

static spAnimation* sena_spine_find_animation(
    const SenaSpineRuntime* runtime,
    const char* animation_name
) {
    if (!runtime || !runtime->skeleton_data || !animation_name) return 0;
    return spSkeletonData_findAnimation(runtime->skeleton_data, animation_name);
}

static spTimeline* sena_spine_find_timeline(
    const SenaSpineRuntime* runtime,
    const char* animation_name,
    int timeline_index
) {
    spAnimation* animation = sena_spine_find_animation(runtime, animation_name);
    if (!animation) return 0;
    if (timeline_index < 0 || timeline_index >= animation->timelinesCount) return 0;
    return animation->timelines[timeline_index];
}

int sena_spine_runtime_animation_timeline_count(
    const SenaSpineRuntime* runtime,
    const char* animation_name
) {
    spAnimation* animation = sena_spine_find_animation(runtime, animation_name);
    return animation ? animation->timelinesCount : -1;
}

int sena_spine_runtime_animation_timeline_type(
    const SenaSpineRuntime* runtime,
    const char* animation_name,
    int timeline_index
) {
    spTimeline* timeline = sena_spine_find_timeline(runtime, animation_name, timeline_index);
    return timeline ? (int)timeline->type : -1;
}

int sena_spine_runtime_animation_timeline_target_kind(
    const SenaSpineRuntime* runtime,
    const char* animation_name,
    int timeline_index
) {
    spTimeline* timeline = sena_spine_find_timeline(runtime, animation_name, timeline_index);
    if (!timeline) return -1;

    switch (timeline->type) {
        case SP_TIMELINE_ROTATE:
        case SP_TIMELINE_TRANSLATE:
        case SP_TIMELINE_SCALE:
        case SP_TIMELINE_SHEAR:
            return 1;
        case SP_TIMELINE_ATTACHMENT:
        case SP_TIMELINE_COLOR:
        case SP_TIMELINE_DEFORM:
        case SP_TIMELINE_TWOCOLOR:
            return 2;
        case SP_TIMELINE_IKCONSTRAINT:
        case SP_TIMELINE_TRANSFORMCONSTRAINT:
        case SP_TIMELINE_PATHCONSTRAINTPOSITION:
        case SP_TIMELINE_PATHCONSTRAINTSPACING:
        case SP_TIMELINE_PATHCONSTRAINTMIX:
            return 3;
        case SP_TIMELINE_EVENT:
        case SP_TIMELINE_DRAWORDER:
        default:
            return 0;
    }
}

const char* sena_spine_runtime_animation_timeline_target_name(
    const SenaSpineRuntime* runtime,
    const char* animation_name,
    int timeline_index
) {
    int index;
    spTimeline* timeline = sena_spine_find_timeline(runtime, animation_name, timeline_index);
    if (!timeline || !runtime || !runtime->skeleton_data) return 0;

    switch (timeline->type) {
        case SP_TIMELINE_ROTATE:
        case SP_TIMELINE_TRANSLATE:
        case SP_TIMELINE_SCALE:
        case SP_TIMELINE_SHEAR:
            index = ((spBaseTimeline*)timeline)->boneIndex;
            if (index < 0 || index >= runtime->skeleton_data->bonesCount) return 0;
            return runtime->skeleton_data->bones[index]->name;

        case SP_TIMELINE_ATTACHMENT:
            index = ((spAttachmentTimeline*)timeline)->slotIndex;
            break;
        case SP_TIMELINE_COLOR:
            index = ((spColorTimeline*)timeline)->slotIndex;
            break;
        case SP_TIMELINE_DEFORM:
            index = ((spDeformTimeline*)timeline)->slotIndex;
            break;
        case SP_TIMELINE_TWOCOLOR:
            index = ((spTwoColorTimeline*)timeline)->slotIndex;
            break;

        case SP_TIMELINE_IKCONSTRAINT:
            index = ((spIkConstraintTimeline*)timeline)->ikConstraintIndex;
            if (index < 0 || index >= runtime->skeleton_data->ikConstraintsCount) return 0;
            return runtime->skeleton_data->ikConstraints[index]->name;

        case SP_TIMELINE_TRANSFORMCONSTRAINT:
            index = ((spTransformConstraintTimeline*)timeline)->transformConstraintIndex;
            if (index < 0 || index >= runtime->skeleton_data->transformConstraintsCount) return 0;
            return runtime->skeleton_data->transformConstraints[index]->name;

        case SP_TIMELINE_PATHCONSTRAINTPOSITION:
            index = ((spPathConstraintPositionTimeline*)timeline)->pathConstraintIndex;
            if (index < 0 || index >= runtime->skeleton_data->pathConstraintsCount) return 0;
            return runtime->skeleton_data->pathConstraints[index]->name;
        case SP_TIMELINE_PATHCONSTRAINTSPACING:
            index = ((spPathConstraintSpacingTimeline*)timeline)->pathConstraintIndex;
            if (index < 0 || index >= runtime->skeleton_data->pathConstraintsCount) return 0;
            return runtime->skeleton_data->pathConstraints[index]->name;
        case SP_TIMELINE_PATHCONSTRAINTMIX:
            index = ((spPathConstraintMixTimeline*)timeline)->pathConstraintIndex;
            if (index < 0 || index >= runtime->skeleton_data->pathConstraintsCount) return 0;
            return runtime->skeleton_data->pathConstraints[index]->name;

        case SP_TIMELINE_EVENT:
        case SP_TIMELINE_DRAWORDER:
        default:
            return 0;
    }

    if (index < 0 || index >= runtime->skeleton_data->slotsCount) return 0;
    return runtime->skeleton_data->slots[index]->name;
}

void sena_spine_runtime_update(SenaSpineRuntime* runtime, float delta_seconds) {
    if (!runtime || delta_seconds < 0.0f) return;

    spSkeleton_update(runtime->skeleton, delta_seconds);
    spAnimationState_update(runtime->animation_state, delta_seconds);
    spAnimationState_apply(runtime->animation_state, runtime->skeleton);
    spSkeleton_updateWorldTransform(runtime->skeleton);
}

static int ensure_world_vertices(SenaSpineRuntime* runtime, int float_count) {
    float* resized;
    if (float_count <= runtime->world_vertices_capacity) return 1;

    resized = (float*)realloc(runtime->world_vertices, sizeof(float) * (size_t)float_count);
    if (!resized) return 0;

    runtime->world_vertices = resized;
    runtime->world_vertices_capacity = float_count;
    return 1;
}

static int ensure_packed_vertices(SenaSpineRuntime* runtime, int vertex_count) {
    SenaSpineVertex* resized;
    if (vertex_count <= runtime->packed_vertices_capacity) return 1;

    resized = (SenaSpineVertex*)realloc(
        runtime->packed_vertices,
        sizeof(SenaSpineVertex) * (size_t)vertex_count
    );
    if (!resized) return 0;

    runtime->packed_vertices = resized;
    runtime->packed_vertices_capacity = vertex_count;
    return 1;
}

static int sena_blend_mode(spBlendMode blend_mode) {
    switch (blend_mode) {
        case SP_BLEND_MODE_ADDITIVE: return SENA_SPINE_BLEND_ADDITIVE;
        case SP_BLEND_MODE_MULTIPLY: return SENA_SPINE_BLEND_MULTIPLY;
        case SP_BLEND_MODE_SCREEN: return SENA_SPINE_BLEND_SCREEN;
        case SP_BLEND_MODE_NORMAL:
        default:
            return SENA_SPINE_BLEND_NORMAL;
    }
}

int sena_spine_runtime_extract_frame(
    SenaSpineRuntime* runtime,
    SenaSpineBatchCallback callback,
    void* user_data
) {
    int slot_index;
    int batch_count = 0;
    spSkeleton* skeleton;
    spSkeletonClipping* clipper;

    if (!runtime || !callback) return 0;

    skeleton = runtime->skeleton;
    clipper = runtime->clipper;
    if (!skeleton || skeleton->color.a == 0.0f) return 0;

    spSkeletonClipping_clipEnd2(clipper);

    for (slot_index = 0; slot_index < skeleton->slotsCount; ++slot_index) {
        spSlot* slot = skeleton->drawOrder[slot_index];
        spAttachment* attachment = slot->attachment;
        float* vertices = 0;
        float* uvs = 0;
        unsigned short* indices = 0;
        int vertex_count = 0;
        int index_count = 0;
        spColor* attachment_color = 0;
        spAtlasRegion* atlas_region = 0;
        int vertex_index;
        float r, g, b, a;
        float dark_r, dark_g, dark_b;

        if (!attachment) continue;

        if (slot->color.a == 0.0f || !slot->bone->active) {
            spSkeletonClipping_clipEnd(clipper, slot);
            continue;
        }

        if (attachment->type == SP_ATTACHMENT_REGION) {
            spRegionAttachment* region = (spRegionAttachment*)attachment;
            attachment_color = &region->color;
            if (attachment_color->a == 0.0f) {
                spSkeletonClipping_clipEnd(clipper, slot);
                continue;
            }

            if (!ensure_world_vertices(runtime, 8)) {
                spSkeletonClipping_clipEnd2(clipper);
                return -1;
            }

            spRegionAttachment_computeWorldVertices(
                region,
                slot->bone,
                runtime->world_vertices,
                0,
                2
            );
            vertices = runtime->world_vertices;
            vertex_count = 4;
            uvs = region->uvs;
            indices = (unsigned short*)QUAD_TRIANGLES;
            index_count = 6;
            atlas_region = (spAtlasRegion*)region->rendererObject;
        } else if (
            attachment->type == SP_ATTACHMENT_MESH ||
            attachment->type == SP_ATTACHMENT_LINKED_MESH
        ) {
            spMeshAttachment* mesh = (spMeshAttachment*)attachment;
            int world_vertices_length = mesh->super.worldVerticesLength;

            attachment_color = &mesh->color;
            if (attachment_color->a == 0.0f) {
                spSkeletonClipping_clipEnd(clipper, slot);
                continue;
            }
            if (world_vertices_length <= 0) {
                spSkeletonClipping_clipEnd(clipper, slot);
                continue;
            }
            if (!ensure_world_vertices(runtime, world_vertices_length)) {
                spSkeletonClipping_clipEnd2(clipper);
                return -1;
            }

            spVertexAttachment_computeWorldVertices(
                SUPER(mesh),
                slot,
                0,
                world_vertices_length,
                runtime->world_vertices,
                0,
                2
            );
            vertices = runtime->world_vertices;
            vertex_count = world_vertices_length >> 1;
            uvs = mesh->uvs;
            indices = mesh->triangles;
            index_count = mesh->trianglesCount;
            atlas_region = (spAtlasRegion*)mesh->rendererObject;
        } else if (attachment->type == SP_ATTACHMENT_CLIPPING) {
            spSkeletonClipping_clipStart(
                clipper,
                slot,
                (spClippingAttachment*)attachment
            );
            continue;
        } else {
            continue;
        }

        if (!atlas_region || !atlas_region->page || vertex_count <= 0 || index_count <= 0) {
            spSkeletonClipping_clipEnd(clipper, slot);
            continue;
        }

        if (spSkeletonClipping_isClipping(clipper)) {
            spSkeletonClipping_clipTriangles(
                clipper,
                vertices,
                vertex_count << 1,
                indices,
                index_count,
                uvs,
                2
            );
            vertices = clipper->clippedVertices->items;
            vertex_count = clipper->clippedVertices->size >> 1;
            uvs = clipper->clippedUVs->items;
            indices = clipper->clippedTriangles->items;
            index_count = clipper->clippedTriangles->size;

            if (vertex_count <= 0 || index_count <= 0) {
                spSkeletonClipping_clipEnd(clipper, slot);
                continue;
            }
        }

        if (!ensure_packed_vertices(runtime, vertex_count)) {
            spSkeletonClipping_clipEnd2(clipper);
            return -1;
        }

        r = skeleton->color.r * slot->color.r * attachment_color->r;
        g = skeleton->color.g * slot->color.g * attachment_color->g;
        b = skeleton->color.b * slot->color.b * attachment_color->b;
        a = skeleton->color.a * slot->color.a * attachment_color->a;

        dark_r = slot->darkColor ? slot->darkColor->r : 0.0f;
        dark_g = slot->darkColor ? slot->darkColor->g : 0.0f;
        dark_b = slot->darkColor ? slot->darkColor->b : 0.0f;

        for (vertex_index = 0; vertex_index < vertex_count; ++vertex_index) {
            int float_index = vertex_index << 1;
            SenaSpineVertex* vertex = &runtime->packed_vertices[vertex_index];
            vertex->x = vertices[float_index];
            vertex->y = vertices[float_index + 1];
            vertex->u = uvs[float_index];
            vertex->v = uvs[float_index + 1];
            vertex->r = r;
            vertex->g = g;
            vertex->b = b;
            vertex->a = a;
            vertex->dark_r = dark_r;
            vertex->dark_g = dark_g;
            vertex->dark_b = dark_b;
        }

        batch_count++;
        if (!callback(
            user_data,
            atlas_region->page->name,
            slot->data->name,
            attachment->name,
            sena_blend_mode(slot->data->blendMode),
            runtime->packed_vertices,
            vertex_count,
            indices,
            index_count
        )) {
            spSkeletonClipping_clipEnd2(clipper);
            return batch_count;
        }

        spSkeletonClipping_clipEnd(clipper, slot);
    }

    spSkeletonClipping_clipEnd2(clipper);
    return batch_count;
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
