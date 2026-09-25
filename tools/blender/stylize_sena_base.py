"""Stylize an imported VRoid base into Sena's first chibi proportion pass.

This script never generates replacement character meshes. It deforms the
existing VRoid Face/Body/Hair topology and the rest armature together so skin
weights and expression shape keys remain usable.

Run through build_sena_base_chibi.ps1.
"""

from __future__ import annotations

import argparse
import json
import math
import sys
from pathlib import Path

import bpy
from mathutils import Vector

SCRIPT_DIR = Path(__file__).resolve().parent
if str(SCRIPT_DIR) not in sys.path:
    sys.path.insert(0, str(SCRIPT_DIR))

import inspect_sena_base as review  # noqa: E402


Z_REMAP = (
    (0.000, 0.000),
    (0.086, 0.060),
    (0.495, 0.230),
    (0.895, 0.405),
    (1.059, 0.485),
    (1.160, 0.540),
    (1.280, 0.605),
    (1.350, 0.645),
    (1.558, 0.950),
)


def args() -> argparse.Namespace:
    argv = sys.argv
    argv = argv[argv.index("--") + 1 :] if "--" in argv else []
    parser = argparse.ArgumentParser()
    parser.add_argument(
        "--output",
        default="pets/sena/models/generated/base_chibi_01",
    )
    return parser.parse_args(argv)


def clamp(value: float, lo: float, hi: float) -> float:
    return max(lo, min(hi, value))


def smoothstep(edge0: float, edge1: float, value: float) -> float:
    if edge1 <= edge0:
        return 1.0 if value >= edge1 else 0.0
    t = clamp((value - edge0) / (edge1 - edge0), 0.0, 1.0)
    return t * t * (3.0 - 2.0 * t)


def remap_z(z: float) -> float:
    if z <= Z_REMAP[0][0]:
        return Z_REMAP[0][1]

    for (x0, y0), (x1, y1) in zip(Z_REMAP, Z_REMAP[1:]):
        if z <= x1:
            t = (z - x0) / (x1 - x0)
            return y0 + (y1 - y0) * t

    x0, y0 = Z_REMAP[-2]
    x1, y1 = Z_REMAP[-1]
    slope = (y1 - y0) / (x1 - x0)
    return y1 + (z - x1) * slope


def global_xy_scales(z: float) -> tuple[float, float]:
    """Shorten limbs/shoulders while enlarging the head as one coherent mass."""

    head_blend = smoothstep(1.185, 1.330, z)
    x_scale = 0.78 + (1.22 - 0.78) * head_blend
    y_scale = 0.94 + (1.10 - 0.94) * head_blend
    return x_scale, y_scale


def compact_arm_x(x: float, z: float) -> float:
    """Shorten horizontal T-pose arm span without squeezing the torso."""

    if not 1.055 <= z <= 1.320:
        return x

    sign = -1.0 if x < 0.0 else 1.0
    distance = abs(x)
    shoulder = 0.105
    if distance <= shoulder:
        return x

    return sign * (shoulder + (distance - shoulder) * 0.66)


def global_point(point: Vector, *, compact_arms: bool = False) -> Vector:
    x_scale, y_scale = global_xy_scales(point.z)
    x = point.x * x_scale
    if compact_arms:
        x = compact_arm_x(x, point.z)

    return Vector(
        (
            x,
            point.y * y_scale,
            remap_z(point.z),
        )
    )


def face_shape_point(point: Vector, original_z: float) -> Vector:
    """Taper the lower face without replacing VRoid's mature face topology."""

    # Chin is narrower, cheeks retain volume, upper head remains unchanged.
    lower_t = smoothstep(1.215, 1.365, original_z)
    lower_factor = 0.82 + 0.18 * lower_t

    # Slight cheek fullness around the eye/lower-eye band.
    cheek = math.exp(-((original_z - 1.365) / 0.055) ** 2)
    cheek_factor = 1.0 + 0.018 * cheek

    point.x *= lower_factor * cheek_factor

    # Keep the profile anime-flat but add a little more cranium depth above
    # the brow so the side view does not become a scaled-up flat mask.
    upper_t = smoothstep(1.395, 1.520, original_z)
    point.y *= 1.0 + 0.035 * upper_t

    return point


