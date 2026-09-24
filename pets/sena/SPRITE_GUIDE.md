# Sena Sprite Production Guide

## Canvas

All production frames use the same transparent canvas:

- **Source canvas:** 768 × 1024 px
- **Aspect ratio:** 3:4
- **Background:** fully transparent
- **Character baseline:** y = 980 px
- **Horizontal center:** x = 384 px
- Keep at least 32 px transparent padding around the maximum silhouette.
- Do not crop individual animation frames differently.

The initial manifest uses `sprite.scale = 0.36`, producing a desktop window of approximately **276 × 369 logical pixels** before OS DPI scaling.

This is deliberately larger than the old placeholder while remaining compact enough to sit beside ordinary desktop windows.

## Rendering style

- Clean high-quality anime illustration.
- Soft cel shading with restrained gradients.
- Crisp outer silhouette.
- Fine internal line work, but avoid hairline-thin detached elements.
- Transparent edges should be properly anti-aliased.
- No baked background, frame, drop shadow rectangle, text, UI, watermark, or signature.
- Lighting direction stays consistent across all frames.
- Camera is fixed: slight eye-level/front three-quarter feel, no lens changes.

## Alpha requirements

- Fully empty canvas pixels must have alpha 0.
- Do not leave invisible RGB matte halos around the character.
- Semi-transparent anti-aliasing is fine.
- Avoid huge low-alpha glows around the full silhouette because they enlarge the native hit region.
- Package default `alpha_threshold` is 8.

## First production animation set

### Idle — 4 frames

Cadence: **650 ms/frame**

Purpose: calm presence without permanent high-frequency animation.

1. Neutral open-eye pose.
2. Tiny breath / hair settle.
3. Blink.
4. Return to neutral.

The loop should feel almost static. Large body movement is incorrect.

### Coding — 4 frames

Cadence: **180 ms/frame**

- Laptop visible.
- Alternate left/right hand typing.
- Small screen-focused eye movement.
- Torso remains stable so the loop does not vibrate.

### Listening Music — 4 frames

Cadence: **260 ms/frame**

- Headphones visible.
- Two-position gentle sway.
- Hair follows by only a few pixels.
- Optional tiny closed-eye content expression on one frame.

### Coding With Music — 4 frames

Cadence: **180 ms/frame**

- Laptop + headphones.
- Typing remains the main motion.
- Music sway is reduced compared with pure listening state.

### Drowsy — 3 frames

Cadence: **850 ms/frame**

1. Heavy eyelids.
2. Head dips slightly.
3. Small recovery, still sleepy.

No dramatic wobble.

### Sleeping — 3 frames

Cadence: **1400 ms/frame**

- Stable sleeping pose.
- Very subtle breathing expansion/contraction.
- Eyes stay closed.
- No position jump between frames.

## Frame naming

Always zero-pad frame names:

```text
000.webp
001.webp
002.webp
003.webp
```

Preferred format is **lossless WebP**. PNG is acceptable during production and review.

## Directory layout

```text
animations/
├─ idle/
│  ├─ 000.webp
│  ├─ 001.webp
│  ├─ 002.webp
│  └─ 003.webp
├─ coding/
├─ listening_music/
├─ coding_with_music/
├─ drowsy/
└─ sleeping/
```

## Generation / drawing workflow

For AI-assisted production, do **not** generate each frame independently from text alone.

Recommended workflow:

1. Create one approved neutral master character sheet.
2. Lock the appearance and palette.
3. Use that approved image as the character reference for every state.
4. Generate/draw one key pose for each behavior.
5. Derive neighboring animation frames from that key pose with minimal motion.
6. Normalize all frames to the 768 × 1024 canvas.
7. Compare face, hair clip, clothing seams, proportions, and accessory geometry between frames.
8. Remove/repair any inconsistent frame before packaging.

The master reference is more important than adding additional prompt detail.

## Base generation brief

Use this as the visual target when producing the first approved master image:

> Original adult anime woman named Sena (星奈), semi-chibi 4.5-head-tall proportions, long dark navy-black hair with subtle violet gradient at the ends, cyan-violet eyes, small four-point star hair clip on viewer-left side, off-white cropped futuristic jacket, deep navy inner top, dark navy skirt-over-shorts silhouette, opaque black tights, short dark ankle boots, restrained lavender and cyan luminous accents, calm warm attentive expression, clean modern Japanese anime illustration, soft cel shading, crisp silhouette, full body, fixed front three-quarter camera, transparent background, no text, no watermark.

Do not add environment/background elements to production Sprite frames.
