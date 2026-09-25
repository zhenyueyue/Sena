# Sena Spine Assets

Production target: **Spine 3.8.75 Professional**.

The first required art gate is described in
[`../SPINE_SETUP_POSE_BRIEF.md`](../SPINE_SETUP_POSE_BRIEF.md).

Expected layout:

```text
spine/
  source/
    sena.psd
    cat.psd

  project/
    sena.spine
    sena_cat.spine

  export/
    sena.skel
    sena.atlas
    sena.png
    sena_cat.skel
    sena_cat.atlas
    sena_cat.png

  settings/
    layer_contract.json
    export.json
    texture_packer.json
```

Source PSD, Spine projects, binary skeletons and exported atlas textures are
covered by Git LFS rules in the repository root.

Do not place flattened concept previews in `source/`. Concept previews belong
in `pets/sena/spine/review/` and are only promoted after the still-art Gate A
passes.
