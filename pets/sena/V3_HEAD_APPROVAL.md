# Sena V3 — Head Approval Gate — Archived

> **Archived / historical experiment.** V3 procedural/Blender head work is no longer active.
> Production has moved to Spine 2D.


V3 is the art-quality rebuild. V1/V2 remain technical references for rigging,
animation names, attachments and runtime experiments.

**Current status: WIP — NOT APPROVED.**

Do not start the V3 body, costume, cat or runtime promotion until this gate is
explicitly passed.

## Visual source of truth

The approved concept direction is the latest Sena Q版 3D character sheet:

- ~3-head-tall mobile-game chibi proportions.
- Soft rounded forehead and cheeks with a small tapered chin.
- Large violet-pink anime eyes, not circular toy eyes.
- Moonlight silver-white hair with pale lilac/pink shading.
- Layered airy bangs; no five-block fringe or helmet cap silhouette.
- Long flowing hair with curved locks and soft pointed ends.
- Large crystal butterfly bow on the character's side.
- Calm, dreamy, warm young-adult character identity.

## Approval views

Every revision must be reviewed as:

1. Front.
2. Three-quarter.
3. True side.

The head fails if any one view looks wrong.

## Approval criteria

### Face

- Chin is small and tapered without becoming sharp.
- Cheeks read soft/round, not rectangular or spherical.
- Forehead-to-chin silhouette resembles the concept sheet.
- Nose and mouth remain subtle from front and readable from profile.

### Eyes

- Eyes sit on the face rather than protruding like separate balls.
- Eye shape is wider than the early V1/V2 circular look.
- Purple/pink iris has visible dark outer ring, inner color and highlights.
- Profile view shows a believable eye/lash edge.

### Hair

- Crown must not read as a helmet.
- Bangs must have curved roots/bodies and tapered tips.
- Face locks must follow cheek/jaw curvature rather than hang as flat boards.
- Back hair must form one readable flowing mass while still being separable for
  secondary motion.
- Hair color must retain enough lilac shadow to show strand separation.

### Bow

- Four butterfly wings remain readable at desktop scale.
- Center crystal is visible.
- Bow must feel attached to the hairstyle, not like a floating polygon prop.

## Current V3 tooling

Generate the isolated head:

```powershell
./tools/blender/build_sena_v3_head.ps1
```

Render the approval views:

```powershell
./tools/blender/render_sena_head_previews.ps1
```

Outputs remain local under:

```text
pets/sena/models/generated/
  sena_v3_head.blend
  sena_v3_head.glb
  previews_v3_head/
    front.png
    three_quarter.png
    side.png
```

The V3 head generator is deliberately isolated from the full character so a
failed head revision cannot destabilize the V1/V2 rig/runtime work.
