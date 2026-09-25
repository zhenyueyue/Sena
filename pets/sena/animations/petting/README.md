# Petting interaction

Place the dedicated double-click / head-petting sprite sequence here.

Recommended naming:

- `000.webp`
- `001.webp`
- `002.webp`
- ...

Then list those files under `interactions.petting.frames` in `pets/sena/pet.json` and set per-frame timing with `frame_durations_ms` when needed.

This animation is one-shot. When it finishes, Sena automatically returns to the desktop-context animation that was active before the interaction.
