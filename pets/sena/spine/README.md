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

## Create the Spine 3.8 project scaffold

The first Spine project structure is generated from the same R3B contract:

```powershell
cargo run --example sena_spine_bootstrap -- --force
```

This writes:

```text
pets/sena/spine/project/sena.bootstrap.json
```

The bootstrap is valid Spine 3.8.75 JSON and already contains:

- 13 R3B bones in parent-first hierarchy.
- 60 core slots in the initial draw order.
- slot -> bone mappings.
- setup attachment names.
- slot blend modes, including additive `bow_glow`.
- an empty `base` skin scaffold.
- empty `idle`, `blink_l`, and `blink_r` animation scaffolds.

It deliberately contains **no fake attachments, mesh geometry, weights, or
animation keys**.

In Spine Editor 3.8.75 Professional:

1. Start an empty project.
2. Use **Import Data** and select `sena.bootstrap.json`.
3. Keep the imported skeleton structure and position the bones against the
   approved Sena setup-pose artwork.
4. Import/create the real image attachments and weighted meshes.
5. Fill the `base` skin.
6. Animate `idle`, `blink_l`, and `blink_r`.
7. Save the editable project as:

```text
pets/sena/spine/project/sena.spine
```

The bootstrap JSON is a generated scaffold, not the final editable source
project and not a Runtime export.

Check that the tracked scaffold still matches the versioned contract:

```powershell
cargo run --example sena_spine_bootstrap -- --check
```

CI runs this check automatically.

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
- 60 core slots with the required slot -> bone mapping.
- 63 required attachments, including alternate mouth states on one `mouth` slot.
- valid setup attachments.
- expected Region / Mesh / Linked Mesh attachment categories.
- required blend modes, including additive `bow_glow`.
- 22 relative draw-order constraints for rear hair, layered skirt and eye stacks.
- all required attachments must belong directly to `base`; fallback from another/default skin does not count.
- `idle` must be 4–6 seconds and may not bake blink scale/shear or eye attachment/deform timelines.
- `blink_l` / `blink_r` must be 0.08–0.35 seconds and their timeline targets must stay completely on their own eye side.
- blink animations may not contain global Event/DrawOrder timelines or constraint timelines.
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
