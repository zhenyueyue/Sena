# Sena Pet Packages

Sena loads pet packages from a directory containing a `pet.json` manifest.

The default lookup order is:

1. `SENA_PET_PACKAGE` environment variable, when set.
2. `pets/sena` next to `sena.exe` when an official `pet.json` is present and valid.
3. The repository's `pets/sena` directory while developing Sena.
4. `pets/default` next to `sena.exe`.
5. The repository's `pets/default` placeholder package.

An invalid higher-priority bundled package is skipped so the next valid bundled package can still start Sena.

If a package cannot be loaded or validated, Sena falls back to its built-in placeholder so a broken pet package cannot prevent the application from starting.

## Manifest

Current schema version: `1`.

```json
{
  "schema_version": 1,
  "id": "example.character",
  "name": "Example",
  "display_name": "示例角色",
  "version": "1.0.0",
  "author": "Example Author",
  "renderer": "sprite",
  "sprite": {
    "scale": 0.75,
    "alpha_threshold": 8
  },
  "license": "CC-BY-4.0",
  "animations": {
    "idle": {
      "frames": [
        "animations/idle/000.webp",
        "animations/idle/001.webp"
      ],
      "frame_durations_ms": [1800, 120],
      "looping": true
    },
    "coding": {
      "frames": [
        "animations/coding/000.webp",
        "animations/coding/001.webp"
      ],
      "interval_ms": 160,
      "looping": true
    }
  }
}
```

`interval_ms` applies one cadence to every frame. `frame_durations_ms` optionally overrides it with one positive duration per frame; when present, its length must exactly match the effective frame count. This is preferred for occasional blink/reaction frames that should be brief while neutral frames remain on screen much longer.

Supported animation keys are:

- `idle`
- `coding`
- `listening_music`
- `coding_with_music`
- `drowsy`
- `sleeping`

Supported renderer identifiers are currently reserved as:

- `placeholder` — Sena's built-in development renderer.
- `sprite` — active transparent PNG/WebP frame renderer with lazy decoded-frame caching.
- `live2d` — reserved for the future Cubism renderer.

Sprite asset paths must be relative to the package directory. Absolute paths and `..` path traversal are rejected. Sprite frame files are intentionally limited to PNG and WebP so Sena does not ship unnecessary image decoders.

### Sprite display and hit testing

`sprite.scale` controls the on-screen size relative to the source image. `1.0` means one logical desktop pixel per source pixel, `0.5` is half size, and `2.0` is double size. Accepted values are `0.1` through `4.0`.

`sprite.alpha_threshold` controls native mouse hit testing. Pixels whose alpha is below the threshold are excluded from the Win32 window region, so clicks pass through transparent parts of the character to applications underneath. The default is `8`, which keeps anti-aliased character edges while ignoring nearly transparent background pixels.

All frames in one character package should use the same canvas dimensions. Sena can handle differing frame dimensions, but a changing canvas would resize the native window between frames and create visible jitter.

### Incremental animation fallback

Sprite packages do not need every behavior to be finished at once. If the current behavior has no usable Sprite frames, Sena automatically uses that package's `idle` animation while preserving the real semantic behavior internally.

This lets an early character package ship only `animations/idle/000.webp`. Other behaviors keep the official character visible through Idle until their own frames are added.

## Recommended Sprite Layout

```text
pets/
└─ my-character/
   ├─ pet.json
   ├─ preview.webp
   └─ animations/
      ├─ idle/
      │  └─ 000.webp
      ├─ coding/
      │  ├─ 000.webp
      │  └─ 001.webp
      ├─ listening_music/
      ├─ coding_with_music/
      ├─ drowsy/
      └─ sleeping/
```

Character artwork and model licenses are independent from Sena's Apache-2.0 source-code license. Every distributable character package should declare the license that applies to its own assets.
