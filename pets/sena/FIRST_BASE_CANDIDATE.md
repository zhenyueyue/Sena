# Sena First Base Candidate

The first VRoid export is not Sena yet. Its only job is to prove that the
underlying anime face/body is attractive from multiple angles before we invest
in hair, dress, cat or runtime integration.

## Target

Create one clean female anime base with:

- soft young-adult face;
- smooth forehead and rounded cheeks;
- small tapered chin;
- large but not circular toy-like eyes;
- a side profile that still looks attractive;
- narrow/soft shoulders and compact proportions;
- simple short or medium hair that does not hide the cheeks/jaw;
- simple fitted/default clothing so body proportions remain visible;
- no butterfly bow, long fantasy hair, elaborate dress or accessories yet.

Do not spend time reproducing Sena's final costume in VRoid. The production
hair/dress will be rebuilt after the base passes review.

## Export

Use VRoid Studio's VRM export. Prefer VRM 1.0 for this pipeline.

Save the first candidate as:

~~~
pets/sena/models/base_candidates/sena_base_01.vrm
~~~

The candidate directory is ignored by Git.

## Review command

After export:

~~~powershell
./tools/blender/inspect_sena_base.ps1 -Model "pets/sena/models/base_candidates/sena_base_01.vrm"
~~~

Review front.png, three_quarter.png, side.png and report.json together.

## Decision rule

Do not fix it later if the naked/base model is already unattractive. Reject the
base if the side profile, eyes, cheeks/jaw, shoulders or humanoid rig need a
fundamental rebuild. If it passes, the next stage is Sena-ization in Blender:
chibi proportion adjustment, face sculpt, Sena hair, crystal butterfly bow and
layered costume.
