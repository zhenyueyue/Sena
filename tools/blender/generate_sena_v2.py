"""Generate Sena V2 with a purpose-built anime head, flat eyes and hair ribbons.

V1 stays intact as the technical/runtime blockout. V2 reuses the proven rig,
body, cat, actions and attachment contract, but replaces the weakest visual
area first: head, face, eyes and hair silhouette.

Run:
    blender --background --python tools/blender/generate_sena_v2.py -- --output pets/sena/models/generated
"""

from __future__ import annotations

import argparse
import math
import sys
from pathlib import Path

import bpy
from mathutils import Vector

SCRIPT_DIR = Path(__file__).resolve().parent
if str(SCRIPT_DIR) not in sys.path:
    sys.path.insert(0, str(SCRIPT_DIR))

import generate_sena_v1 as base  # noqa: E402


def cli_args() -> argparse.Namespace:
    argv = sys.argv
    argv = argv[argv.index("--") + 1 :] if "--" in argv else []
    parser = argparse.ArgumentParser()
    parser.add_argument(
        "--output",
        default="pets/sena/models/generated",
        help="Directory for sena_v2.blend and sena_v2.glb",
    )
    return parser.parse_args(argv)


def remove_object(obj: bpy.types.Object) -> None:
    data = obj.data
    object_type = obj.type
    bpy.data.objects.remove(obj, do_unlink=True)
    if data is None:
        return
    collections = {
        "MESH": bpy.data.meshes,
        "CURVE": bpy.data.curves,
    }
    collection = collections.get(object_type)
    if collection is not None and data.users == 0:
        collection.remove(data)


def remove_v1_head_parts() -> None:
    exact = {
        "HeadMesh",
        "BackHair",
        "Nose",
        "Mouth",
        "FaceBlushL",
        "FaceBlushR",
    }
    prefixes = (
        "HairBackLock",
        "HairSide",
        "Bang",
        "EyeWhite",
        "Iris",
        "Pupil",
        "EyeHighlight",
        "UpperLash",
        "Brow",
        "Bow",
    )
    for obj in list(bpy.data.objects):
        if obj.name in exact or obj.name.startswith(prefixes):
            remove_object(obj)


def apply_modifier(obj: bpy.types.Object, modifier_name: str) -> None:
    bpy.ops.object.select_all(action="DESELECT")
    obj.select_set(True)
    bpy.context.view_layer.objects.active = obj
    bpy.ops.object.modifier_apply(modifier=modifier_name)
    obj.select_set(False)


def anime_head_mesh(name: str, mat) -> bpy.types.Object:
    """Create a ring-based chibi anime head instead of a scaled UV sphere.

    Front is -Y. The lower rings narrow strongly into a small chin while the
    eye/temple region stays broad, which reads much closer to a game chibi face
    from both front and 3/4 views.
    """

    segments = 40
    # z, x radius, front depth, back depth, y center
    rings = [
        (0.945, 0.070, 0.105, 0.115, 0.000),
        (0.975, 0.145, 0.145, 0.160, 0.000),
        (1.015, 0.205, 0.185, 0.205, 0.000),
        (1.070, 0.245, 0.205, 0.225, 0.000),
        (1.135, 0.265, 0.212, 0.235, 0.002),
        (1.205, 0.262, 0.205, 0.238, 0.008),
        (1.265, 0.235, 0.185, 0.220, 0.012),
        (1.310, 0.175, 0.150, 0.185, 0.015),
    ]

    vertices: list[tuple[float, float, float]] = []
    faces: list[tuple[int, ...]] = []

    for z, rx, front, back, center_y in rings:
        for segment in range(segments):
            angle = math.tau * segment / segments
            sin_a = math.sin(angle)
            depth = front if sin_a < 0 else back
            # Slight temple flattening near the front keeps the face from
            # reading as a balloon while preserving a rounded skull.
            front_flatten = 1.0 - 0.035 * max(0.0, -sin_a)
            x = rx * math.cos(angle) * front_flatten
            y = center_y + depth * sin_a
            vertices.append((x, y, z))

    for ring in range(len(rings) - 1):
        start = ring * segments
        next_start = (ring + 1) * segments
        for segment in range(segments):
            nxt = (segment + 1) % segments
            faces.append(
                (
                    start + segment,
                    start + nxt,
                    next_start + nxt,
                    next_start + segment,
                )
            )

    bottom = len(vertices)
    vertices.append((0.0, 0.005, 0.925))
    top = len(vertices)
    vertices.append((0.0, 0.020, 1.340))

    for segment in range(segments):
        nxt = (segment + 1) % segments
        faces.append((bottom, nxt, segment))
        top_ring = (len(rings) - 1) * segments
        faces.append((top, top_ring + segment, top_ring + nxt))

    mesh = bpy.data.meshes.new(name)
    mesh.from_pydata(vertices, [], faces)
    mesh.update()
    obj = bpy.data.objects.new(name, mesh)
    bpy.context.collection.objects.link(obj)
    base.assign_material(obj, mat)
    base.smooth(obj)

    subdivision = obj.modifiers.new("FaceSubdivision", "SUBSURF")
    subdivision.subdivision_type = "CATMULL_CLARK"
    subdivision.levels = 1
    subdivision.render_levels = 1
    apply_modifier(obj, subdivision.name)
    return obj