def transform_mesh_object(obj: bpy.types.Object) -> None:
    original = [vertex.co.copy() for vertex in obj.data.vertices]

    def transform(index: int, co: Vector) -> Vector:
        old = original[index]
        result = global_point(co, compact_arms=obj.name == "Body")
        if obj.name == "Face":
            result = face_shape_point(result, old.z)
        return result

    if obj.data.shape_keys:
        for key_block in obj.data.shape_keys.key_blocks:
            for index, point in enumerate(key_block.data):
                point.co = transform(index, point.co.copy())
    else:
        for index, vertex in enumerate(obj.data.vertices):
            vertex.co = transform(index, vertex.co.copy())

    obj.data.update()


def material_vertex_indices(obj: bpy.types.Object, keywords: tuple[str, ...]) -> set[int]:
    material_indices = {
        index
        for index, slot in enumerate(obj.material_slots)
        if slot.material
        and any(keyword.lower() in slot.material.name.lower() for keyword in keywords)
    }
    result: set[int] = set()
    for polygon in obj.data.polygons:
        if polygon.material_index in material_indices:
            result.update(polygon.vertices)
    return result


def eye_groups(face: bpy.types.Object) -> tuple[list[int], list[int]]:
    indices = material_vertex_indices(
        face,
        ("EyeIris", "EyeHighlight", "EyeWhite", "FaceEyeline"),
    )
    left = [index for index in indices if face.data.vertices[index].co.x < 0.0]
    right = [index for index in indices if face.data.vertices[index].co.x >= 0.0]
    return left, right


def scale_shape_key_region(
    obj: bpy.types.Object,
    indices: list[int],
    *,
    scale_x: float,
    scale_z: float,
    move_z: float = 0.0,
) -> None:
    if not indices:
        return

    basis = (
        obj.data.shape_keys.key_blocks["Basis"].data
        if obj.data.shape_keys
        else obj.data.vertices
    )
    center = Vector(
        (
            sum(basis[index].co.x for index in indices) / len(indices),
            sum(basis[index].co.y for index in indices) / len(indices),
            sum(basis[index].co.z for index in indices) / len(indices),
        )
    )

    if obj.data.shape_keys:
        for key_block in obj.data.shape_keys.key_blocks:
            for index in indices:
                co = key_block.data[index].co
                co.x = center.x + (co.x - center.x) * scale_x
                co.z = center.z + (co.z - center.z) * scale_z + move_z
    else:
        for index in indices:
            co = obj.data.vertices[index].co
            co.x = center.x + (co.x - center.x) * scale_x
            co.z = center.z + (co.z - center.z) * scale_z + move_z

    obj.data.update()


def refine_face_features(face: bpy.types.Object) -> None:
    left, right = eye_groups(face)
    # VRoid's eye construction is already good. The first Sena pass enlarges
    # it only slightly and more horizontally than vertically to avoid toy eyes.
    for group in (left, right):
        scale_shape_key_region(
            face,
            group,
            scale_x=1.075,
            scale_z=1.035,
            move_z=-0.002,
        )

    mouth = sorted(material_vertex_indices(face, ("FaceMouth",)))
    if mouth:
        scale_shape_key_region(
            face,
            mouth,
            scale_x=0.90,
            scale_z=0.95,
        )


def transform_armature(armature: bpy.types.Object) -> None:
    bpy.context.view_layer.objects.active = armature
    bpy.ops.object.mode_set(mode="EDIT")
    try:
        for bone in armature.data.edit_bones:
            bone.head = global_point(bone.head.copy(), compact_arms=True)
            bone.tail = global_point(bone.tail.copy(), compact_arms=True)
    finally:
        bpy.ops.object.mode_set(mode="OBJECT")


