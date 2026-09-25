# Sena Q版 3D Model Guide

## Goal

The primary long-term renderer is a lightweight chibi 3D character that can move along the bottom of the desktop like a game companion. Sprite remains the fallback renderer until the 3D path is production-ready.

## V1 visual target

- Approx. 3.0 heads tall (2.8–3.3 acceptable).
- Large head and eyes, compact torso, short limbs, readable hands/feet.
- Moonlight silver-white hair with faint pink/lilac tint.
- Large translucent lavender butterfly/crystal bow remains the strongest silhouette landmark.
- Simplified white/lilac/ice-blue crystal dress; preserve layered ribbon language without reproducing every illustration detail.
- Cream/orange-and-white chibi cat, rounded enough to be comfortably carried by Sena.
- Toon/anime shading. Avoid realistic skin, heavy PBR, complex environment lighting and expensive post-processing.

## V2 art rebuild

`generate_sena_v1.py` remains the technical blockout used to validate rigging, animation names, attachments and runtime rendering. It is not the final visual model.

`tools/blender/generate_sena_v2.py` is the visual rebuild path. V2 currently replaces the head system with a custom tapered anime head mesh, nearly-flat layered eyes, tapered ribbon-based hair locks and a four-wing butterfly bow while reusing the proven V1 body/rig/action contract. This lets visual quality improve without destabilizing animation/runtime work.

Build and review V2 with:

```powershell
./tools/blender/build_sena_v2.ps1
./tools/blender/render_sena_previews.ps1 -Blend pets/sena/models/generated/sena_v2.blend -Output pets/sena/models/generated/previews_v2
```

V2 now also replaces the bead-like body with one voxel-fused, watertight `BodyMeshV2`. A controllable torso/limb proxy is fused at 12 mm voxel resolution, smoothed, then receives deterministic spatial humanoid weights for the existing Hips/Spine/Chest/arm/leg bones. Compact crystal shoes remain separate so footwear can keep its own material silhouette. The V2 `Walk` clip is retuned for the continuous body with smaller leg/arm arcs, knee motion, body bob and preserved hair/bow lag; the wider V1 blockout walk is not reused blindly.

The next visual milestones are layered skirt geometry and a rebuilt cat. Do not promote V2 to the production `sena.vrm` slot until the front, three-quarter, side and representative motion reviews all pass.

## V3 head approval gate

The visual-quality rebuild has moved to an isolated V3 head workflow. See [V3_HEAD_APPROVAL.md](V3_HEAD_APPROVAL.md). V3 does **not** inherit V2 visual meshes as an assumption of quality: it rebuilds the face/head/hair/bow separately and must pass front, three-quarter and side approval before any V3 body/costume work begins. V1/V2 remain technical references and runtime test assets.

## Runtime budget

V1 target, not a hard file-format restriction:

- Sena: about 30k–60k triangles.
- Cat: about 5k–15k triangles.
- Total visible character/props target: below ~100k triangles.
- Body/outfit texture: 2K maximum for V1.
- Face/hair: 1K–2K where needed.
- Cat/props: 512–1K.
- Prefer opaque/cutout materials. Reserve alpha blending for a small number of crystal/chiffon accents.

Runtime policy:

- Static/quiet: event-driven or low refresh; do not hold 60 FPS just because the model exists.
- Breathing/blink: low-cost bone/blendshape updates.
- Active locomotion/actions: up to 60 FPS rendering, animation may run at 30 Hz with interpolation.
- Spring bone/secondary motion: target ~30 Hz first; reduce for power-saving mode.
- Hidden/locked: stop 3D rendering.

## Required humanoid structure

Use a standard humanoid skeleton compatible with VRM/glTF animation retargeting. At minimum:

- Hips / Spine / Chest / Neck / Head
- Left/Right UpperArm / LowerArm / Hand
- Left/Right UpperLeg / LowerLeg / Foot

