# Sena / 星奈

This directory is the production character package used by Sena at runtime. `pets/default` remains only as the safe placeholder/fallback package.

## Character direction

Sena is an original **chibi 3D** desktop companion with a calm, warm, dreamy presence. Her current approved visual identity is **moonlight × starlight × crystal butterflies × companionship**.

Core visual identity:

- Chibi proportions around **3 heads tall** (roughly 2.8–3.3), while keeping Sena's young-adult identity rather than making her childlike.
- Very long moonlight silver-white hair with a faint blush-pink / lilac tint.
- Violet-pink eyes with glassy highlights.
- Large translucent lavender crystal-organza bow with butterfly/star ornaments.
- White / lilac / ice-blue layered chiffon-and-lace crystal dress.
- Delicate butterfly, moon and star jewelry rather than heavy accessories.
- Simplified crystal shoes with a light silhouette; no oversized platform sole.
- Calm, observant, warm, slightly sleepy and quietly playful expression language.
- Companion cat remains part of the approved desktop-pet presentation; human and cat blinks must be staggered.

The complete visual rules live in [CHARACTER.md](CHARACTER.md). Treat that file as the source of truth if any older prompt or asset note disagrees with this README.

## Rendering direction

The long-term primary renderer is Q版 3D (`VRM` preferred, `GLB` supported). The current Sprite package remains fully supported as a safe fallback until the 3D renderer passes transparency, hit-testing, locomotion and power-usage acceptance tests.

The 3D V1 contract prioritizes `Idle`, `Walk`, real 3D turning, sitting and carrying the cat. See [MODEL3D_GUIDE.md](MODEL3D_GUIDE.md).

The existing Sprite fallback still uses environment-driven animations stored under `animations` in `pet.json`:

- `idle`
- `coding`
- `listening_music`
- `coding_with_music`
- `drowsy`
- `sleeping`

One-shot character interactions are stored separately under `interactions`:

- `petting`
- `stretch`
- `look_at_cat`
- `daydream`

Interaction slots are optional. When an interaction has no production frames yet, Sena automatically uses the procedural fallback animation instead.

## Production files

- [CHARACTER.md](CHARACTER.md): authoritative character bible.
- [MODEL3D_GUIDE.md](MODEL3D_GUIDE.md): Q版 3D proportions, budgets, rig, motions, expressions and anchors.
- [SPRITE_GUIDE.md](SPRITE_GUIDE.md): legacy/fallback Sprite canvas, animation and export requirements.
- [pet.json](pet.json): current Sprite runtime manifest.
- [pet.template.json](pet.template.json): migration template containing both Sprite data and the future `model3d` contract.
- `models/`: Q版 3D model and prop assets.
- `animations/`: current transparent WebP/PNG fallback frames.

## Asset validation

Before committing new frames, run:

```powershell
cargo run --example sena_asset_check -- pets/sena
```

The checker validates the 768×1024 production canvas, file existence, transparent corners, timing arrays and one-shot interaction rules. A warning about the recommended 32px transparent margin is reviewable; manifest errors, missing files, wrong canvas size and opaque corners fail validation.
