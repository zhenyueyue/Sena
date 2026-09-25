# Sena Base Mesh Pipeline

The procedural V1/V2/V3 models proved the rig/runtime architecture, but they are
not the final art model.  The production visual model now starts from an
existing high-quality anime base whose face/body proportions are already
artistically convincing.

## Preferred source

The preferred first source is **VRoid Studio**:

- create a clean original anime base using only VRoid/pixiv content whose item
  license has no extra restrictive clause;
- export as VRM;
- import into Blender;
- evaluate the naked/base silhouette before adding Sena-specific hair, dress or
  accessories;
- record source/version/license details in the candidate metadata file.

Do not assume VRoid's bundled base meshes are CC0.  Keep the source/license
record with the project even when the resulting model is heavily modified.

## Local candidate directory

Candidate binaries are intentionally not committed:

```text
pets/sena/models/base_candidates/
  candidate_name.vrm
  candidate_name.license.json
```

This prevents a third-party or tool-provided base model from being accidentally
published in the source repository.

## Candidate gate

A base is accepted only if the *unmodified* model already looks good in:

1. front view;
2. three-quarter view;
3. true side view.

Reject the candidate before any Sena work if:

- the face only looks good from the front;
- the eyes protrude badly in profile;
- the jaw/cheeks need major reconstruction;
- shoulders/hips are visually awkward;
- topology is obviously broken;
- rigging is unsuitable for humanoid animation;
- licensing is unclear.

The point of the base is to avoid another months-long procedural sculpt loop.

## Automated inspection

VRM requires the VRM Add-on for Blender. GLB can be inspected with stock Blender.

```powershell
./tools/blender/inspect_sena_base.ps1 -Model "pets/sena/models/base_candidates/base.vrm"
```

The tool writes:

```text
pets/sena/models/generated/base_review/
  report.json
  front.png
  three_quarter.png
  side.png
  imported.blend
```

The report records mesh/triangle/material/armature/bone counts, overall bounds,
candidate file hash and a coarse humanoid-bone check.

## After approval

Only after the base passes the gate:

1. duplicate the accepted base into the Sena production working file;
2. move the body toward ~3-head-tall chibi proportions;
3. sculpt the face against the approved Sena concept;
4. replace hair with Sena's moonlight-silver style;
5. build the crystal butterfly bow and layered dress;
6. retarget/rebind to the existing Sena runtime animation contract;
7. export VRM/GLB and run the existing runtime probe.

V1/V2/V3 remain technical references; they are not promoted to production art.
