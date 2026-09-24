# Sena Character Bible

## Identity

**Name:** Sena / 星奈  
**Role:** desktop companion  
**Visual age:** young adult  
**Personality:** calm, observant, warm, slightly sleepy, quietly playful  
**Theme:** moonlight × starlight × crystal butterflies × companionship

Sena should feel like a quiet dream living beside the user's desktop: delicate and luminous, but calm enough to remain on screen for hours. Her production Sprite must preserve the approved elegant body proportions and avoid the oversized-head or heavy-platform-shoe look that appeared in early drafts.

## Face

- Refined soft oval face with an elegant young-adult appearance.
- Head must remain proportionate to the body; do not enlarge it into chibi proportions.
- Large but not exaggerated violet-pink irises with a darker outer ring and glassy highlights.
- Fine upper lashes and delicate lower-lash detail.
- Small natural nose and softly defined lips.
- Default expression: gentle, dreamy, attentive and slightly wistful.
- Fine glitter/freckle-like star accents under the eyes are allowed, but should remain subtle at desktop scale.

## Hair

- Very long moonlight silver-white hair with a faint blush-pink/lilac tint.
- Hair reaches below the hips and forms a soft flowing silhouette.
- Airy bangs and face-framing strands; the forehead should not be fully hidden.
- A large translucent lavender crystal-organza bow is the primary head accessory.
- Small butterfly/star crystal ornaments may decorate the bow and side hair.
- Hair detail can be rich in the master artwork, but production frames must avoid excessive isolated one-pixel strands that create noisy alpha regions.

## Outfit

The approved base outfit is a moonlight-lavender crystal dress rather than the earlier techwear concept.

- White/lilac/ice-blue layered chiffon and lace dress.
- Short fitted inner skirt with longer translucent asymmetric outer layers and ribbon tails.
- Crystal-butterfly motifs at the waist, sleeves and selected ribbon ends.
- Delicate star/moon charms may hang from a few controlled attachment points.
- Sheer pale stockings/tights with restrained crystal-ribbon details.
- Shoes are elegant slim-heeled crystal shoes with a **thin sole**. No thick platform sole.
- Heel and sole mass must remain visually light; footwear must not dominate the lower silhouette.
- Jewelry is fine and delicate: butterfly choker, tiny stars, small crystals.
- The costume can sparkle, but the silhouette must stay readable at desktop scale.

For future Live2D, keep front/back hair, bow, face, eyes, mouth, upper/lower dress layers, ribbons, arms, jewelry clusters and optional accessories visually separable.

## Modular accessories

### Headphones

- Elegant over-ear design derived from the crystal-butterfly motif.
- Pearl white / translucent lavender shell with a subtle violet ring.
- Must remain recognizable at 300 px character height without becoming visually heavy.
- Used by `listening_music` and `coding_with_music`.

### Laptop

- Compact pearl-white or pale-lavender laptop, approximately shoulder-width when seated.
- Minimal back emblem: a small star or butterfly.
- Screen glow should be faint lilac-blue, not a bright rectangle.
- Used by `coding` and `coding_with_music`.

## Pose language

Sena should not constantly perform large gestures. Her charm comes from small, believable changes.

- **Idle:** relaxed standing/sitting pose, tiny breathing, occasional blink, subtle gaze shift.
- **Coding:** sits or kneels comfortably with laptop; hands alternate in a typing loop; eyes mostly on screen.
- **Listening:** headphones on; tiny head/body sway; one occasional musical micro-reaction.
- **Coding + Music:** laptop + headphones; typing remains primary, sway is secondary.
- **Drowsy:** posture softens, eyelids lower, head dips slightly.
- **Sleeping:** curled/seated sleeping pose; eyes closed; slow breathing. Optional small `Z` effect should be a separate overlay later, not permanently baked into every base frame.

## Expression set for future expansion

Not required for the first Sprite package, but preserve compatibility with:

- neutral
- happy
- curious
- surprised
- embarrassed
- annoyed
- sleepy
- focused

## Design invariants

Every generated or hand-drawn frame must preserve:

1. Moonlight silver-pink hair color.
2. Large translucent lavender bow shape and placement.
3. Violet-pink eye color and face structure.
4. Crystal-lavender layered dress design.
5. Elegant adult body proportions with a relatively small head.
6. Thin-soled slim-heel shoe silhouette.
7. Butterfly/star jewelry language.
8. Camera angle and scale unless the animation specification explicitly says otherwise.

If any of these drift between frames, regenerate/redraw before adding the frame to the package.
