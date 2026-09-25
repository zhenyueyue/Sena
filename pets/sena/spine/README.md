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
- bones `root`, `body_root`, `head`, `face_root`, `eye_l`, and `eye_r`.
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