def face_disc(
    name: str,
    center: tuple[float, float, float],
    radius_x: float,
    radius_z: float,
    mat,
    segments: int = 32,
    curvature: float = 0.004,
) -> bpy.types.Object:
    """Create an almost-flat curved facial patch with normal facing -Y."""

    cx, cy, cz = center
    vertices = [(cx, cy - curvature, cz)]
    for index in range(segments):
        angle = math.tau * index / segments
        dx = radius_x * math.cos(angle)
        dz = radius_z * math.sin(angle)
        normalized = (dx / radius_x) ** 2 + (dz / radius_z) ** 2
        y = cy + curvature * min(1.0, normalized)
        vertices.append((cx + dx, y, cz + dz))

    faces = []
    for index in range(segments):
        current = index + 1
        nxt = ((index + 1) % segments) + 1
        # Winding points the patch toward the -Y desktop camera.
        faces.append((0, current, nxt))

    mesh = bpy.data.meshes.new(name)
    mesh.from_pydata(vertices, [], faces)
    mesh.update()
    obj = bpy.data.objects.new(name, mesh)
    bpy.context.collection.objects.link(obj)
    base.assign_material(obj, mat)
    base.smooth(obj)
    return obj


def hair_ribbon(
    name: str,
    points: list[tuple[float, float, float]],
    widths: list[float],
    mat,
    *,
    thickness: float = 0.010,
    bevel: float = 0.004,
) -> bpy.types.Object:
    """Create a tapered hair card following an X/Z curve.

    Unlike the old capsule/cone hair, this has a broad root, curved body and
    real pointed tip. Solidify/bevel give just enough volume for 3/4 views.
    """

    if len(points) != len(widths) or len(points) < 2:
        raise ValueError("hair ribbon points/widths mismatch")

    vertices: list[tuple[float, float, float]] = []
    for index, (point_tuple, width) in enumerate(zip(points, widths)):
        point = Vector(point_tuple)
        width = max(width, 0.004)
        if index == 0:
            tangent = Vector(points[1]) - point
        elif index == len(points) - 1:
            tangent = point - Vector(points[index - 1])
        else:
            tangent = Vector(points[index + 1]) - Vector(points[index - 1])
        tangent.y = 0.0
        if tangent.length_squared == 0:
            tangent = Vector((0.0, 0.0, -1.0))
        tangent.normalize()
        side = Vector((-tangent.z, 0.0, tangent.x)).normalized() * width
        vertices.append(tuple(point - side))
        vertices.append(tuple(point + side))

    faces = []
    for index in range(len(points) - 1):
        a = index * 2
        b = a + 1
        c = a + 3
        d = a + 2
        faces.append((a, b, c, d))

    mesh = bpy.data.meshes.new(name)
    mesh.from_pydata(vertices, [], faces)
    mesh.update()
    obj = bpy.data.objects.new(name, mesh)
    bpy.context.collection.objects.link(obj)
    base.assign_material(obj, mat)
    base.smooth(obj)

    solidify = obj.modifiers.new("HairThickness", "SOLIDIFY")
    solidify.thickness = thickness
    solidify.offset = 0.0
    apply_modifier(obj, solidify.name)

    edge = obj.modifiers.new("HairSoftEdge", "BEVEL")
    edge.width = bevel
    edge.segments = 2
    apply_modifier(obj, edge.name)

    return obj


