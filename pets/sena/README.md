# Sena / 星奈

This directory is the production workspace for Sena's first real character asset set.

The current application still ships `pets/default` as the safe placeholder package. Do not rename this directory to `default` until every required animation is complete and validated.

## Character direction

Sena is an original adult Japanese-anime-style desktop companion with a calm, warm, slightly futuristic presence. She should feel like a character who quietly lives on the desktop rather than a mascot pasted on top of it.

Core visual identity:

- Young adult woman; clearly adult proportions and styling.
- Semi-chibi anime proportion: roughly 4.5 heads tall. Cute enough for a desktop pet, but not super-deformed.
- Long dark navy-black hair with a subtle violet gradient toward the ends.
- Soft cyan-violet eyes with a small star-like highlight.
- Small four-point star hair clip on the viewer's left side; this is the strongest silhouette/detail identifier.
- Off-white cropped tech jacket over a deep navy inner top.
- Dark navy pleated skirt/shorts silhouette with opaque black tights.
- Small lavender/cyan luminous accents, used sparingly.
- Neutral expression is gentle and attentive rather than permanently smiling.
- Accessories are modular: over-ear headphones for music and a compact laptop for coding.

Primary palette:

| Role | Color |
| --- | --- |
| Hair base | `#20263D` |
| Hair highlight | `#45406B` |
| Star violet | `#9B8CFF` |
| Ice cyan | `#72D8F4` |
| Jacket | `#F5F3FA` |
| Deep navy | `#242A42` |
| Warm skin accent | `#F3B8AC` |

Avoid:

- Existing anime/game character likenesses.
- School uniforms or child-coded styling.
- Highly detailed jewelry that disappears at desktop scale.
- Large loose particles baked into every frame.
- Very thin isolated hair strands that produce noisy alpha hit regions.
- Perspective or camera changes between animation states.

## Production files

- [CHARACTER.md](CHARACTER.md): detailed character bible.
- [SPRITE_GUIDE.md](SPRITE_GUIDE.md): canvas, animation and export requirements.
- [pet.template.json](pet.template.json): target package manifest once production frames exist.
- `animations/`: final transparent WebP/PNG frames.