def remove_review_objects() -> None:
    for obj in list(bpy.context.scene.objects):
        if obj.type in {"CAMERA", "LIGHT"}:
            bpy.data.objects.remove(obj, do_unlink=True)


def set_review_pose(armature: bpy.types.Object) -> None:
    """Use a relaxed A-pose for art review without changing the rest rig."""

    bpy.context.view_layer.objects.active = armature
    bpy.ops.object.mode_set(mode="POSE")
    try:
        rotations = {
            "J_Bip_L_UpperArm": math.radians(58.0),
            "J_Bip_R_UpperArm": math.radians(-58.0),
        }
        for name, angle in rotations.items():
            bone = armature.pose.bones.get(name)
            if bone is None:
                continue
            bone.rotation_mode = "XYZ"
            bone.rotation_euler = (0.0, angle, 0.0)
    finally:
        bpy.ops.object.mode_set(mode="OBJECT")
    bpy.context.view_layer.update()


def clear_review_pose(armature: bpy.types.Object) -> None:
    bpy.context.view_layer.objects.active = armature
    bpy.ops.object.mode_set(mode="POSE")
    try:
        for bone in armature.pose.bones:
            bone.rotation_mode = "QUATERNION"
            bone.rotation_quaternion = (1.0, 0.0, 0.0, 0.0)
            bone.location = (0.0, 0.0, 0.0)
            bone.scale = (1.0, 1.0, 1.0)
    finally:
        bpy.ops.object.mode_set(mode="OBJECT")
    bpy.context.view_layer.update()


def write_report(output: Path) -> dict[str, object]:
    minimum, maximum = review.scene_bounds()
    report = {
        "stage": "Sena Base Chibi 01",
        "status": "WIP",
        "bounds": {
            "min": list(minimum),
            "max": list(maximum),
            "size": list(maximum - minimum),
        },
        "triangle_count": sum(
            review.evaluated_triangle_count(
                obj,
                bpy.context.evaluated_depsgraph_get(),
            )
            for obj in bpy.context.scene.objects
            if obj.type == "MESH"
        ),
        "humanoid": review.humanoid_report(),
        "transform": {
            "z_remap": Z_REMAP,
            "body_x_scale": 0.78,
            "head_x_scale": 1.22,
            "body_y_scale": 0.94,
            "head_y_scale": 1.10,
            "eye_scale_x": 1.075,
            "eye_scale_z": 1.035,
            "arm_extension_scale": 0.66,
            "review_pose": "relaxed_A_pose",
        },
    }
    (output / "report.json").write_text(
        json.dumps(report, indent=2, ensure_ascii=False),
        encoding="utf-8",
    )
    return report


def main() -> None:
    options = args()
    output = Path(options.output)
    output.mkdir(parents=True, exist_ok=True)

    armatures = [obj for obj in bpy.context.scene.objects if obj.type == "ARMATURE"]
    if len(armatures) != 1:
        raise RuntimeError(f"Expected one armature, found {len(armatures)}")

    meshes = {
        obj.name: obj
        for obj in bpy.context.scene.objects
        if obj.type == "MESH"
    }
    required = {"Face", "Body", "Hair"}
    missing = required - set(meshes)
    if missing:
        raise RuntimeError(f"Missing VRoid meshes: {sorted(missing)}")

    # Mesh and rest armature receive the exact same base-space transform.
    for name in ("Face", "Body", "Hair"):
        transform_mesh_object(meshes[name])
    refine_face_features(meshes["Face"])
    transform_armature(armatures[0])

    remove_review_objects()
    set_review_pose(armatures[0])
    minimum, maximum = review.scene_bounds()
    review.render_views(output, minimum, maximum)
    clear_review_pose(armatures[0])

    blend_path = output / "sena_base_chibi_01.blend"
    bpy.ops.wm.save_as_mainfile(filepath=str(blend_path.resolve()))

    report = write_report(output)
    print(f"[Sena chibi base] wrote {blend_path}")
    print(json.dumps(report, indent=2, ensure_ascii=False))


if __name__ == "__main__":
    main()
