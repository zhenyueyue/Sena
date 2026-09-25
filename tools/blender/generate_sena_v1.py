"""Generate the first procedural chibi Sena + cat prototype in Blender.

Run with:
    blender --background --python tools/blender/generate_sena_v1.py -- --output pets/sena/models/generated

This is intentionally a clean, editable blockout rather than the final art model.
It establishes proportions, material language, humanoid bones, attachment anchors,
and named actions before manual/AI-assisted refinement.
"""

from __future__ import annotations

import argparse
import math
import sys
from pathlib import Path

import bpy
from mathutils import Vector


ROOT_NAME = "SenaV1"
MODEL_HEIGHT = 1.35


def cli_args() -> argparse.Namespace:
    argv = sys.argv
    argv = argv[argv.index("--") + 1 :] if "--" in argv else []
    parser = argparse.ArgumentParser()
    parser.add_argument(
        "--output",
        default="pets/sena/models/generated",
        help="Directory for sena_v1.blend and sena_v1.glb",
    )
    return parser.parse_args(argv)


def reset_scene() -> None:
    bpy.ops.object.select_all(action="SELECT")
    bpy.ops.object.delete(use_global=False)
    for datablocks in (
        bpy.data.meshes,
        bpy.data.curves,
        bpy.data.armatures,
        bpy.data.materials,
    ):
        # Orphan blocks from a previous scripted run make names unstable.
        for block in list(datablocks):
            if block.users == 0:
                datablocks.remove(block)


def material(name: str, rgba: tuple[float, float, float, float], roughness: float = 0.62):
    mat = bpy.data.materials.new(name)
    mat.diffuse_color = rgba
    mat.use_nodes = True
    # Blender localizes node display names (for example Chinese builds rename
    # "Principled BSDF"), so locate the shader by stable node type instead.
    bsdf = next((node for node in mat.node_tree.nodes if node.type == "BSDF_PRINCIPLED"), None)
    if bsdf:
        bsdf.inputs["Base Color"].default_value = rgba
        bsdf.inputs["Roughness"].default_value = roughness
        bsdf.inputs["Metallic"].default_value = 0.0
        bsdf.inputs["Alpha"].default_value = rgba[3]
    return mat


def assign_material(obj, mat) -> None:
    if obj.data and hasattr(obj.data, "materials"):
        obj.data.materials.append(mat)


def smooth(obj) -> None:
    if obj.type != "MESH":
        return
    for polygon in obj.data.polygons:
        polygon.use_smooth = True


def uv_sphere(name: str, location, scale, mat, segments: int = 32, rings: int = 20):
    bpy.ops.mesh.primitive_uv_sphere_add(
        segments=segments,
        ring_count=rings,
        location=location,
    )
    obj = bpy.context.object
    obj.name = name
    obj.scale = scale
    bpy.ops.object.transform_apply(location=False, rotation=False, scale=True)
    assign_material(obj, mat)
    smooth(obj)
    return obj


def cone(name: str, location, radius1: float, radius2: float, depth: float, mat, vertices: int = 32):
    bpy.ops.mesh.primitive_cone_add(
        vertices=vertices,
        radius1=radius1,
        radius2=radius2,
        depth=depth,
        location=location,
    )
    obj = bpy.context.object
    obj.name = name
    assign_material(obj, mat)
    smooth(obj)
    return obj


def cube(name: str, location, scale, mat, bevel: float = 0.0):
    bpy.ops.mesh.primitive_cube_add(location=location)
    obj = bpy.context.object
    obj.name = name
    obj.scale = scale
    bpy.ops.object.transform_apply(location=False, rotation=False, scale=True)
    assign_material(obj, mat)
    if bevel > 0:
        modifier = obj.modifiers.new("SoftEdges", "BEVEL")
        modifier.width = bevel
        modifier.segments = 3
    return obj