def shape_prism(
    name: str,
    outline: list[tuple[float, float]],
    y: float,
    thickness: float,
    mat,
) -> bpy.types.Object:
    """Create a beveled X/Z silhouette prism for bows and flat ornaments."""

    front_y = y - thickness * 0.5
    back_y = y + thickness * 0.5
    vertices = [(x, front_y, z) for x, z in outline]
    vertices += [(x, back_y, z) for x, z in outline]
    count = len(outline)

    # Front face points toward -Y.
    faces: list[tuple[int, ...]] = [tuple(range(count - 1, -1, -1))]
    faces.append(tuple(range(count, count * 2)))
    for index in range(count):
        nxt = (index + 1) % count
        faces.append((index, nxt, count + nxt, count + index))

    mesh = bpy.data.meshes.new(name)
    mesh.from_pydata(vertices, [], faces)
    mesh.update()
    obj = bpy.data.objects.new(name, mesh)
    bpy.context.collection.objects.link(obj)
    base.assign_material(obj, mat)
    base.smooth(obj)

    bevel = obj.modifiers.new("ShapeSoftEdge", "BEVEL")
    bevel.width = 0.006
    bevel.segments = 3
    apply_modifier(obj, bevel.name)
    return obj


def create_butterfly_bow(armature: bpy.types.Object, lilac, pale_lilac, ice_blue, white) -> None:
    """Build Sena's signature head bow from readable butterfly silhouettes."""

    # Coordinates are world-space so the bow can be reviewed independently of
    # the old V1 ellipsoid construction. The center sits at the right temple.
    center_x = 0.238
    center_z = 1.286

    upper_left = shape_prism(
        "BowV2UpperInner",
        [
            (center_x - 0.018, center_z + 0.008),
            (center_x - 0.102, center_z + 0.088),
            (center_x - 0.135, center_z + 0.060),
            (center_x - 0.110, center_z - 0.010),
            (center_x - 0.035, center_z - 0.018),
        ],
        -0.005,
        0.026,
        pale_lilac,
    )
    upper_right = shape_prism(
        "BowV2UpperOuter",
        [
            (center_x + 0.018, center_z + 0.010),
            (center_x + 0.105, center_z + 0.105),
            (center_x + 0.170, center_z + 0.080),
            (center_x + 0.145, center_z - 0.002),
            (center_x + 0.040, center_z - 0.020),
        ],
        -0.004,
        0.028,
        pale_lilac,
    )
    lower_left = shape_prism(
        "BowV2LowerInner",
        [
            (center_x - 0.020, center_z - 0.005),
            (center_x - 0.095, center_z - 0.030),
            (center_x - 0.105, center_z - 0.090),
            (center_x - 0.040, center_z - 0.075),
            (center_x + 0.005, center_z - 0.025),
        ],
        -0.003,
        0.024,
        ice_blue,
    )
    lower_right = shape_prism(
        "BowV2LowerOuter",
        [
            (center_x + 0.018, center_z - 0.008),
            (center_x + 0.115, center_z - 0.030),
            (center_x + 0.130, center_z - 0.105),
            (center_x + 0.055, center_z - 0.085),
            (center_x - 0.002, center_z - 0.025),
        ],
        -0.002,
        0.025,
        ice_blue,
    )

    knot = base.uv_sphere(
        "BowV2CrystalKnot",
        (center_x, -0.022, center_z),
        (0.036, 0.020, 0.036),
        lilac,
        24,
        16,
    )
    crystal = base.uv_sphere(
        "BowV2Highlight",
        (center_x - 0.009, -0.043, center_z + 0.011),
        (0.010, 0.005, 0.013),
        white,
        16,
        10,
    )

    tail_left = hair_ribbon(
        "BowV2TailL",
        [
            (center_x - 0.020, 0.005, center_z - 0.030),
            (center_x - 0.045, 0.008, center_z - 0.095),
            (center_x - 0.065, 0.010, center_z - 0.155),
        ],
        [0.018, 0.025, 0.002],
        pale_lilac,
        thickness=0.010,
        bevel=0.003,
    )
    tail_right = hair_ribbon(
        "BowV2TailR",
        [
            (center_x + 0.020, 0.006, center_z - 0.028),
            (center_x + 0.055, 0.010, center_z - 0.100),
            (center_x + 0.085, 0.012, center_z - 0.165),
        ],
        [0.018, 0.026, 0.002],
        ice_blue,
        thickness=0.010,
        bevel=0.003,
    )

    for obj in (upper_left, upper_right, lower_left, lower_right, knot, crystal):
        base.parent_keep_world(obj, armature, "BowRoot")
    base.parent_keep_world(tail_left, armature, "BowTailL")
    base.parent_keep_world(tail_right, armature, "BowTailR")


