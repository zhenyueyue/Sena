# Look-at-cat interaction

The runtime currently ships a **V1 look-at-cat reaction** assembled from approved idle frames. Sena keeps her normal open-eye pose while the cat uses its dedicated blink frame, and the UI layers a tiny lean toward the cat plus a small heart accent. This gives the autonomous event a readable “noticed the cat / cat noticed back” beat without generating an off-model head turn.

The final production sequence should replace the reused idle paths with dedicated files here:

- `000.webp` — neutral entry
- `001.webp` — Sena shifts her gaze/head slightly toward the cat
- `002.webp` — cat returns a tiny blink/ear/head response
- `003.webp` — brief warm hold between them
- `004.webp` — both settle back to neutral

Keep the body baseline fixed on the 768×1024 canvas. Sena's head turn should be small; the cat's response should be even smaller so the pair still reads as a calm desktop companion rather than a large gesture.

`pet.template.json` already contains the final five-frame paths and suggested timing. When those frames are approved, copy that `interactions.look_at_cat` block into `pet.json`; no Rust changes are required. At that point the procedural lean can be reduced if the dedicated artwork already carries enough directional motion.
