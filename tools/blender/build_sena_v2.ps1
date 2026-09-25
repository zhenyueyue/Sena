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
        if (-not $Blender -and (Test-Path $root)) {
            $candidate = Get-ChildItem $root -Recurse -Filter blender.exe -ErrorAction SilentlyContinue |
                Sort-Object FullName -Descending |
                Select-Object -First 1
            if ($candidate) {
                $Blender = $candidate.FullName
            }
        }
    }
}

if (-not $Blender -or -not (Test-Path $Blender)) {
    throw "Blender was not found. Install Blender 4.x/5.x or pass -Blender."
}

$script = Join-Path $PSScriptRoot "generate_sena_v2.py"
$blendOutput = Join-Path $Output "sena_v2.blend"
$glbOutput = Join-Path $Output "sena_v2.glb"

Remove-Item $blendOutput, $glbOutput -Force -ErrorAction SilentlyContinue

Write-Host "Generating Sena V2 anime head prototype..."
Write-Host "  Blender: $Blender"
Write-Host "  Output : $Output"

& $Blender --background --python $script -- --output $Output
if ($LASTEXITCODE -ne 0) {
    throw "Blender generation failed with exit code $LASTEXITCODE."
}

if (-not (Test-Path $blendOutput) -or -not (Test-Path $glbOutput)) {
    throw "Blender exited without producing sena_v2.blend and sena_v2.glb."
}

Write-Host "Sena V2 generation completed."
Write-Host "  BLEND: $blendOutput"
Write-Host "  GLB  : $glbOutput"
