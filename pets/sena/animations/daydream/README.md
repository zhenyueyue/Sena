# Daydream interaction

The runtime currently ships a **V1 daydream reaction** assembled from approved idle frames: neutral gaze → tiny breathing settle → Sena-only closed eyes → a longer quiet hold → reopen → return. The UI layers only a 1–2 px slow drift plus the existing `…` or `✦` accent, so the event reads as a brief moment of zoning out rather than sleep.

The final production sequence should replace the reused idle paths with dedicated files here:

- `000.webp` — neutral entry / gaze begins to drift away
- `001.webp` — eyes soften, attention leaves the foreground
- `002.webp` — quiet daydream hold with minimal hair/breath motion
- `003.webp` — refocus and return to neutral

Keep the body baseline stable on the 768×1024 canvas. Do not make this look sleepy enough to compete with the Drowsy/Sleeping states; Daydream should feel awake but mentally elsewhere.

`pet.template.json` already contains the final four-frame paths and suggested timing. Both the “发会儿呆也不错……” and quiet-companion autonomous events use this slot. When dedicated artwork is approved, copy the `interactions.daydream` block into `pet.json`; no Rust changes are required. The procedural 1–2 px drift can be reduced later if the final frames already contain enough motion.
