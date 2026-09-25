"""Import and inspect a candidate anime base model for Sena.

Supports GLB/GLTF with stock Blender. VRM requires VRM Add-on for Blender.

Usage:
    blender --background --python tools/blender/inspect_sena_base.py -- \
      --model pets/sena/models/base_candidates/base.vrm \
      --output pets/sena/models/generated/base_review
"""

from __future__ import annotations

import argparse
import hashlib
import json
import math
import sys
from pathlib import Path

import bpy
from mathutils import Vector


HUMANOID_ALIASES = {
    "hips": ("hips", "hip", "pelvis", "j_bip_c_hips"),
    "spine": ("spine", "j_bip_c_spine"),
    "chest": ("chest", "upperchest", "upper_chest", "j_bip_c_chest"),
    "neck": ("neck", "j_bip_c_neck"),
    "head": ("head", "j_bip_c_head"),
    "left_upper_arm": ("leftupperarm", "upperarm_l", "left_arm", "j_bip_l_upperarm"),
    "right_upper_arm": ("rightupperarm", "upperarm_r", "right_arm", "j_bip_r_upperarm"),
    "left_lower_arm": ("leftlowerarm", "lowerarm_l", "left_forearm", "j_bip_l_lowerarm"),
    "right_lower_arm": ("rightlowerarm", "lowerarm_r", "right_forearm", "j_bip_r_lowerarm"),
    "left_hand": ("lefthand", "hand_l", "j_bip_l_hand"),
    "right_hand": ("righthand", "hand_r", "j_bip_r_hand"),
    "left_upper_leg": ("leftupperleg", "thigh_l", "left_thigh", "j_bip_l_upperleg"),
    "right_upper_leg": ("rightupperleg", "thigh_r", "right_thigh", "j_bip_r_upperleg"),
    "left_lower_leg": ("leftlowerleg", "calf_l", "left_calf", "j_bip_l_lowerleg"),
    "right_lower_leg": ("rightlowerleg", "calf_r", "right_calf", "j_bip_r_lowerleg"),
    "left_foot": ("leftfoot", "foot_l", "j_bip_l_foot"),
    "right_foot": ("rightfoot", "foot_r", "j_bip_r_foot"),
}


def parse_args() -> argparse.Namespace:
    argv = sys.argv
    argv = argv[argv.index("--") + 1 :] if "--" in argv else []
    parser = argparse.ArgumentParser()
    parser.add_argument("--model", required=True)
    parser.add_argument(
        "--output",
        default="pets/sena/models/generated/base_review",
    )
    return parser.parse_args(argv)


def reset_scene() -> None:
    bpy.ops.object.select_all(action="SELECT")
    bpy.ops.object.delete(use_global=False)


def vrm_operator_available() -> bool:
    try:
        bpy.ops.import_scene.vrm.get_rna_type()
        return True
    except (AttributeError, RuntimeError):
        return False


def import_model(path: Path) -> None:
    suffix = path.suffix.lower()
    if suffix in {".glb", ".gltf"}:
        bpy.ops.import_scene.gltf(filepath=str(path.resolve()))
        return

    if suffix == ".vrm":
        # bpy.ops is dynamic, so hasattr() is not a reliable registration test.
        # Querying the operator RNA fails when VRM Add-on is not registered.
        if not vrm_operator_available():
            raise RuntimeError(
                "VRM Add-on for Blender is not installed/enabled. "
                "Install 'VRM format' in Blender 4.2+ via "
                "Edit > Preferences > Get Extensions, then retry."
            )
        bpy.ops.import_scene.vrm(filepath=str(path.resolve()))
        return

    raise RuntimeError(f"Unsupported candidate format: {suffix}. Use .vrm/.glb/.gltf")