def curve_tube(name: str, points, radius: float, mat):
    curve_data = bpy.data.curves.new(name, type="CURVE")
    curve_data.dimensions = "3D"
    curve_data.resolution_u = 3
    curve_data.bevel_depth = radius
    curve_data.bevel_resolution = 3
    spline = curve_data.splines.new("BEZIER")
    spline.bezier_points.add(len(points) - 1)
    for point, coordinate in zip(spline.bezier_points, points):
        point.co = coordinate
        point.handle_left_type = "AUTO"
        point.handle_right_type = "AUTO"
    obj = bpy.data.objects.new(name, curve_data)
    bpy.context.collection.objects.link(obj)
    assign_material(obj, mat)
    return obj


def star(name: str, location, outer_radius: float, inner_radius: float, depth: float, mat):
    vertices = []
    for i in range(10):
        angle = math.radians(90 + i * 36)
        radius = outer_radius if i % 2 == 0 else inner_radius
        vertices.append((math.cos(angle) * radius, 0.0, math.sin(angle) * radius))
    vertices += [(x, depth, z) for x, _, z in vertices]
    faces = []
    faces.append(tuple(range(10)))
    faces.append(tuple(range(19, 9, -1)))
    for i in range(10):
        j = (i + 1) % 10
        faces.append((i, j, j + 10, i + 10))
    mesh = bpy.data.meshes.new(name)
    mesh.from_pydata(vertices, [], faces)
    mesh.update()
    obj = bpy.data.objects.new(name, mesh)
    obj.location = location
    bpy.context.collection.objects.link(obj)
    assign_material(obj, mat)
    return obj


def parent_keep_world(obj, parent, bone: str | None = None) -> None:
    matrix = obj.matrix_world.copy()
    obj.parent = parent
    if bone:
        obj.parent_type = "BONE"
        obj.parent_bone = bone
    obj.matrix_world = matrix


def create_armature() -> bpy.types.Object:
    armature_data = bpy.data.armatures.new("SenaArmature")
    armature = bpy.data.objects.new("SenaArmature", armature_data)
    bpy.context.collection.objects.link(armature)
    bpy.context.view_layer.objects.active = armature
    armature.select_set(True)
    bpy.ops.object.mode_set(mode="EDIT")

    def bone(name: str, head, tail, parent: str | None = None):
        b = armature_data.edit_bones.new(name)
        b.head = head
        b.tail = tail
        if parent:
            b.parent = armature_data.edit_bones[parent]
        return b

    bone("Hips", (0, 0, 0.54), (0, 0, 0.66))
    bone("Spine", (0, 0, 0.66), (0, 0, 0.79), "Hips")
    bone("Chest", (0, 0, 0.79), (0, 0, 0.91), "Spine")
    bone("Neck", (0, 0, 0.91), (0, 0, 0.99), "Chest")
    bone("Head", (0, 0, 0.99), (0, 0, 1.22), "Neck")

    for side, sign in (("Left", 1), ("Right", -1)):
        x0 = 0.17 * sign
        x1 = 0.33 * sign
        x2 = 0.45 * sign
        bone(f"{side}UpperArm", (x0, 0, 0.84), (x1, 0, 0.70), "Chest")
        bone(f"{side}LowerArm", (x1, 0, 0.70), (x2, -0.01, 0.60), f"{side}UpperArm")
        bone(f"{side}Hand", (x2, -0.01, 0.60), (x2 + 0.07 * sign, -0.02, 0.56), f"{side}LowerArm")

        lx = 0.11 * sign
        bone(f"{side}UpperLeg", (lx, 0, 0.55), (lx, 0, 0.31), "Hips")
        bone(f"{side}LowerLeg", (lx, 0, 0.31), (lx, -0.01, 0.11), f"{side}UpperLeg")
        bone(f"{side}Foot", (lx, -0.01, 0.11), (lx, -0.12, 0.07), f"{side}LowerLeg")

    # Runtime attachment contract.
    bone("CatCarry", (0, -0.19, 0.73), (0, -0.28, 0.73), "Chest")
    bone("RightHandProp", (-0.51, -0.02, 0.56), (-0.58, -0.02, 0.56), "RightHand")
    bone("LeftHandProp", (0.51, -0.02, 0.56), (0.58, -0.02, 0.56), "LeftHand")
    bone("Headphones", (0, 0, 1.18), (0, 0.06, 1.18), "Head")
    bone("Laptop", (0, -0.20, 0.55), (0, -0.30, 0.55), "Hips")

    bpy.ops.object.mode_set(mode="OBJECT")
    armature.show_in_front = True
    return armature


