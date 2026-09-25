# Sena Blender pipeline

This directory turns the first Q版 3D model into a repeatable build step instead of a one-off hand-edited asset.

## Current V1 generator

`generate_sena_v1.py` creates an editable Blender blockout containing:

- ~3-head-tall Sena silhouette.
- Moonlight silver/lilac hair blocks.
- Violet-pink eyes.
- Simplified crystal-lilac dress and signature bow.
- Cream/orange-white chibi cat.
- Humanoid armature.
- Runtime attachment bones: `CatCarry`, `RightHandProp`, `LeftHandProp`, `Headphones`, `Laptop`.
- Named V1 actions including `Idle`, `Walk`, turning, sitting and cat-carry clips.
- Transparent Eevee preview scene.
- `.blend` and `.glb` output.

This is a **procedural blockout**, not the final art model. Its job is to lock proportions, rig names, animation names and attachment semantics so visual refinement does not break runtime integration.

## Build

Blender 4.x is required for generation.

```powershell
./tools/blender/build_sena_v1.ps1
```

Custom Blender path:

```powershell
./tools/blender/build_sena_v1.ps1 -Blender "D:\Apps\Blender\blender.exe"
```

Default output:

```text
pets/sena/models/generated/
  sena_v1.blend
  sena_v1.glb
```

Generated binaries are local build artifacts and should be reviewed before promoting a model to the official `pets/sena/models/sena.vrm` or `sena.glb` slot.

## Refinement strategy

The next visual pass should replace primitive blockout meshes without renaming the armature, actions or anchors. That lets us improve face, hair, clothes and cat quality independently from the Rust animation controller.

VRM remains the preferred final character format. The GLB generated here is the lowest-friction preview artifact; a VRM export step will be added once the VRM exporter is part of the toolchain.
