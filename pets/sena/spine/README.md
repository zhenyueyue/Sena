# Sena Spine Assets

Production target: **Spine 3.8.75 Professional**.

The first required art gate is described in
[`../SPINE_SETUP_POSE_BRIEF.md`](../SPINE_SETUP_POSE_BRIEF.md).

The exact first-project build order is in
[`../SPINE_FIRST_EXPORT_CHECKLIST.md`](../SPINE_FIRST_EXPORT_CHECKLIST.md).

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

## Create the first PSD scaffold

Do not manually create and rename 79 layers.

In Photoshop choose **File -> Scripts -> Browse...** and run:

```text
tools/spine/create_sena_psd_template.jsx
```

The script reads the current `layer_contract.json` directly and creates:

```text
pets/sena/spine/source/sena.psd
```

with a 2400 x 3000 transparent RGB/8 canvas, 8 production LayerSets and exactly
the 79 required production art layers. It refuses to overwrite
an existing `sena.psd`.

The generated layers are intentionally empty. This is only a painting scaffold;
the final Source Gate rejects empty pixel bounds.

## PSD source gate

The source-art gate uses the existing machine-readable layer contract:

```text
pets/sena/spine/settings/layer_contract.json
```

Before a real PSD exists, validate the contract itself:

```powershell
cargo run --example sena_source_gate -- --contract-only
```

After `sena.psd` exists:

1. Open `pets/sena/spine/source/sena.psd` in Photoshop.
2. Choose **File -> Scripts -> Browse...**.
3. Run `tools/spine/export_psd_layers.jsx`.
4. Photoshop writes `pets/sena/spine/source/sena.layers.json`.
5. Run:

```powershell
cargo run --example sena_source_gate
```

The gate checks all 79 required named art layers, duplicate names, lower
`snake_case`, left/right eye independence, rear-hair segmentation, bow
segmentation, skirt front/mid/back segmentation, forbidden base-pose props and
non-empty pixel bounds.

The Photoshop script is read-only: it traverses the open document and writes a
JSON inventory next to the PSD; it does not rename, move, hide or modify layers.

CI follows the same rule. If neither source file is committed, it validates the
contract only. Once source art starts landing, `sena.psd` and
`sena.layers.json` must be committed together.

## R3B import gate

After exporting the first real Sena model from **Spine 3.8.75 Professional**,
place the files here:

```text
pets/sena/spine/export/
  sena.skel
  sena.atlas
  sena.png
```

Development JSON may be used instead of the binary skeleton:

```text
pets/sena/spine/export/sena.json
```

Then run:

```powershell
cargo run --example sena_spine_asset_gate
```

The R3B gate currently requires:

- Spine runtime version 3.8.x.
- skin `base`.
- animations `idle`, `blink_l`, and `blink_r`.
- all R3B bones declared in `r3b_contract.json`.
- the exact required parent hierarchy for those bones; correct names with the wrong parent still fail.
- a non-empty renderable setup pose.
- finite geometry and valid triangle indices.
- every texture page referenced by the atlas to exist on disk.
- a renderable `idle` frame.
- effective setup-pose vertices preferably below 1500 and no more than about 2500.

To inspect an arbitrary Spine 3.8 export without applying Sena-specific
requirements:

```powershell
cargo run --example sena_spine_asset_gate -- `
  --skeleton path/to/model.json `
  --atlas path/to/model.atlas `
  --inventory-only
```

The gate intentionally does **not** switch the official package to
`renderer: "spine"`. The static Sena artwork still needs visual approval
before the production renderer becomes the default.

## Versioned production contract

The machine-readable first-export contract lives at:

```text
pets/sena/spine/settings/r3b_contract.json
```

It is the source of truth for the R3B blocking requirements: Spine major/minor,
default skin, required animations, required bones, setup-pose geometry bounds
and vertex budgets.

Validate the contract before any real export exists:

```powershell
cargo run --example sena_spine_asset_gate -- --contract-only
```

GitHub Actions runs the same check automatically. Once a complete Sena export
is committed, CI automatically upgrades from `--contract-only` to the full
asset gate. A partial export (for example only `sena.atlas`) fails CI instead
of silently passing.