def create_sena(armature: bpy.types.Object) -> None:
    skin = material("Skin", (1.0, 0.84, 0.88, 1.0), 0.78)
    hair = material("MoonlightHair", (0.93, 0.91, 1.0, 1.0), 0.60)
    hair_shadow = material("MoonlightHairShadow", (0.76, 0.69, 0.94, 1.0), 0.62)
    white = material("PearlWhite", (0.97, 0.97, 1.0, 1.0), 0.68)
    lilac = material("CrystalLilac", (0.66, 0.43, 0.96, 1.0), 0.40)
    pale_lilac = material("PaleLilac", (0.84, 0.72, 1.0, 1.0), 0.62)
    ice_blue = material("IceBlue", (0.66, 0.86, 1.0, 1.0), 0.54)
    eye_white = material("EyeWhite", (1.0, 0.99, 1.0, 1.0), 0.48)
    iris = material("VioletPinkEye", (0.56, 0.18, 0.76, 1.0), 0.32)
    dark = material("LashDark", (0.16, 0.11, 0.22, 1.0), 0.64)
    blush = material("Blush", (1.0, 0.48, 0.62, 1.0), 0.72)

    # Compact hourglass-like chibi torso rather than a single oval body.
    torso = uv_sphere("Body", (0, 0.015, 0.745), (0.180, 0.130, 0.175), white)
    parent_keep_world(torso, armature, "Chest")
    waist_body = uv_sphere("WaistBody", (0, 0.020, 0.605), (0.145, 0.112, 0.120), pale_lilac)
    parent_keep_world(waist_body, armature, "Spine")

    # Slightly tapered/flattened head gives a more anime face than the V1 sphere.
    head = uv_sphere("HeadMesh", (0, -0.02, 1.105), (0.258, 0.215, 0.245), skin, 48, 32)
    parent_keep_world(head, armature, "Head")

    # Back cap + separated long locks establish the silver-white silhouette.
    back_hair = uv_sphere("BackHair", (0, 0.070, 1.085), (0.275, 0.175, 0.300), hair)
    parent_keep_world(back_hair, armature, "Head")
    for index, (x, z, sx, sz) in enumerate((
        (-0.205, 0.82, 0.070, 0.34),
        (-0.105, 0.76, 0.075, 0.40),
        (0.000, 0.74, 0.080, 0.42),
        (0.105, 0.76, 0.075, 0.40),
        (0.205, 0.82, 0.070, 0.34),
    )):
        lock = uv_sphere(f"HairBackLock{index}", (x, 0.115, z), (sx, 0.065, sz), hair_shadow)
        parent_keep_world(lock, armature, "Head")
    for side, x in (("L", -0.225), ("R", 0.225)):
        lock = uv_sphere(f"HairSide{side}", (x, -0.075, 0.92), (0.060, 0.052, 0.285), hair)
        parent_keep_world(lock, armature, "Head")
    for index, (x, z, angle, radius) in enumerate((
        (-0.135, 1.235, -0.30, 0.070),
        (-0.045, 1.250, -0.12, 0.064),
        (0.048, 1.250, 0.11, 0.061),
        (0.132, 1.230, 0.28, 0.066),
    )):
        bang = cone(f"Bang{index}", (x, -0.220, z), 0.022, radius, 0.235, hair, 28)
        bang.rotation_euler[1] = angle
        parent_keep_world(bang, armature, "Head")

    # Layered anime eyes with independent meshes for future BlinkLeft/BlinkRight shapes.
    for index, x in enumerate((-0.090, 0.090)):
        suffix = "L" if index == 0 else "R"
        eye = uv_sphere(f"EyeWhite{suffix}", (x, -0.222, 1.135), (0.061, 0.009, 0.087), eye_white, 28, 18)
        iris_mesh = uv_sphere(f"Iris{suffix}", (x, -0.230, 1.132), (0.035, 0.006, 0.054), iris, 24, 16)
        pupil = uv_sphere(f"Pupil{suffix}", (x, -0.235, 1.132), (0.013, 0.0035, 0.028), dark, 20, 12)
        highlight = uv_sphere(f"EyeHighlight{suffix}", (x - 0.010, -0.239, 1.154), (0.010, 0.0025, 0.014), eye_white, 16, 10)
        for obj in (eye, iris_mesh, pupil, highlight):
            parent_keep_world(obj, armature, "Head")

        sign = -1 if x < 0 else 1
        lash = curve_tube(
            f"UpperLash{suffix}",
            [
                (x - 0.055 * sign, -0.245, 1.178),
                (x, -0.255, 1.191),
                (x + 0.060 * sign, -0.244, 1.174),
            ],
            0.0065,
            dark,
        )
        parent_keep_world(lash, armature, "Head")
        brow = curve_tube(
            f"Brow{suffix}",
            [
                (x - 0.045, -0.224, 1.225),
                (x, -0.232, 1.234),
                (x + 0.045, -0.224, 1.225),
            ],
            0.0045,
            hair_shadow,
        )
        parent_keep_world(brow, armature, "Head")

    # Tiny nose/mouth + subtle cheek blush keep the face readable without realism.
    nose = uv_sphere("Nose", (0, -0.238, 1.080), (0.015, 0.008, 0.011), skin, 16, 10)
    mouth = curve_tube(
        "Mouth",
        [(-0.035, -0.242, 1.042), (0, -0.250, 1.032), (0.035, -0.242, 1.042)],
        0.006,
        blush,
    )
    blush_l = uv_sphere("FaceBlushL", (-0.165, -0.225, 1.064), (0.040, 0.006, 0.018), blush, 18, 10)
    blush_r = uv_sphere("FaceBlushR", (0.165, -0.225, 1.064), (0.040, 0.006, 0.018), blush, 18, 10)
    for obj in (nose, mouth, blush_l, blush_r):
        parent_keep_world(obj, armature, "Head")

    # Layered crystal dress: fitted bodice, compact bell skirt and ice-blue hem.
    bodice = cone("DressBodice", (0, -0.005, 0.69), 0.190, 0.155, 0.26, white, 40)
    parent_keep_world(bodice, armature, "Chest")
    neckline = curve_tube(
        "Neckline",
        [(-0.090, -0.150, 0.835), (0, -0.162, 0.790), (0.090, -0.150, 0.835)],
        0.010,
        lilac,
    )
    parent_keep_world(neckline, armature, "Chest")
    dress = cone("Dress", (0, 0.010, 0.47), 0.295, 0.155, 0.40, pale_lilac, 48)
    parent_keep_world(dress, armature, "Hips")
    underskirt = cone("DressUnderLayer", (0, 0.030, 0.40), 0.270, 0.165, 0.25, white, 48)
    parent_keep_world(underskirt, armature, "Hips")
    hem = cone("IceBlueHem", (0, -0.005, 0.315), 0.300, 0.270, 0.075, ice_blue, 48)
    parent_keep_world(hem, armature, "Hips")
    waist = uv_sphere("WaistCrystal", (0, -0.170, 0.655), (0.060, 0.020, 0.052), lilac, 24, 16)
    parent_keep_world(waist, armature, "Spine")
    waist_wing_l = uv_sphere("WaistWingL", (-0.060, -0.166, 0.660), (0.055, 0.015, 0.032), ice_blue, 20, 12)
    waist_wing_r = uv_sphere("WaistWingR", (0.060, -0.166, 0.660), (0.055, 0.015, 0.032), ice_blue, 20, 12)
    waist_wing_l.rotation_euler[1] = math.radians(-22)
    waist_wing_r.rotation_euler[1] = math.radians(22)
    for obj in (waist_wing_l, waist_wing_r):
        parent_keep_world(obj, armature, "Spine")

    # Small collar butterfly keeps the torso from reading as a blank dress block.
    collar = uv_sphere("CollarCrystal", (0, -0.153, 0.795), (0.040, 0.018, 0.035), lilac, 20, 12)
    collar_l = uv_sphere("CollarWingL", (-0.052, -0.151, 0.800), (0.050, 0.014, 0.027), pale_lilac, 20, 12)
    collar_r = uv_sphere("CollarWingR", (0.052, -0.151, 0.800), (0.050, 0.014, 0.027), pale_lilac, 20, 12)
    collar_l.rotation_euler[1] = math.radians(-18)
    collar_r.rotation_euler[1] = math.radians(18)
    for obj in (collar, collar_l, collar_r):
        parent_keep_world(obj, armature, "Chest")

    # Puffy sleeves + shorter rounded limbs read better at desktop size.
    for side, sign in (("Left", 1), ("Right", -1)):
        sleeve = uv_sphere(f"{side}Sleeve", (0.205 * sign, 0.005, 0.790), (0.095, 0.082, 0.100), pale_lilac)
        parent_keep_world(sleeve, armature, f"{side}UpperArm")
        upper_arm = uv_sphere(f"{side}UpperArmMesh", (0.27 * sign, -0.005, 0.715), (0.070, 0.063, 0.145), skin)
        lower_arm = uv_sphere(f"{side}LowerArmMesh", (0.39 * sign, -0.015, 0.615), (0.060, 0.056, 0.125), skin)
        hand = uv_sphere(f"{side}HandMesh", (0.485 * sign, -0.030, 0.555), (0.072, 0.058, 0.068), skin)
        upper_leg = uv_sphere(f"{side}UpperLegMesh", (0.105 * sign, 0.015, 0.400), (0.090, 0.080, 0.155), white)
        lower_leg = uv_sphere(f"{side}LowerLegMesh", (0.105 * sign, -0.010, 0.205), (0.072, 0.064, 0.145), skin)
        foot = uv_sphere(f"{side}FootMesh", (0.105 * sign, -0.075, 0.065), (0.105, 0.130, 0.055), lilac)
        for obj, bone in (
            (upper_arm, f"{side}UpperArm"),
            (lower_arm, f"{side}LowerArm"),
            (hand, f"{side}Hand"),
            (upper_leg, f"{side}UpperLeg"),
            (lower_leg, f"{side}LowerLeg"),
            (foot, f"{side}Foot"),
        ):
            parent_keep_world(obj, armature, bone)

    # Signature butterfly bow is moved toward the visible side silhouette.
    bow_center = uv_sphere("BowCenter", (0.225, -0.035, 1.285), (0.045, 0.025, 0.045), lilac)
    bow_upper_l = uv_sphere("BowUpperLeft", (0.155, -0.030, 1.325), (0.090, 0.023, 0.070), pale_lilac)
    bow_upper_r = uv_sphere("BowUpperRight", (0.305, -0.030, 1.330), (0.105, 0.023, 0.075), pale_lilac)
    bow_lower_l = uv_sphere("BowLowerLeft", (0.165, -0.028, 1.250), (0.075, 0.021, 0.055), ice_blue)
    bow_lower_r = uv_sphere("BowLowerRight", (0.300, -0.028, 1.250), (0.085, 0.021, 0.060), ice_blue)
    bow_upper_l.rotation_euler[1] = math.radians(-25)
    bow_upper_r.rotation_euler[1] = math.radians(22)
    bow_lower_l.rotation_euler[1] = math.radians(25)
    bow_lower_r.rotation_euler[1] = math.radians(-22)
    bow_tail_l = cone("BowTailLeft", (0.190, -0.015, 1.180), 0.040, 0.018, 0.180, pale_lilac, 20)
    bow_tail_r = cone("BowTailRight", (0.270, -0.015, 1.175), 0.040, 0.018, 0.190, ice_blue, 20)
    bow_tail_l.rotation_euler[1] = math.radians(-12)
    bow_tail_r.rotation_euler[1] = math.radians(14)
    bow_crystal = uv_sphere("BowCrystal", (0.225, -0.063, 1.286), (0.025, 0.012, 0.025), white, 18, 12)
    for obj in (bow_center, bow_upper_l, bow_upper_r, bow_lower_l, bow_lower_r, bow_tail_l, bow_tail_r, bow_crystal):
        parent_keep_world(obj, armature, "Head")


