param(
    [Parameter(Mandatory = $true)]
    [string]$Model,

    [string]$Output = "pets/sena/models/generated/base_review",
    [string]$Blender = ""
)

$ErrorActionPreference = "Stop"

if (-not $Blender) {
    $command = Get-Command blender.exe -ErrorAction SilentlyContinue
    if ($command) {
        $Blender = $command.Source
    }
}

if (-not $Blender) {
    foreach ($root in @(
        "D:\Blender",
        "C:\Program Files\Blender Foundation",
        "D:\Program Files\Blender Foundation"
    )) {
        if (Test-Path $root) {
            $candidate = Get-ChildItem $root -Recurse -Filter blender.exe -ErrorAction SilentlyContinue |
                Sort-Object FullName -Descending |
                Select-Object -First 1
            if ($candidate) {
                $Blender = $candidate.FullName
                break
            }
        }
    }
}

if (-not $Blender -or -not (Test-Path $Blender)) {
    throw "Blender was not found."
}

if (-not (Test-Path $Model)) {
    throw "Candidate model not found: $Model"
}

$extension = [IO.Path]::GetExtension($Model).ToLowerInvariant()
if ($extension -eq ".vrm") {
    $probeScript = Join-Path $env:TEMP "sena_vrm_operator_probe.py"
    @'
import bpy
try:
    bpy.ops.import_scene.vrm.get_rna_type()
    print("SENA_VRM_OPERATOR=1")
except (AttributeError, RuntimeError):
    print("SENA_VRM_OPERATOR=0")
'@ | Set-Content -Path $probeScript -Encoding UTF8

    try {
        $probe = & $Blender --background --python $probeScript 2>&1
    }
    finally {
        Remove-Item $probeScript -Force -ErrorAction SilentlyContinue
    }

    if (-not ($probe -match "SENA_VRM_OPERATOR=1")) {
        throw @"
VRM Add-on for Blender is not installed/enabled.

Blender 5.2:
  Edit -> Preferences -> Get Extensions
  Search: VRM
  Install/enable: VRM format

Then run this command again.
"@
    }
}

$script = Join-Path $PSScriptRoot "inspect_sena_base.py"

if (Test-Path $Output) {
    Remove-Item $Output -Recurse -Force
}

& $Blender --background --python $script -- --model $Model --output $Output
if ($LASTEXITCODE -ne 0) {
    throw "Base model inspection failed with exit code $LASTEXITCODE."
}

$required = @("report.json", "front.png", "three_quarter.png", "side.png", "imported.blend")
foreach ($file in $required) {
    if (-not (Test-Path (Join-Path $Output $file))) {
        throw "Inspection did not produce $file."
    }
}

Write-Host ""
Write-Host "Sena base candidate review completed:"
Write-Host "  $Output"
Write-Host ""
Get-Content (Join-Path $Output "report.json")
