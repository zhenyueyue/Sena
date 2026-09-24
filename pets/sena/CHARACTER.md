# Sena Character Bible

## Identity

**Name:** Sena / 星奈  
**Role:** desktop companion  
**Visual age:** young adult  
**Personality:** calm, observant, warm, slightly sleepy, quietly playful  
**Theme:** starlight × desktop technology × companionship

Sena should feel comfortable existing beside a code editor for hours. Her design must remain readable at approximately 250–450 logical pixels tall, so silhouette, face, hair, and two or three signature details matter more than dense costume decoration.

## Face

- Soft oval anime face; not childlike.
- Cyan-violet irises with a darker outer ring.
- Small four-point star catchlight may appear in one eye, but keep normal white highlights too.
- Fine upper lashes; restrained lower lashes.
- Small natural mouth.
- Default expression: relaxed, attentive, faintly curious.
- Blush is subtle and only stronger in reactions.

## Hair

- Long navy-black hair reaching the lower back.
- Slight outward curve near the ends.
- Soft side bangs framing both cheeks.
- Violet tint/gradient only in the lower third of the hair.
- One small four-point star hair clip on the viewer's left.
- Hair volume should create one clear outer silhouette. Avoid dozens of detached thin strands.

## Outfit

Base outfit is modern and slightly futuristic rather than a school uniform.

- Off-white cropped jacket, soft fabric with clean geometric seams.
- Deep navy fitted inner top.
- Dark navy pleated skirt-over-shorts silhouette.
- Opaque black tights.
- Short dark ankle boots with one subtle cyan accent.
- Small star emblem or stitch detail on jacket cuff or chest, never a large logo.
- No dangling chains or dense accessories.

The outfit needs to work in a future Live2D rig. Keep jacket, inner top, skirt, hair front/back, star clip, eyes, mouth, arms, and accessories visually separable.

## Modular accessories

### Headphones

- Over-ear design.
- Deep navy shell.
- Thin violet ring with a small cyan status light.
- Must remain recognizable at 300 px character height.
- Used by `listening_music` and `coding_with_music`.

### Laptop

- Compact dark laptop, approximately shoulder-width when seated.
- Minimal back emblem: a four-point star.
- Screen glow should be faint cyan, not a bright rectangle.
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

1. Star hair clip position and shape.
2. Hair color and violet-end gradient.
3. Eye colors.
4. Jacket/inner-top/skirt palette.
5. Body proportions.
6. Face shape.
7. Accessory design.
8. Camera angle and scale unless the animation specification explicitly says otherwise.

If any of these drift between frames, regenerate/redraw before adding the frame to the package.
