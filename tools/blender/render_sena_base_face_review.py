"""Render close-up face review views with hair/body hidden."""

from __future__ import annotations

import argparse
import sys
from pathlib import Path

import bpy
from mathutils import Vector


def args() -> argparse.Namespace:
    argv = sys.argv
    argv = argv[argv.index("--") + 1 :] if "--" in argv else []
    parser = argparse.ArgumentParser()
    parser.add_argument(
        "--output",
        default="pets/sena/models/generated/base_chibi_01/face_review",
    )
    return parser.parse_args(argv)


def look_at(camera: bpy.types.Object, target: Vector) -> None:
    camera.rotation_euler = (target - camera.location).to_track_quat("-Z", "Y").to_euler()


def render(
    camera: bpy.types.Object,
    output: Path,
    name: str,
    position: tuple[float, float, float],
    target: tuple[float, float, float],
) -> None:
    camera.location = position
    look_at(camera, Vector(target))
    bpy.context.scene.render.filepath = str((output / f"{name}.png").resolve())
    bpy.ops.render.render(write_still=True)
    print(f"[Sena face review] wrote {output / f'{name}.png'}")


def main() -> None:
    options = args()
    output = Path(options.output)
    output.mkdir(parents=True, exist_ok=True)

    face = bpy.data.objects.get("Face")
    hair = bpy.data.objects.get("Hair")
    body = bpy.data.objects.get("Body")
    if face is None or hair is None or body is None:
        raise RuntimeError("Expected Face/Body/Hair VRoid objects")

    hair.hide_render = True
    body.hide_render = True

    camera = bpy.data.objects.get("BaseReviewCamera")
    if camera is None:
        raise RuntimeError("BaseReviewCamera not found")

    scene = bpy.context.scene
    scene.camera = camera
    scene.render.resolution_x = 720
    scene.render.resolution_y = 720
    scene.render.resolution_percentage = 100
    scene.render.film_transparent = True
    camera.data.type = "ORTHO"
    camera.data.ortho_scale = 0.47

    target = (0.0, 0.0, 0.790)
    render(camera, output, "front", (0.0, -2.2, 0.80), target)
    render(camera, output, "three_quarter", (1.28, -1.78, 0.81), target)
    render(camera, output, "side", (2.2, -0.04, 0.80), target)

    hair.hide_render = False
    body.hide_render = False


if __name__ == "__main__":
    main()