def sha256(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as file:
        for chunk in iter(lambda: file.read(1024 * 1024), b""):
            digest.update(chunk)
    return digest.hexdigest()


def evaluated_triangle_count(obj: bpy.types.Object, depsgraph) -> int:
    evaluated = obj.evaluated_get(depsgraph)
    mesh = evaluated.to_mesh()
    try:
        mesh.calc_loop_triangles()
        return len(mesh.loop_triangles)
    finally:
        evaluated.to_mesh_clear()


def scene_bounds() -> tuple[Vector, Vector]:
    minimum = Vector((math.inf, math.inf, math.inf))
    maximum = Vector((-math.inf, -math.inf, -math.inf))
    found = False

    depsgraph = bpy.context.evaluated_depsgraph_get()
    for obj in bpy.context.scene.objects:
        if obj.type != "MESH":
            continue
        evaluated = obj.evaluated_get(depsgraph)
        for corner in evaluated.bound_box:
            world = evaluated.matrix_world @ Vector(corner)
            minimum.x = min(minimum.x, world.x)
            minimum.y = min(minimum.y, world.y)
            minimum.z = min(minimum.z, world.z)
            maximum.x = max(maximum.x, world.x)
            maximum.y = max(maximum.y, world.y)
            maximum.z = max(maximum.z, world.z)
        found = True

    if not found:
        raise RuntimeError("Imported model contains no mesh objects")

    return minimum, maximum


def normalized_bone_name(name: str) -> str:
    return "".join(ch for ch in name.lower() if ch.isalnum() or ch == "_")


def humanoid_report() -> dict[str, object]:
    bones = []
    for obj in bpy.context.scene.objects:
        if obj.type == "ARMATURE":
            bones.extend(bone.name for bone in obj.data.bones)

    normalized = {normalized_bone_name(name): name for name in bones}
    found = {}
    for semantic, aliases in HUMANOID_ALIASES.items():
        match = None
        for alias in aliases:
            key = normalized_bone_name(alias)
            for normalized_name, original in normalized.items():
                if normalized_name == key or key in normalized_name:
                    match = original
                    break
            if match:
                break
        found[semantic] = match

    present = sum(value is not None for value in found.values())
    return {
        "bone_count": len(bones),
        "required_found": present,
        "required_total": len(HUMANOID_ALIASES),
        "bones": found,
    }


def setup_review_scene(minimum: Vector, maximum: Vector) -> bpy.types.Object:
    center = (minimum + maximum) * 0.5
    size = maximum - minimum
    height = max(size.z, 0.01)

    # Neutral studio lighting: no dramatic rim light that can hide bad forms.
    bpy.ops.object.light_add(type="AREA", location=(2.8, -3.5, center.z + height * 0.65))
    key = bpy.context.object
    key.name = "BaseReviewKey"
    key.data.energy = 520
    key.data.shape = "DISK"
    key.data.size = max(height * 1.2, 2.0)
    key.rotation_euler = (center - key.location).to_track_quat("-Z", "Y").to_euler()

    bpy.ops.object.light_add(type="AREA", location=(-2.4, -2.0, center.z + height * 0.25))
    fill = bpy.context.object
    fill.name = "BaseReviewFill"
    fill.data.energy = 260
    fill.data.size = max(height * 1.4, 2.5)
    fill.rotation_euler = (center - fill.location).to_track_quat("-Z", "Y").to_euler()

    bpy.ops.object.camera_add()
    camera = bpy.context.object
    camera.name = "BaseReviewCamera"
    camera.data.type = "ORTHO"
    camera.data.ortho_scale = max(height * 1.12, 0.8)
    bpy.context.scene.camera = camera

    scene = bpy.context.scene
    scene.render.engine = "BLENDER_EEVEE"
    scene.render.resolution_x = 720
    scene.render.resolution_y = 720
    scene.render.resolution_percentage = 100
    scene.render.image_settings.file_format = "PNG"
    scene.render.film_transparent = True
    try:
        scene.render.use_freestyle = False
        scene.view_settings.view_transform = "Standard"
    except (AttributeError, TypeError):
        pass

    return camera


def look_at(camera: bpy.types.Object, location: Vector, target: Vector) -> None:
    camera.location = location
    camera.rotation_euler = (target - location).to_track_quat("-Z", "Y").to_euler()


def render_views(output: Path, minimum: Vector, maximum: Vector) -> None:
    camera = setup_review_scene(minimum, maximum)
    center = (minimum + maximum) * 0.5
    size = maximum - minimum
    distance = max(size.length * 2.2, 3.0)

    views = {
        "front": center + Vector((0.0, -distance, size.z * 0.03)),
        "three_quarter": center + Vector((distance * 0.57, -distance * 0.82, size.z * 0.03)),
        "side": center + Vector((distance, -distance * 0.04, size.z * 0.03)),
    }

    for name, position in views.items():
        look_at(camera, position, center)
        bpy.context.scene.render.filepath = str((output / f"{name}.png").resolve())
        bpy.ops.render.render(write_still=True)
        print(f"[Sena base] wrote {output / f'{name}.png'}")


def build_report(model_path: Path) -> dict[str, object]:
    depsgraph = bpy.context.evaluated_depsgraph_get()
    mesh_objects = [obj for obj in bpy.context.scene.objects if obj.type == "MESH"]
    armatures = [obj for obj in bpy.context.scene.objects if obj.type == "ARMATURE"]

    triangles = {
        obj.name: evaluated_triangle_count(obj, depsgraph)
        for obj in mesh_objects
    }
    materials = sorted(
        {
            slot.material.name
            for obj in mesh_objects
            for slot in obj.material_slots
            if slot.material is not None
        }
    )
    minimum, maximum = scene_bounds()
    size = maximum - minimum

    return {
        "source_file": str(model_path),
        "sha256": sha256(model_path),
        "mesh_count": len(mesh_objects),
        "armature_count": len(armatures),
        "material_count": len(materials),
        "materials": materials,
        "triangle_count": sum(triangles.values()),
        "triangles_by_mesh": dict(sorted(triangles.items(), key=lambda item: item[1], reverse=True)),
        "bounds": {
            "min": list(minimum),
            "max": list(maximum),
            "size": list(size),
            "height_z": size.z,
        },
        "humanoid": humanoid_report(),
    }


def main() -> None:
    options = parse_args()
    model_path = Path(options.model)
    output = Path(options.output)

    if not model_path.is_file():
        raise RuntimeError(f"Candidate model not found: {model_path}")

    output.mkdir(parents=True, exist_ok=True)
    reset_scene()
    import_model(model_path)

    report = build_report(model_path)
    minimum, maximum = scene_bounds()
    render_views(output, minimum, maximum)

    blend_path = output / "imported.blend"
    bpy.ops.wm.save_as_mainfile(filepath=str(blend_path.resolve()))

    report_path = output / "report.json"
    report_path.write_text(
        json.dumps(report, indent=2, ensure_ascii=False),
        encoding="utf-8",
    )

    print(f"[Sena base] report: {report_path}")
    print(json.dumps(report, indent=2, ensure_ascii=False))


if __name__ == "__main__":
    main()
