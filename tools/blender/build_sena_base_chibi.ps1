param(
    [string]$SourceBlend = "pets/sena/models/generated/base_review_01/imported.blend",
    [string]$Output = "pets/sena/models/generated/base_chibi_01",
    [string]$Blender = "D:\Blender\blender.exe"
)

$ErrorActionPreference = "Stop"

if (-not (Test-Path $Blender)) {
    throw "Blender not found: $Blender"
}
if (-not (Test-Path $SourceBlend)) {
    throw "Imported VRoid blend not found: $SourceBlend"
}

if (Test-Path $Output) {
    Remove-Item $Output -Recurse -Force
}

$script = Join-Path $PSScriptRoot "stylize_sena_base.py"

& $Blender --background $SourceBlend --python $script -- --output $Output
if ($LASTEXITCODE -ne 0) {
    throw "Sena chibi base build failed with exit code $LASTEXITCODE."
}

foreach ($file in @(
    "sena_base_chibi_01.blend",
    "report.json",
    "front.png",
    "three_quarter.png",
    "side.png"
)) {
    if (-not (Test-Path (Join-Path $Output $file))) {
        throw "Missing chibi-base output: $file"
    }
}

Write-Host ""
Write-Host "Sena first chibi base pass completed:"
Write-Host "  $Output"
Get-Content (Join-Path $Output "report.json")