def create_cat() -> bpy.types.Object:
    cream = material("CatCream", (1.0, 0.80, 0.56, 1.0), 0.78)
    white = material("CatWhite", (1.0, 0.95, 0.90, 1.0), 0.80)
    orange = material("CatOrangePatch", (0.91, 0.45, 0.19, 1.0), 0.72)
    dark = material("CatEye", (0.20, 0.13, 0.12, 1.0), 0.64)
    pink = material("CatPink", (1.0, 0.48, 0.57, 1.0), 0.74)

    root = bpy.data.objects.new("CatRoot", None)
    root.location = (0.43, -0.01, 0)
    bpy.context.collection.objects.link(root)

    body = uv_sphere("CatBody", (0.0, 0.0, 0.18), (0.16, 0.125, 0.15), cream)
    head = uv_sphere("CatHead", (0.0, -0.07, 0.34), (0.145, 0.125, 0.135), white)
    patch = uv_sphere("CatOrangePatch", (0.060, -0.183, 0.375), (0.065, 0.020, 0.060), orange, 20, 12)
    muzzle = uv_sphere("CatMuzzle", (0.0, -0.190, 0.315), (0.070, 0.028, 0.048), white, 20, 12)
    nose = uv_sphere("CatNose", (0.0, -0.220, 0.332), (0.020, 0.010, 0.015), pink, 16, 10)
    for obj in (body, head, patch, muzzle, nose):
        obj.parent = root

    for x in (-0.052, 0.052):
        eye = uv_sphere("CatEye", (x, -0.190, 0.375), (0.019, 0.010, 0.028), dark, 16, 10)
        eye.parent = root

    for x in (-0.080, 0.080):
        ear = cone("CatEar", (x, -0.050, 0.475), 0.060, 0.005, 0.135, cream, 16)
        ear.parent = root

    # Tail built from a few chunky beads for a controllable V1 silhouette.
    for i, (dx, dz) in enumerate(((0.13, 0.15), (0.20, 0.22), (0.22, 0.31))):
        tail = uv_sphere(f"CatTail{i}", (dx, 0.065, dz), (0.060, 0.052, 0.090), orange, 18, 12)
        tail.rotation_euler[1] = -0.45 + i * 0.18
        tail.parent = root

    return root


