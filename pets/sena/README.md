# Sena / 星奈

This directory is the production character package used by Sena at runtime. `pets/default` remains only as the safe placeholder/fallback package.

## Character direction

Sena is an original adult Japanese-anime-style desktop companion with a calm, warm, dreamy presence. Her current approved visual identity is **moonlight × starlight × crystal butterflies × companionship**.

Core visual identity:

- Elegant young-adult proportions, approximately **6.5–7 heads tall** with a relatively small refined head.
- Very long moonlight silver-white hair with a faint blush-pink / lilac tint.
- Violet-pink eyes with glassy highlights.
- Large translucent lavender crystal-organza bow with butterfly/star ornaments.
- White / lilac / ice-blue layered chiffon-and-lace crystal dress.
- Delicate butterfly, moon and star jewelry rather than heavy accessories.
- Pale stockings and slim crystal heels with a **thin sole**; never use thick platform shoes.
- Calm, observant, warm, slightly sleepy and quietly playful expression language.
- Companion cat remains part of the approved desktop-pet presentation; human and cat blinks must be staggered.

The complete visual rules live in [CHARACTER.md](CHARACTER.md). Treat that file as the source of truth if any older prompt or asset note disagrees with this README.

## Animation model

Environment-driven animations are stored under `animations` in `pet.json`:

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
- [SPRITE_GUIDE.md](SPRITE_GUIDE.md): canvas, animation and export requirements.
- [pet.json](pet.json): current runtime manifest.
- [pet.template.json](pet.template.json): package template for asset production.
- `animations/`: final transparent WebP/PNG frames.

## Asset validation

Before committing new frames, run:

```powershell
cargo run --example sena_asset_check -- pets/sena
```

The checker validates the 768×1024 production canvas, file existence, transparent corners, timing arrays and one-shot interaction rules. A warning about the recommended 32px transparent margin is reviewable; manifest errors, missing files, wrong canvas size and opaque corners fail validation.
