# Petting interaction

The runtime currently ships a **V1 petting reaction** assembled from approved idle frames: open eyes → Sena-only blink/closed eyes → small settle → return. The existing procedural bounce is layered on top, so double-clicking already reads as a distinct petting response without introducing a newly generated off-model face.

The final production sequence should replace those reused idle paths with dedicated files in this directory:

- `000.webp` — neutral entry
- `001.webp` — head lowers 2–4 px / hair compresses subtly
- `002.webp` — eyes softly closed / warm smile
- `003.webp` — small rebound / eyes half-open
- `004.webp` — neutral exit

Keep every frame on the 768×1024 transparent production canvas and use the timing already defined in `pet.template.json` unless animation review calls for a change.

When the five dedicated files are approved, copy the `interactions.petting` block from `pet.template.json` into `pet.json`. No Rust changes are required.

This animation is one-shot. When it finishes, Sena automatically returns to the desktop-context animation that is currently appropriate.