def hair_cap(mat) -> bpy.types.Object:
    """Create a custom upper/back scalp shell with a large face opening."""

    segments = 40
    # z, rx, front depth, back depth
    rings = [
        (1.085, 0.270, 0.202, 0.245),
        (1.160, 0.282, 0.210, 0.255),
        (1.230, 0.270, 0.205, 0.250),
        (1.290, 0.225, 0.180, 0.220),
        (1.330, 0.145, 0.125, 0.170),
    ]

    vertices: list[tuple[float, float, float]] = []
    valid: list[list[bool]] = []
    for ring_index, (z, rx, front, back) in enumerate(rings):
        row = []
        for segment in range(segments):
            angle = math.tau * segment / segments
            sin_a = math.sin(angle)
            depth = front if sin_a < 0 else back
            x = rx * math.cos(angle)
            y = 0.018 + depth * sin_a
            vertices.append((x, y, z))

            degrees = math.degrees(math.atan2(sin_a, math.cos(angle)))
            # Front sector around -90 degrees is opened below the crown so the
            # face/eyes are not hidden by a helmet-shaped shell.
            front_open = -150.0 < degrees < -30.0 and ring_index < 3
            row.append(not front_open)
        valid.append(row)

    faces: list[tuple[int, ...]] = []
    for ring in range(len(rings) - 1):
        for segment in range(segments):
            nxt = (segment + 1) % segments
            if not (
                valid[ring][segment]
                and valid[ring][nxt]
                and valid[ring + 1][segment]
                and valid[ring + 1][nxt]
            ):
                continue
            a = ring * segments + segment
            b = ring * segments + nxt
            c = (ring + 1) * segments + nxt
            d = (ring + 1) * segments + segment
            faces.append((a, b, c, d))

    top = len(vertices)
    vertices.append((0.0, 0.025, 1.350))
    top_ring = (len(rings) - 1) * segments
    for segment in range(segments):
        nxt = (segment + 1) % segments
        faces.append((top, top_ring + segment, top_ring + nxt))

    mesh = bpy.data.meshes.new("HairCapV2")
    mesh.from_pydata(vertices, [], faces)
    mesh.update()
    obj = bpy.data.objects.new("HairCapV2", mesh)
    bpy.context.collection.objects.link(obj)
    base.assign_material(obj, mat)
    base.smooth(obj)

    subdivision = obj.modifiers.new("HairCapSubdivision", "SUBSURF")
    subdivision.levels = 1
    subdivision.render_levels = 1
    apply_modifier(obj, subdivision.name)
    return obj