def ensure_action(armature: bpy.types.Object, name: str, end_frame: int, keys) -> None:
    if armature.animation_data is None:
        armature.animation_data_create()
    action = bpy.data.actions.new(name=name)
    armature.animation_data.action = action

    for frame, bone_values in keys:
        for bone_name, rotation, location in bone_values:
            pose = armature.pose.bones.get(bone_name)
            if pose is None:
                continue
            pose.rotation_mode = "XYZ"
            pose.rotation_euler = rotation
            pose.location = location
            pose.keyframe_insert("rotation_euler", frame=frame, group=bone_name)
            pose.keyframe_insert("location", frame=frame, group=bone_name)

    # Blender 5.x uses layered/slotted Actions and no longer exposes
    # Action.fcurves directly. keyframe_insert already creates smooth Bezier
    # interpolation by default, so there is no need to mutate F-curves here.
    action.frame_start = 1
    action.frame_end = end_frame
    armature.animation_data.action = None


def create_actions(armature: bpy.types.Object) -> None:
    zero = (0.0, 0.0, 0.0)

    ensure_action(
        armature,
        "Idle",
        90,
        [
            (1, [("Hips", zero, (0, 0, 0)), ("Chest", zero, (0, 0, 0))]),
            (45, [("Hips", zero, (0, 0, 0.010)), ("Chest", (0.018, 0, 0), (0, 0, 0))]),
            (90, [("Hips", zero, (0, 0, 0)), ("Chest", zero, (0, 0, 0))]),
        ],
    )

    walk_keys = []
    for frame, phase in ((1, 0), (7, 1), (13, 2), (19, 3), (25, 4)):
        direction = 1 if phase % 2 == 0 else -1
        walk_keys.append(
            (
                frame,
                [
                    ("Hips", zero, (0, 0, 0.012 if phase % 2 else 0)),
                    ("LeftUpperLeg", (0.34 * direction, 0, 0), (0, 0, 0)),
                    ("RightUpperLeg", (-0.34 * direction, 0, 0), (0, 0, 0)),
                    ("LeftUpperArm", (-0.24 * direction, 0, 0), (0, 0, 0)),
                    ("RightUpperArm", (0.24 * direction, 0, 0), (0, 0, 0)),
                ],
            )
        )
    ensure_action(armature, "Walk", 25, walk_keys)

    ensure_action(
        armature,
        "TurnLeft",
        20,
        [
            (1, [("Hips", zero, (0, 0, 0))]),
            (20, [("Hips", (0, 0, math.radians(90)), (0, 0, 0))]),
        ],
    )
    ensure_action(
        armature,
        "TurnRight",
        20,
        [
            (1, [("Hips", zero, (0, 0, 0))]),
            (20, [("Hips", (0, 0, math.radians(-90)), (0, 0, 0))]),
        ],
    )

    sit_pose = [
        ("Hips", (math.radians(-6), 0, 0), (0, 0, -0.22)),
        ("LeftUpperLeg", (math.radians(72), 0, 0), (0, 0, 0)),
        ("RightUpperLeg", (math.radians(72), 0, 0), (0, 0, 0)),
        ("LeftLowerLeg", (math.radians(-78), 0, 0), (0, 0, 0)),
        ("RightLowerLeg", (math.radians(-78), 0, 0), (0, 0, 0)),
    ]
    ensure_action(armature, "SitDown", 28, [(1, [("Hips", zero, (0, 0, 0))]), (28, sit_pose)])
    ensure_action(armature, "SitIdle", 90, [(1, sit_pose), (45, sit_pose), (90, sit_pose)])
    ensure_action(armature, "StandUp", 28, [(1, sit_pose), (28, [("Hips", zero, (0, 0, 0))])])

    ensure_action(
        armature,
        "Stretch",
        36,
        [
            (1, [("LeftUpperArm", zero, (0, 0, 0)), ("RightUpperArm", zero, (0, 0, 0))]),
            (
                18,
                [
                    ("LeftUpperArm", (0, math.radians(-28), math.radians(105)), (0, 0, 0)),
                    ("RightUpperArm", (0, math.radians(28), math.radians(-105)), (0, 0, 0)),
                    ("Chest", (math.radians(-8), 0, 0), (0, 0, 0)),
                ],
            ),
            (36, [("LeftUpperArm", zero, (0, 0, 0)), ("RightUpperArm", zero, (0, 0, 0))]),
        ],
    )

    ensure_action(
        armature,
        "LookAtCat",
        42,
        [
            (1, [("Head", zero, (0, 0, 0))]),
            (20, [("Head", (math.radians(8), 0, math.radians(-16)), (0, 0, 0))]),
            (42, [("Head", zero, (0, 0, 0))]),
        ],
    )

    ensure_action(
        armature,
        "Daydream",
        80,
        [
            (1, [("Head", zero, (0, 0, 0)), ("Chest", zero, (0, 0, 0))]),
            (40, [("Head", (math.radians(5), 0, math.radians(6)), (0, 0, 0)), ("Chest", (math.radians(3), 0, 0), (0, 0, 0))]),
            (80, [("Head", zero, (0, 0, 0)), ("Chest", zero, (0, 0, 0))]),
        ],
    )

    # Carry clips establish arm positioning and named clips; runtime handles CatRoot attachment.
    carry_pose = [
        ("LeftUpperArm", (math.radians(38), math.radians(-18), math.radians(26)), (0, 0, 0)),
        ("RightUpperArm", (math.radians(38), math.radians(18), math.radians(-26)), (0, 0, 0)),
        ("LeftLowerArm", (math.radians(-48), 0, 0), (0, 0, 0)),
        ("RightLowerArm", (math.radians(-48), 0, 0), (0, 0, 0)),
    ]
    ensure_action(armature, "CarryCatPickup", 34, [(1, []), (34, carry_pose)])
    ensure_action(armature, "CarryCatIdle", 90, [(1, carry_pose), (45, carry_pose), (90, carry_pose)])
    ensure_action(
        armature,
        "CarryCatWalk",
        25,
        [
            (1, carry_pose),
            (13, carry_pose + [("Hips", zero, (0, 0, 0.012))]),
            (25, carry_pose),
        ],
    )
    ensure_action(armature, "CarryCatPutdown", 34, [(1, carry_pose), (34, [])])
    ensure_action(armature, "Petting", 34, [(1, []), (17, [("Head", (math.radians(4), 0, 0), (0, 0, -0.015))]), (34, [])])


