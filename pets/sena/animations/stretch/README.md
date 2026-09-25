# Stretch interaction

The runtime currently ships a **V1 stretch reaction** assembled from approved idle frames. It alternates the neutral and tiny breathing/hair-settle frames while the UI layers a controlled upward stretch curve (`0 → -2 → -5 → -8 → -8 → -5 → -2 → 0 px`). This keeps Sena perfectly on-model while making the autonomous “伸懒腰” event visibly different from normal idle.

The final production sequence should replace the reused idle paths with dedicated files here:

- `000.webp` — neutral entry
- `001.webp` — shoulders begin to lift
- `002.webp` — arms / upper body extend slightly
- `003.webp` — comfortable stretch apex
- `004.webp` — begin settling
- `005.webp` — neutral exit

Keep the feet and baseline stable inside the 768×1024 canvas. The animation should feel small enough for a desktop companion, with hair and ribbons following the body by only a few pixels.

`pet.template.json` already contains the final six-frame paths and suggested timing. When those frames are approved, copy that `interactions.stretch` block into `pet.json`; no Rust changes are needed. Review the procedural vertical overlay at that point and reduce it if the final artwork already contains enough body extension.
