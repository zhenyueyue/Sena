# Sena Pet Packages

Sena loads pet packages from a directory containing a `pet.json` manifest.

The default lookup order is:

1. `SENA_PET_PACKAGE` environment variable, when set.
2. `pets/default` next to `sena.exe`.
3. The repository's `pets/default` directory while developing Sena.

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
  "license": "CC-BY-4.0",
  "animations": {
    "idle": {
      "frames": [
        "animations/idle/000.webp"
      ],
      "interval_ms": null,
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
