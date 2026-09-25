"""Render front / three-quarter / side review images from a generated Sena blend."""

from __future__ import annotations

import argparse
import math
import sys
from pathlib import Path

import bpy
from mathutils import Vector


def args() -> argparse.Namespace:
    argv = sys.argv
    argv = argv[argv.index("--") + 1 :] if "--" in argv else []
    parser = argparse.ArgumentParser()
    parser.add_argument("--output", default="pets/sena/models/generated/previews")
    return parser.parse_args(argv)


def look_at(camera, target: Vector) -> None:
    camera.rotation_euler = (target - camera.location).to_track_quat("-Z", "Y").to_euler()


def render_view(camera, output: Path, name: str, position, target=(0.06, 0.0, 0.70)) -> None:
    camera.location = position
    look_at(camera, Vector(target))
    bpy.context.scene.render.filepath = str((output / f"{name}.png").resolve())
    bpy.ops.render.render(write_still=True)
    print(f"[Sena preview] wrote {output / f'{name}.png'}")


def main() -> None:
    options = args()
    output = Path(options.output)
    output.mkdir(parents=True, exist_ok=True)

    camera = bpy.data.objects.get("PreviewCamera")
    if camera is None:
        raise RuntimeError("PreviewCamera not found; generate the Sena model first")

    scene = bpy.context.scene
    scene.camera = camera
    scene.render.resolution_x = 640
    scene.render.resolution_y = 640
    scene.render.resolution_percentage = 100
    scene.render.image_settings.file_format = "PNG"
    scene.render.film_transparent = True
    camera.data.type = "ORTHO"
    camera.data.ortho_scale = 1.72

    render_view(camera, output, "front", (0.08, -4.4, 1.0))
    render_view(camera, output, "three_quarter", (2.55, -3.45, 1.05))
    render_view(camera, output, "side", (4.35, -0.25, 1.0))


if __name__ == "__main__":
    main()
