param(
    [string]$Blend = "pets/sena/models/generated/sena_v3_head.blend",
    [string]$Output = "pets/sena/models/generated/previews_v3_head",
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
if (-not (Test-Path $Blend)) {
    throw "Blend file not found: $Blend"
}

$script = Join-Path $PSScriptRoot "render_sena_head_previews.py"
& $Blender --background $Blend --python $script -- --output $Output
if ($LASTEXITCODE -ne 0) {
    throw "Head preview render failed with exit code $LASTEXITCODE."
}

foreach ($file in @("front.png", "three_quarter.png", "side.png")) {
    if (-not (Test-Path (Join-Path $Output $file))) {
        throw "Missing head preview: $file"
    }
}

Write-Host "Sena V3 head review views rendered to $Output"