def create_preview_camera_and_light() -> None:
    bpy.ops.object.light_add(type="AREA", location=(2.5, -3.0, 3.2))
    key = bpy.context.object
    key.name = "PreviewKey"
    key.data.energy = 500
    key.data.shape = "DISK"
    key.data.size = 3.0
    key.rotation_euler = (math.radians(28), 0, math.radians(34))

    bpy.ops.object.camera_add(location=(0.08, -4.4, 1.0))
    camera = bpy.context.object
    camera.name = "PreviewCamera"
    camera.data.type = "ORTHO"
    camera.data.ortho_scale = 1.72
    camera.rotation_euler = (math.radians(90), 0, 0)
    # Track towards the character instead of relying on hand-tuned Euler values.
    direction = Vector((0.06, 0, 0.70)) - camera.location
    camera.rotation_euler = direction.to_track_quat("-Z", "Y").to_euler()
    bpy.context.scene.camera = camera


def configure_scene() -> None:
    scene = bpy.context.scene
    # Blender 5.2 exposes Eevee as BLENDER_EEVEE; Blender 4.x used
    # BLENDER_EEVEE_NEXT. Probe the enum so the generator works across both.
    engine_items = scene.bl_rna.properties["render"].fixed_type.properties["engine"].enum_items
    engine_ids = {item.identifier for item in engine_items}
    scene.render.engine = "BLENDER_EEVEE" if "BLENDER_EEVEE" in engine_ids else "BLENDER_EEVEE_NEXT"
    scene.render.film_transparent = True
    scene.render.resolution_x = 720
    scene.render.resolution_y = 720
    scene.render.resolution_percentage = 100
    scene.render.image_settings.file_format = "PNG"
    scene.render.fps = 30
    scene.unit_settings.system = "METRIC"
    scene.unit_settings.scale_length = 1.0
    scene.world.color = (0.035, 0.025, 0.055)


def export(output: Path) -> None:
    output.mkdir(parents=True, exist_ok=True)
    blend_path = output / "sena_v1.blend"
    glb_path = output / "sena_v1.glb"

    bpy.ops.wm.save_as_mainfile(filepath=str(blend_path.resolve()))

    bpy.ops.object.select_all(action="SELECT")
    bpy.ops.export_scene.gltf(
        filepath=str(glb_path.resolve()),
        export_format="GLB",
        export_animations=True,
        export_skins=True,
        export_morph=True,
    )
    print(f"[Sena] wrote {blend_path}")
    print(f"[Sena] wrote {glb_path}")


def main() -> None:
    args = cli_args()
    output = Path(args.output)

    reset_scene()
    configure_scene()
    armature = create_armature()
    create_sena(armature)
    create_cat()
    create_actions(armature)
    create_preview_camera_and_light()
    export(output)

    print(f"[Sena] procedural chibi V1 generated at height target {MODEL_HEIGHT:.2f}m")


if __name__ == "__main__":
    main()