def create_v2_head(armature: bpy.types.Object) -> None:
    skin = bpy.data.materials["Skin"]
    hair = bpy.data.materials["MoonlightHair"]
    hair_shadow = bpy.data.materials["MoonlightHairShadow"]
    eye_white = bpy.data.materials["EyeWhite"]
    iris = bpy.data.materials["VioletPinkEye"]
    dark = bpy.data.materials["LashDark"]
    blush = bpy.data.materials["Blush"]
    white = bpy.data.materials["PearlWhite"]
    lilac = bpy.data.materials["CrystalLilac"]
    pale_lilac = bpy.data.materials["PaleLilac"]
    ice_blue = bpy.data.materials["IceBlue"]

    head = anime_head_mesh("HeadMeshV2", skin)
    base.parent_keep_world(head, armature, "Head")

    cap = hair_cap(hair)
    base.parent_keep_world(cap, armature, "Head")

    # Back locks: broad flowing ribbons, not capsule stacks.
    back_lock_specs = [
        (
            "HairBackOuterL",
            [(-0.205, 0.125, 1.235), (-0.245, 0.145, 1.075), (-0.255, 0.150, 0.900), (-0.235, 0.145, 0.720), (-0.205, 0.135, 0.595)],
            [0.070, 0.085, 0.090, 0.070, 0.012],
            "HairBackL",
        ),
        (
            "HairBackInnerL",
            [(-0.105, 0.150, 1.285), (-0.130, 0.165, 1.080), (-0.145, 0.170, 0.880), (-0.125, 0.165, 0.675), (-0.085, 0.150, 0.555)],
            [0.075, 0.095, 0.100, 0.075, 0.010],
            "HairBackL",
        ),
        (
            "HairBackCenter",
            [(0.000, 0.165, 1.305), (0.000, 0.180, 1.080), (0.010, 0.185, 0.855), (-0.010, 0.180, 0.635), (0.000, 0.165, 0.520)],
            [0.085, 0.110, 0.115, 0.085, 0.010],
            "HairBackC",
        ),
        (
            "HairBackInnerR",
            [(0.105, 0.150, 1.285), (0.130, 0.165, 1.080), (0.145, 0.170, 0.880), (0.125, 0.165, 0.675), (0.085, 0.150, 0.555)],
            [0.075, 0.095, 0.100, 0.075, 0.010],
            "HairBackR",
        ),
        (
            "HairBackOuterR",
            [(0.205, 0.125, 1.235), (0.245, 0.145, 1.075), (0.255, 0.150, 0.900), (0.235, 0.145, 0.720), (0.205, 0.135, 0.595)],
            [0.070, 0.085, 0.090, 0.070, 0.012],
            "HairBackR",
        ),
    ]
    for name, points, widths, bone in back_lock_specs:
        lock = hair_ribbon(name, points, widths, hair_shadow, thickness=0.014, bevel=0.005)
        base.parent_keep_world(lock, armature, bone)

    # Face-framing locks curve around the cheeks before tapering below the jaw.
    side_specs = [
        (
            "HairSideV2L",
            [(-0.225, -0.055, 1.235), (-0.255, -0.120, 1.145), (-0.248, -0.145, 1.025), (-0.225, -0.130, 0.885), (-0.195, -0.105, 0.750)],
            [0.052, 0.060, 0.055, 0.040, 0.008],
            "HairSideL",
        ),
        (
            "HairSideV2R",
            [(0.225, -0.055, 1.235), (0.255, -0.120, 1.145), (0.248, -0.145, 1.025), (0.225, -0.130, 0.885), (0.195, -0.105, 0.750)],
            [0.052, 0.060, 0.055, 0.040, 0.008],
            "HairSideR",
        ),
    ]
    for name, points, widths, bone in side_specs:
        lock = hair_ribbon(name, points, widths, hair, thickness=0.012, bevel=0.004)
        base.parent_keep_world(lock, armature, bone)

    # Airy bangs leave a visible forehead gap and taper to real points.
    bang_specs = [
        (
            "BangV2OuterL",
            [(-0.215, -0.130, 1.300), (-0.202, -0.170, 1.272), (-0.178, -0.205, 1.225), (-0.150, -0.222, 1.165), (-0.132, -0.220, 1.118)],
            [0.022, 0.048, 0.055, 0.035, 0.0015],
        ),
        (
            "BangV2InnerL",
            [(-0.115, -0.155, 1.320), (-0.105, -0.195, 1.292), (-0.088, -0.221, 1.250), (-0.070, -0.232, 1.198), (-0.055, -0.226, 1.153)],
            [0.020, 0.044, 0.050, 0.032, 0.0015],
        ),
        (
            "BangV2Center",
            [(-0.012, -0.168, 1.330), (-0.004, -0.205, 1.305), (0.004, -0.228, 1.268), (0.012, -0.235, 1.226), (0.020, -0.226, 1.184)],
            [0.018, 0.038, 0.042, 0.026, 0.0012],
        ),
        (
            "BangV2InnerR",
            [(0.105, -0.155, 1.320), (0.100, -0.195, 1.292), (0.085, -0.221, 1.250), (0.068, -0.232, 1.198), (0.054, -0.226, 1.153)],
            [0.020, 0.044, 0.050, 0.032, 0.0015],
        ),
        (
            "BangV2OuterR",
            [(0.210, -0.130, 1.300), (0.198, -0.170, 1.272), (0.176, -0.205, 1.225), (0.150, -0.222, 1.165), (0.132, -0.220, 1.118)],
            [0.022, 0.048, 0.055, 0.035, 0.0015],
        ),
    ]
    for name, points, widths in bang_specs:
        bang = hair_ribbon(name, points, widths, hair, thickness=0.009, bevel=0.003)
        base.parent_keep_world(bang, armature, "Head")

    create_butterfly_bow(armature, lilac, pale_lilac, ice_blue, white)

    # Flat layered anime eyes: no protruding eyeballs in side view.
    eye_centers = [("L", -0.092), ("R", 0.092)]
    for suffix, x in eye_centers:
        eye = face_disc(f"EyeWhiteV2{suffix}", (x, -0.2115, 1.135), 0.064, 0.080, eye_white, curvature=0.0025)
        iris_mesh = face_disc(f"IrisV2{suffix}", (x, -0.2155, 1.133), 0.037, 0.055, iris, curvature=0.0015)
        pupil = face_disc(f"PupilV2{suffix}", (x, -0.2180, 1.132), 0.014, 0.030, dark, curvature=0.0008)
        highlight_big = face_disc(f"EyeHighlightBigV2{suffix}", (x - 0.012, -0.2200, 1.157), 0.011, 0.015, white, 20, 0.0002)
        highlight_small = face_disc(f"EyeHighlightSmallV2{suffix}", (x + 0.013, -0.2202, 1.135), 0.0055, 0.0075, white, 16, 0.0001)
        for obj in (eye, iris_mesh, pupil, highlight_big, highlight_small):
            base.parent_keep_world(obj, armature, "Head")

        lash_points = (
            [(-0.153, -0.222, 1.174), (-0.118, -0.226, 1.188), (-0.074, -0.226, 1.190), (-0.031, -0.220, 1.171)]
            if suffix == "L"
            else [(0.031, -0.220, 1.171), (0.074, -0.226, 1.190), (0.118, -0.226, 1.188), (0.153, -0.222, 1.174)]
        )
        lash = base.curve_tube(f"UpperLashV2{suffix}", lash_points, 0.0052, dark)
        base.parent_keep_world(lash, armature, "Head")

        outer_x = -0.160 if suffix == "L" else 0.160
        lash_tip = hair_ribbon(
            f"LashTipV2{suffix}",
            [(outer_x, -0.220, 1.178), (outer_x + (-0.030 if suffix == "L" else 0.030), -0.218, 1.188)],
            [0.010, 0.002],
            dark,
            thickness=0.004,
            bevel=0.001,
        )
        base.parent_keep_world(lash_tip, armature, "Head")

        brow_points = (
            [(-0.145, -0.205, 1.225), (-0.102, -0.212, 1.238), (-0.055, -0.207, 1.228)]
            if suffix == "L"
            else [(0.055, -0.207, 1.228), (0.102, -0.212, 1.238), (0.145, -0.205, 1.225)]
        )
        brow = base.curve_tube(f"BrowV2{suffix}", brow_points, 0.0038, hair_shadow)
        base.parent_keep_world(brow, armature, "Head")

    # Small anime nose/mouth sit almost flush to the face.
    nose = face_disc("NoseV2", (0.0, -0.2130, 1.075), 0.008, 0.010, blush, 16, 0.0002)
    base.parent_keep_world(nose, armature, "Head")
    mouth = base.curve_tube(
        "MouthV2",
        [(-0.032, -0.215, 1.035), (0.0, -0.219, 1.027), (0.032, -0.215, 1.035)],
        0.0042,
        blush,
    )
    base.parent_keep_world(mouth, armature, "Head")

    cheek_blush = base.material("FaceBlushV2", (1.0, 0.58, 0.68, 1.0), 0.82)
    for suffix, x in eye_centers:
        blush_patch = face_disc(
            f"CheekBlushV2{suffix}",
            (x * 1.65, -0.2075, 1.070),
            0.036,
            0.012,
            cheek_blush,
            24,
            0.0002,
        )
        base.parent_keep_world(blush_patch, armature, "Head")


def export_v2(output: Path) -> None:
    output.mkdir(parents=True, exist_ok=True)
    blend_path = output / "sena_v2.blend"
    glb_path = output / "sena_v2.glb"

    bpy.ops.wm.save_as_mainfile(filepath=str(blend_path.resolve()))
    bpy.ops.object.select_all(action="SELECT")
    bpy.ops.export_scene.gltf(
        filepath=str(glb_path.resolve()),
        export_format="GLB",
        export_animations=True,
        export_skins=True,
        export_morph=True,
    )
    print(f"[Sena V2] wrote {blend_path}")
    print(f"[Sena V2] wrote {glb_path}")


def main() -> None:
    args = cli_args()
    output = Path(args.output)

    base.reset_scene()
    base.configure_scene()
    armature = base.create_armature()

    # Reuse the proven V1 body/costume/rig contract for now, then replace only
    # the head system. This keeps the visual experiment isolated and reversible.
    base.create_sena(armature)
    remove_v1_head_parts()
    create_v2_head(armature)

    base.create_cat()
    base.create_actions(armature)
    base.create_preview_camera_and_light()
    export_v2(output)

    print("[Sena V2] anime head/face/hair prototype generated")


if __name__ == "__main__":
    main()
