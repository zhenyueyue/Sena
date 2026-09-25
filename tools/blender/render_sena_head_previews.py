"""Render Sena V3 head-approval views."""

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
        default="pets/sena/models/generated/previews_v3_head",
    )
    return parser.parse_args(argv)


def look_at(camera, target: Vector) -> None:
    camera.rotation_euler = (target - camera.location).to_track_quat("-Z", "Y").to_euler()


def render(camera, output: Path, name: str, position, target=(0.0, 0.0, 1.15)) -> None:
    camera.location = position
    look_at(camera, Vector(target))
    bpy.context.scene.render.filepath = str((output / f"{name}.png").resolve())
    bpy.ops.render.render(write_still=True)
    print(f"[Sena V3 head] wrote {output / f'{name}.png'}")


def main() -> None:
    options = args()
    output = Path(options.output)
    output.mkdir(parents=True, exist_ok=True)

    camera = bpy.data.objects.get("V3HeadCamera")
    if camera is None:
        raise RuntimeError("V3HeadCamera not found")

    scene = bpy.context.scene
    scene.camera = camera
    scene.render.resolution_x = 720
    scene.render.resolution_y = 720
    scene.render.resolution_percentage = 100
    scene.render.image_settings.file_format = "PNG"
    scene.render.film_transparent = True
    try:
        scene.render.use_freestyle = False
    except AttributeError:
        pass
    camera.data.type = "ORTHO"
    camera.data.ortho_scale = 0.78

    render(camera, output, "front", (0.0, -3.6, 1.16))
    render(camera, output, "three_quarter", (2.05, -2.85, 1.18))
    render(camera, output, "side", (3.55, -0.10, 1.16))


if __name__ == "__main__":
    main()
