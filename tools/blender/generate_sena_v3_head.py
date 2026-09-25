"""Build the Sena V3 head-approval model.

This file intentionally does NOT build the full character.  The only goal is
to approve Sena's face/head/hair/bow from front, three-quarter and side views
before any more body/costume/runtime work is allowed.

V1/V2 remain technical references.  V3 uses a new parametric anime head surface
with explicit face-profile shaping, custom almond eyes and layered hair cards.
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
import generate_sena_v2 as v2  # noqa: E402


def cli_args() -> argparse.Namespace:
    argv = sys.argv
    argv = argv[argv.index("--") + 1 :] if "--" in argv else []
    parser = argparse.ArgumentParser()
    parser.add_argument(
        "--output",
        default="pets/sena/models/generated",
        help="Directory for sena_v3_head.blend/glb",
    )
    return parser.parse_args(argv)


def make_materials() -> dict[str, bpy.types.Material]:
    mats = {
        "skin": base.material("V3Skin", (1.0, 0.83, 0.88, 1.0), 0.80),
        "hair": base.material("V3MoonlightHair", (0.86, 0.82, 0.96, 1.0), 0.66),
        "hair_shadow": base.material("V3HairShadow", (0.68, 0.59, 0.86, 1.0), 0.70),
        "eye_white": base.material("V3EyeWhite", (1.0, 0.995, 1.0, 1.0), 0.70),
        "iris_outer": base.material("V3IrisOuter", (0.43, 0.10, 0.68, 1.0), 0.42),
        "iris_inner": base.material("V3IrisInner", (0.95, 0.31, 0.78, 1.0), 0.38),
        "pupil": base.material("V3Pupil", (0.12, 0.055, 0.19, 1.0), 0.60),
        "lash": base.material("V3Lash", (0.20, 0.11, 0.27, 1.0), 0.64),
        "blush": base.material("V3Blush", (1.0, 0.50, 0.66, 1.0), 0.82),
        "lilac": base.material("V3CrystalLilac", (0.68, 0.48, 0.98, 1.0), 0.40),
        "pale_lilac": base.material("V3PaleLilac", (0.86, 0.75, 1.0, 0.88), 0.48),
        "ice_blue": base.material("V3IceBlue", (0.67, 0.87, 1.0, 0.84), 0.48),
        "white": base.material("V3PearlWhite", (0.985, 0.985, 1.0, 1.0), 0.70),
    }

    for key in ("pale_lilac", "ice_blue"):
        mat = mats[key]
        try:
            mat.surface_render_method = "DITHERED"
        except (AttributeError, TypeError):
            pass
    return mats


def create_anime_head(mat: bpy.types.Material) -> bpy.types.Object:
    """Create a smooth anime head with a deliberately designed side profile."""

    segments = 56
    # z, x radius, front depth, back depth, y-center
    rings = [
        (0.925, 0.025, 0.066, 0.080, 0.002),
        (0.945, 0.075, 0.090, 0.105, 0.000),
        (0.975, 0.125, 0.122, 0.142, -0.002),
        (1.015, 0.176, 0.158, 0.182, -0.002),
        (1.065, 0.218, 0.184, 0.208, 0.001),
        (1.120, 0.255, 0.202, 0.228, 0.006),
        (1.180, 0.268, 0.207, 0.238, 0.011),
        (1.240, 0.265, 0.198, 0.242, 0.016),
        (1.295, 0.238, 0.178, 0.223, 0.020),
        (1.335, 0.185, 0.145, 0.190, 0.022),
        (1.365, 0.105, 0.100, 0.125, 0.023),
        (1.380, 0.025, 0.045, 0.055, 0.024),
    ]

    vertices: list[tuple[float, float, float]] = []
    faces: list[tuple[int, ...]] = []

    for z, rx, front, back, center_y in rings:
        for i in range(segments):
            angle = math.tau * i / segments
            sin_a = math.sin(angle)
            cos_a = math.cos(angle)
            depth = front if sin_a < 0.0 else back

            x = rx * cos_a
            y = center_y + depth * sin_a

            # Flatten the central face plane slightly while keeping rounded
            # cheeks.  This prevents the "ball with stickers" look.
            if sin_a < -0.55:
                central = math.exp(-((x / 0.185) ** 4))
                y += 0.010 * central

            # Subtle integrated nose bridge/tip; intentionally tiny for chibi.
            nose = math.exp(-((x / 0.045) ** 2) - (((z - 1.080) / 0.030) ** 2))
            y -= 0.014 * nose

            # Pull the lower side cheeks inward toward the chin.
            if z < 1.045:
                y += 0.006 * (abs(x) / max(rx, 0.001))

            vertices.append((x, y, z))

    for ring in range(len(rings) - 1):
        a0 = ring * segments
        b0 = (ring + 1) * segments
        for i in range(segments):
            j = (i + 1) % segments
            faces.append((a0 + i, a0 + j, b0 + j, b0 + i))

    mesh = bpy.data.meshes.new("SenaV3HeadMesh")
    mesh.from_pydata(vertices, [], faces)
    mesh.update()
    obj = bpy.data.objects.new("SenaV3Head", mesh)
    bpy.context.collection.objects.link(obj)
    base.assign_material(obj, mat)
    base.smooth(obj)

    sub = obj.modifiers.new("SenaV3HeadSubdiv", "SUBSURF")
    sub.subdivision_type = "CATMULL_CLARK"
    sub.levels = 1
    sub.render_levels = 1
    v2.apply_modifier(obj, sub.name)
    return obj


def create_ear(name: str, x: float, mat: bpy.types.Material) -> bpy.types.Object:
    obj = base.uv_sphere(name, (x, -0.006, 1.105), (0.035, 0.020, 0.052), mat, 24, 16)
    return obj


def eye_patch(
    name: str,
    center: tuple[float, float, float],
    width: float,
    height: float,
    mat: bpy.types.Material,
    *,
    upper_bias: float = 0.0,
) -> bpy.types.Object:
    """Create a smooth anime eye patch with an almond/rounded lower contour."""

    cx, cy, cz = center
    outline: list[tuple[float, float, float]] = []
    count = 40

    for i in range(count):
        t = math.tau * i / count
        x = math.cos(t)
        s = math.sin(t)

        # Upper lid is flatter, lower lid is rounder.
        if s >= 0:
            z_scale = 0.72 + upper_bias
        else:
            z_scale = 1.00
        px = cx + x * width
        pz = cz + s * height * z_scale

        # Wrap over the curved face: the center sits furthest forward while
        # inner/outer edges recede, so the eye keeps a readable profile view.
        radial = x * x + s * s
        dx = px - cx
        wrap = 0.115 * (dx * dx / max(width, 1e-5))
        py = cy - 0.0015 + 0.0020 * radial + wrap
        outline.append((px, py, pz))

    vertices = [(cx, cy - 0.003, cz), *outline]
    faces = []
    for i in range(count):
        a = i + 1
        b = ((i + 1) % count) + 1
        faces.append((0, a, b))

    mesh = bpy.data.meshes.new(name)
    mesh.from_pydata(vertices, [], faces)
    mesh.update()
    obj = bpy.data.objects.new(name, mesh)
    bpy.context.collection.objects.link(obj)
    base.assign_material(obj, mat)
    base.smooth(obj)
    return obj


def create_eye(
    suffix: str,
    x: float,
    armature: bpy.types.Object,
    mats: dict[str, bpy.types.Material],
) -> None:
    cy = -0.197
    cz = 1.136

    eye = eye_patch(f"V3EyeWhite{suffix}", (x, cy, cz), 0.078, 0.060, mats["eye_white"])
    iris_outer = eye_patch(
        f"V3IrisOuter{suffix}",
        (x, cy - 0.004, cz - 0.002),
        0.040,
        0.050,
        mats["iris_outer"],
    )
    iris_inner = eye_patch(
        f"V3IrisInner{suffix}",
        (x, cy - 0.006, cz - 0.006),
        0.029,
        0.038,
        mats["iris_inner"],
    )
    pupil = eye_patch(
        f"V3Pupil{suffix}",
        (x, cy - 0.008, cz - 0.004),
        0.0115,
        0.023,
        mats["pupil"],
    )

    sign = -1.0 if suffix == "L" else 1.0
    highlight_big = eye_patch(
        f"V3EyeHighlightBig{suffix}",
        (x - 0.012 * sign, cy - 0.010, cz + 0.023),
        0.009,
        0.013,
        mats["white"],
    )
    highlight_small = eye_patch(
        f"V3EyeHighlightSmall{suffix}",
        (x + 0.014 * sign, cy - 0.0105, cz - 0.003),
        0.0045,
        0.006,
        mats["white"],
    )

    for obj in (eye, iris_outer, iris_inner, pupil, highlight_big, highlight_small):
        base.parent_keep_world(obj, armature, "Head")

    # Deliberately thicker outer upper lash gives the eye the polished mobile-
    # game look missing from V1/V2.
    if suffix == "L":
        lash_points = [
            (x - 0.066, cy - 0.010, cz + 0.035),
            (x - 0.030, cy - 0.014, cz + 0.055),
            (x + 0.016, cy - 0.014, cz + 0.058),
            (x + 0.067, cy - 0.008, cz + 0.034),
        ]
        tip_start = (x - 0.064, cy - 0.008, cz + 0.036)
        tip_end = (x - 0.095, cy - 0.004, cz + 0.052)
    else:
        lash_points = [
            (x - 0.067, cy - 0.008, cz + 0.034),
            (x - 0.016, cy - 0.014, cz + 0.058),
            (x + 0.030, cy - 0.014, cz + 0.055),
            (x + 0.066, cy - 0.010, cz + 0.035),
        ]
        tip_start = (x + 0.064, cy - 0.008, cz + 0.036)
        tip_end = (x + 0.095, cy - 0.004, cz + 0.052)

    lash = base.curve_tube(f"V3UpperLash{suffix}", lash_points, 0.0054, mats["lash"])
    lash_tip = v2.hair_ribbon(
        f"V3LashTip{suffix}",
        [tip_start, tip_end],
        [0.010, 0.0035],
        mats["lash"],
        thickness=0.004,
        bevel=0.001,
    )
    brow = base.curve_tube(
        f"V3Brow{suffix}",
        [
            (x - 0.050, cy + 0.004, cz + 0.092),
            (x, cy - 0.003, cz + 0.104),
            (x + 0.050, cy + 0.004, cz + 0.092),
        ],
        0.0036,
        mats["hair_shadow"],
    )
    for obj in (lash, lash_tip, brow):
        base.parent_keep_world(obj, armature, "Head")


def create_face(armature: bpy.types.Object, mats: dict[str, bpy.types.Material]) -> None:
    for suffix, x in (("L", -0.088), ("R", 0.088)):
        create_eye(suffix, x, armature, mats)

    mouth = base.curve_tube(
        "V3Mouth",
        [
            (-0.033, -0.193, 1.022),
            (0.0, -0.199, 1.015),
            (0.033, -0.193, 1.022),
        ],
        0.0039,
        mats["blush"],
    )
    base.parent_keep_world(mouth, armature, "Head")

    # Nose is a tiny blush-toned mark rather than a separate sphere.
    nose = v2.face_disc(
        "V3Nose",
        (0.0, -0.198, 1.071),
        0.007,
        0.006,
        mats["blush"],
        16,
        0.0001,
    )
    base.parent_keep_world(nose, armature, "Head")

    blush_mat = base.material("V3CheekBlush", (1.0, 0.62, 0.72, 0.72), 0.86)
    try:
        blush_mat.surface_render_method = "DITHERED"
    except (AttributeError, TypeError):
        pass

    for suffix, x in (("L", -0.165), ("R", 0.165)):
        patch = v2.face_disc(
            f"V3CheekBlush{suffix}",
            (x, -0.190, 1.065),
            0.038,
            0.012,
            blush_mat,
            28,
            0.0001,
        )
        base.parent_keep_world(patch, armature, "Head")


def create_hair_cap(armature: bpy.types.Object, mat: bpy.types.Material) -> None:
    """Create a smooth crown mass kept behind the forehead and bangs."""

    crown = base.uv_sphere(
        "V3HairCap",
        (0.0, 0.080, 1.245),
        (0.282, 0.188, 0.180),
        mat,
        56,
        32,
    )
    # Keep the crown behind the face so it never becomes a visible helmet rim.
    # Bangs and side locks define the front hairline.
    for vertex in crown.data.vertices:
        world_z = vertex.co.z + 1.245
        if vertex.co.y < -0.060 and world_z < 1.250:
            vertex.co.y = -0.060
        if world_z < 1.105:
            vertex.co.z += (1.105 - world_z) * 0.60
    base.parent_keep_world(crown, armature, "Head")


def sample_cubic(p0, p1, p2, p3, steps: int) -> list[tuple[float, float, float]]:
    result = []
    a = Vector(p0)
    b = Vector(p1)
    c = Vector(p2)
    d = Vector(p3)
    for i in range(steps):
        t = i / (steps - 1)
        u = 1.0 - t
        point = (
            a * (u**3)
            + b * (3.0 * u * u * t)
            + c * (3.0 * u * t * t)
            + d * (t**3)
        )
        result.append(tuple(point))
    return result


def smooth_widths(root: float, middle: float, steps: int) -> list[float]:
    result = []
    for i in range(steps):
        t = i / (steps - 1)
        if t < 0.32:
            local = t / 0.32
            width = root + (middle - root) * math.sin(local * math.pi * 0.5)
        else:
            local = (t - 0.32) / 0.68
            width = middle * (1.0 - local) + 0.004 * local
        result.append(max(0.004, width))
    return result


def add_bezier_hair(
    name: str,
    controls,
    root_width: float,
    middle_width: float,
    material,
    armature,
    bone: str,
    *,
    steps: int = 14,
    thickness: float = 0.012,
) -> None:
    points = sample_cubic(*controls, steps)
    widths = smooth_widths(root_width, middle_width, steps)
    strand = v2.hair_ribbon(
        name,
        points,
        widths,
        material,
        thickness=thickness,
        bevel=0.003,
    )
    base.parent_keep_world(strand, armature, bone)


def create_hair(armature: bpy.types.Object, mats: dict[str, bpy.types.Material]) -> None:
    create_hair_cap(armature, mats["hair"])

    # Smooth long locks: four cubic control points are sampled into many ribbon
    # sections, eliminating the straight-card look of V1/V2.
    back_specs = [
        ("V3BackHairFarL", (-0.220, 0.145, 1.300), (-0.285, 0.180, 1.125), (-0.285, 0.175, 0.820), (-0.215, 0.145, 0.655), 0.032, 0.082, "HairBackL"),
        ("V3BackHairInnerL", (-0.105, 0.175, 1.335), (-0.165, 0.210, 1.110), (-0.155, 0.205, 0.780), (-0.075, 0.170, 0.605), 0.038, 0.098, "HairBackL"),
        ("V3BackHairCenter", (0.000, 0.190, 1.345), (-0.015, 0.220, 1.105), (0.015, 0.215, 0.760), (0.000, 0.180, 0.565), 0.042, 0.108, "HairBackC"),
        ("V3BackHairInnerR", (0.105, 0.175, 1.335), (0.165, 0.210, 1.110), (0.155, 0.205, 0.780), (0.075, 0.170, 0.605), 0.038, 0.098, "HairBackR"),
        ("V3BackHairFarR", (0.220, 0.145, 1.300), (0.285, 0.180, 1.125), (0.285, 0.175, 0.820), (0.215, 0.145, 0.655), 0.032, 0.082, "HairBackR"),
    ]
    for name, p0, p1, p2, p3, root, middle, bone in back_specs:
        add_bezier_hair(
            name,
            (p0, p1, p2, p3),
            root,
            middle,
            mats["hair_shadow"],
            armature,
            bone,
            steps=18,
            thickness=0.014,
        )

    # Curved face-framing locks taper below the jaw.
    for side, sign in (("L", -1.0), ("R", 1.0)):
        add_bezier_hair(
            f"V3FaceLock{side}",
            (
                (0.220 * sign, -0.070, 1.300),
                (0.285 * sign, -0.145, 1.210),
                (0.255 * sign, -0.185, 0.965),
                (0.165 * sign, -0.105, 0.805),
            ),
            0.018,
            0.046,
            mats["hair"],
            armature,
            f"HairSide{side}",
            steps=16,
            thickness=0.012,
        )

    # Airy bangs: each lock follows one smooth curve with a narrow root and a
    # real tapered tip, rather than five faceted polygon strips.
    bang_specs = [
        ("V3BangOuterL", (-0.220, -0.120, 1.335), (-0.215, -0.205, 1.310), (-0.175, -0.240, 1.205), (-0.130, -0.218, 1.150), 0.014, 0.052),
        ("V3BangInnerL", (-0.120, -0.145, 1.355), (-0.120, -0.220, 1.325), (-0.090, -0.250, 1.230), (-0.058, -0.225, 1.178), 0.013, 0.046),
        ("V3BangCenter", (-0.015, -0.155, 1.362), (-0.015, -0.225, 1.340), (0.002, -0.255, 1.270), (0.018, -0.226, 1.208), 0.012, 0.041),
        ("V3BangInnerR", (0.108, -0.145, 1.355), (0.110, -0.220, 1.325), (0.088, -0.250, 1.230), (0.056, -0.225, 1.178), 0.013, 0.046),
        ("V3BangOuterR", (0.212, -0.120, 1.335), (0.210, -0.205, 1.310), (0.175, -0.240, 1.205), (0.130, -0.218, 1.150), 0.014, 0.052),
    ]
    for name, p0, p1, p2, p3, root, middle in bang_specs:
        add_bezier_hair(
            name,
            (p0, p1, p2, p3),
            root,
            middle,
            mats["hair"],
            armature,
            "Head",
            steps=16,
            thickness=0.009,
        )

    # Small asymmetrical wisps/ahoge keep the silhouette from feeling like a
    # perfectly mirrored procedural helmet.
    ahoge = base.curve_tube(
        "V3Ahoge",
        [
            (-0.035, 0.005, 1.382),
            (-0.060, -0.005, 1.438),
            (-0.020, -0.010, 1.468),
            (0.010, -0.012, 1.445),
        ],
        0.006,
        mats["hair"],
    )
    highlight_l = base.curve_tube(
        "V3HairHighlightL",
        [
            (-0.155, -0.058, 1.340),
            (-0.195, -0.078, 1.270),
            (-0.210, -0.090, 1.205),
        ],
        0.0035,
        mats["white"],
    )
    highlight_r = base.curve_tube(
        "V3HairHighlightR",
        [
            (0.130, -0.050, 1.350),
            (0.180, -0.070, 1.285),
            (0.205, -0.082, 1.225),
        ],
        0.0032,
        mats["white"],
    )
    for obj in (ahoge, highlight_l, highlight_r):
        base.parent_keep_world(obj, armature, "Head")


def create_bow(armature: bpy.types.Object, mats: dict[str, bpy.types.Material]) -> None:
    """Large crystal-butterfly silhouette matching the approved concept."""

    cx = 0.248
    cz = 1.306

    parts = [
        (
            "V3BowUpperInner",
            [
                (cx - 0.018, cz + 0.008),
                (cx - 0.090, cz + 0.094),
                (cx - 0.145, cz + 0.085),
                (cx - 0.130, cz + 0.010),
                (cx - 0.040, cz - 0.020),
            ],
            mats["pale_lilac"],
        ),
        (
            "V3BowUpperOuter",
            [
                (cx + 0.018, cz + 0.008),
                (cx + 0.095, cz + 0.118),
                (cx + 0.176, cz + 0.104),
                (cx + 0.158, cz + 0.018),
                (cx + 0.040, cz - 0.020),
            ],
            mats["pale_lilac"],
        ),
        (
            "V3BowLowerInner",
            [
                (cx - 0.012, cz - 0.006),
                (cx - 0.102, cz - 0.022),
                (cx - 0.122, cz - 0.096),
                (cx - 0.050, cz - 0.085),
                (cx + 0.006, cz - 0.025),
            ],
            mats["ice_blue"],
        ),
        (
            "V3BowLowerOuter",
            [
                (cx + 0.012, cz - 0.006),
                (cx + 0.120, cz - 0.024),
                (cx + 0.146, cz - 0.108),
                (cx + 0.060, cz - 0.090),
                (cx - 0.004, cz - 0.025),
            ],
            mats["ice_blue"],
        ),
    ]
    for name, outline, material in parts:
        obj = v2.shape_prism(name, outline, -0.005, 0.020, material)
        base.parent_keep_world(obj, armature, "BowRoot")

    knot = base.uv_sphere(
        "V3BowKnot",
        (cx, -0.025, cz),
        (0.034, 0.018, 0.034),
        mats["lilac"],
        28,
        18,
    )
    shine = base.uv_sphere(
        "V3BowKnotShine",
        (cx - 0.010, -0.043, cz + 0.012),
        (0.009, 0.004, 0.011),
        mats["white"],
        16,
        10,
    )
    for obj in (knot, shine):
        base.parent_keep_world(obj, armature, "BowRoot")

    for suffix, sign, material in (
        ("L", -1.0, mats["pale_lilac"]),
        ("R", 1.0, mats["ice_blue"]),
    ):
        tail = v2.hair_ribbon(
            f"V3BowTail{suffix}",
            [
                (cx + 0.020 * sign, 0.005, cz - 0.030),
                (cx + 0.045 * sign, 0.010, cz - 0.100),
                (cx + 0.070 * sign, 0.015, cz - 0.170),
            ],
            [0.017, 0.026, 0.004],
            material,
            thickness=0.009,
            bevel=0.003,
        )
        base.parent_keep_world(tail, armature, f"BowTail{suffix}")


def create_neck_context(armature: bpy.types.Object, mats: dict[str, bpy.types.Material]) -> None:
    """Minimal bust context so head proportions can be judged without a full body."""

    neck = base.uv_sphere(
        "V3NeckContext",
        (0.0, 0.010, 0.895),
        (0.068, 0.060, 0.085),
        mats["skin"],
        28,
        18,
    )
    collar = base.uv_sphere(
        "V3CollarContext",
        (0.0, -0.005, 0.835),
        (0.150, 0.090, 0.060),
        mats["white"],
        32,
        18,
    )
    ribbon = base.uv_sphere(
        "V3CollarCrystal",
        (0.0, -0.096, 0.845),
        (0.040, 0.020, 0.032),
        mats["lilac"],
        24,
        14,
    )
    for obj, bone in ((neck, "Neck"), (collar, "Chest"), (ribbon, "Chest")):
        base.parent_keep_world(obj, armature, bone)


def configure_head_camera_and_light() -> None:
    def area(name, location, energy, size, color, target=(0.0, 0.0, 1.15)):
        bpy.ops.object.light_add(type="AREA", location=location)
        light = bpy.context.object
        light.name = name
        light.data.energy = energy
        light.data.shape = "DISK"
        light.data.size = size
        light.data.color = color
        light.rotation_euler = (Vector(target) - light.location).to_track_quat("-Z", "Y").to_euler()

    area("V3HeadKey", (1.9, -3.0, 2.5), 260, 2.8, (1.0, 0.91, 0.94))
    area("V3HeadFill", (-2.0, -2.0, 1.7), 185, 3.2, (0.74, 0.82, 1.0))
    area("V3HeadRim", (1.1, 2.0, 2.1), 125, 2.2, (0.76, 0.62, 1.0))

    bpy.ops.object.camera_add(location=(0.0, -3.6, 1.16))
    camera = bpy.context.object
    camera.name = "V3HeadCamera"
    camera.data.type = "ORTHO"
    camera.data.ortho_scale = 0.78
    direction = Vector((0.0, 0.0, 1.14)) - camera.location
    camera.rotation_euler = direction.to_track_quat("-Z", "Y").to_euler()
    bpy.context.scene.camera = camera


def export(output: Path) -> None:
    output.mkdir(parents=True, exist_ok=True)
    blend_path = output / "sena_v3_head.blend"
    glb_path = output / "sena_v3_head.glb"

    bpy.ops.wm.save_as_mainfile(filepath=str(blend_path.resolve()))
    bpy.ops.object.select_all(action="SELECT")
    bpy.ops.export_scene.gltf(
        filepath=str(glb_path.resolve()),
        export_format="GLB",
        export_animations=False,
        export_skins=True,
        export_morph=True,
    )
    print(f"[Sena V3 head] wrote {blend_path}")
    print(f"[Sena V3 head] wrote {glb_path}")


def main() -> None:
    options = cli_args()
    output = Path(options.output)

    base.reset_scene()
    base.configure_scene()
    armature = base.create_armature()
    mats = make_materials()

    head = create_anime_head(mats["skin"])
    base.parent_keep_world(head, armature, "Head")

    for suffix, x in (("L", -0.270), ("R", 0.270)):
        ear = create_ear(f"V3Ear{suffix}", x, mats["skin"])
        base.parent_keep_world(ear, armature, "Head")

    create_face(armature, mats)
    create_hair(armature, mats)
    create_bow(armature, mats)
    create_neck_context(armature, mats)
    configure_head_camera_and_light()

    export(output)
    print("[Sena V3 head] head-approval model generated")


if __name__ == "__main__":
    main()