Hair, bow, skirt/ribbons and cat secondary bones may be custom.

Current spring-ready secondary-motion bone contract:

- `HairBackL`, `HairBackC`, `HairBackR`
- `HairSideL`, `HairSideR`
- `BowRoot`, `BowTailL`, `BowTailR`

The authored `Idle` and `Walk` clips contain a small secondary-motion fallback so exported GLB previews do not look rigid. The runtime 3D renderer may later layer a lightweight spring solver over these same bones (target ~30 Hz) instead of inventing another naming scheme.

## Required V1 motion names

The package contract in `pet.template.json` maps semantic names to embedded animation clips. The first production model should provide:

- `Idle`
- `Walk`
- `TurnLeft`
- `TurnRight`
- `SitDown`
- `SitIdle`
- `StandUp`
- `Stretch`
- `Petting`
- `LookAtCat`
- `Daydream`
- `CarryCatPickup`
- `CarryCatIdle`
- `CarryCatWalk`
- `CarryCatPutdown`

Transitions should be blendable; avoid animation clips that begin several frames away from the shared neutral pose unless the action explicitly requires it.

## Expressions

Minimum expression/blendshape contract:

- `BlinkLeft`
- `BlinkRight`
- `Happy`
- `Curious`
- `Sleepy`
- `Focused`

Blink left/right must be independently controllable so Sena and the cat do not look mechanically synchronized.

## Attachment anchors

Create named bones/nodes even if the props are not shipped in V1:

- `CatCarry` — cat root while being held.
- `RightHandProp`
- `LeftHandProp`
- `Headphones`
- `Laptop`

The cat remains an independently animated entity when on the ground. During pickup, blend into a carry pose and attach its root to `CatCarry`; do not teleport it visibly between ground and arms.

## Desktop locomotion rules

The 3D model should support game-like transitions:

`Idle -> Turn -> Walk -> Decelerate -> Idle`

and:

`Idle -> WalkToCat -> CarryCatPickup -> CarryCatIdle/Walk -> CarryCatPutdown -> Idle`

Do not implement left movement by simply mirroring the rendered image. The character should rotate/turn in 3D and then walk in the new facing direction.

## Asset layout

Planned layout:

```text
pets/sena/
  models/
    sena.vrm
    cat.glb            # optional if cat is not embedded
    props/
      headphones.glb
      laptop.glb
```

`.vrm` is preferred for Sena because it gives us a humanoid/avatar convention; `.glb` remains supported for generic model/prop assets.

## Windows runtime probe

The repository includes a native Win32 + wgpu GLB probe. Generate the local model first, then run:

```powershell
./tools/blender/build_sena_v1.ps1
cargo run --example sena_3d_preview
```

The preview loads `pets/sena/models/generated/sena_v1.glb`, creates a borderless top-most Win32 window, renders through wgpu and requests premultiplied surface alpha. Press `Esc` after clicking the preview window to close it. Automated smoke tests can use `--frames 120`.

Backend override for compatibility checks:

```powershell
$env:SENA_WGPU_BACKEND = "dx12"
cargo run --example sena_3d_preview -- --frames 120
```

`vulkan` is also accepted. On the current Windows test machine, Vulkan exposes `PreMultiplied` surface alpha and renders the transparent path successfully; the ordinary DX12 HWND surface exposes only `Opaque`. Production must therefore capability-check alpha and fall back to Sprite rather than showing an opaque 3D window. A DirectComposition/DXGI DX12 transparency path can be added separately for machines without a suitable Vulkan surface.

The probe intentionally renders the current static GLB node transforms first; skeletal skinning, animation blending and spring simulation are the next runtime layer rather than being hidden inside this proof-of-rendering step.

## Migration rule

Do not delete the current Sprite assets. They remain the safe fallback until the Model3d renderer passes transparency, hit-testing, locomotion and power-usage acceptance tests on Windows.
