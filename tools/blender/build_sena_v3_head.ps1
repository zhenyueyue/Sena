param(
    [string]$Output = "pets/sena/models/generated",
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

$script = Join-Path $PSScriptRoot "generate_sena_v3_head.py"
$blendOutput = Join-Path $Output "sena_v3_head.blend"
$glbOutput = Join-Path $Output "sena_v3_head.glb"

Remove-Item $blendOutput, $glbOutput -Force -ErrorAction SilentlyContinue

Write-Host "Generating Sena V3 head-approval model..."
& $Blender --background --python $script -- --output $Output
if ($LASTEXITCODE -ne 0) {
    throw "Blender generation failed with exit code $LASTEXITCODE."
}

if (-not (Test-Path $blendOutput) -or -not (Test-Path $glbOutput)) {
    throw "Blender did not produce sena_v3_head.blend/glb."
}

Write-Host "Sena V3 head generated:"
Write-Host "  BLEND: $blendOutput"
Write-Host "  GLB  : $glbOutput"
