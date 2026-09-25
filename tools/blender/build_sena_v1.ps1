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

if (-not $Blender -and (Test-Path "C:\Program Files\Blender Foundation")) {
    $candidate = Get-ChildItem "C:\Program Files\Blender Foundation" -Recurse -Filter blender.exe -ErrorAction SilentlyContinue |
        Sort-Object FullName -Descending |
        Select-Object -First 1
    if ($candidate) {
        $Blender = $candidate.FullName
    }
}

if (-not $Blender -or -not (Test-Path $Blender)) {
    throw "Blender was not found. Install Blender 4.x or pass -Blender 'C:\path\to\blender.exe'."
}

$script = Join-Path $PSScriptRoot "generate_sena_v1.py"

Write-Host "Generating Sena chibi 3D V1 with:"
Write-Host "  Blender: $Blender"
Write-Host "  Output : $Output"

& $Blender --background --python $script -- --output $Output
if ($LASTEXITCODE -ne 0) {
    throw "Blender generation failed with exit code $LASTEXITCODE."
}

Write-Host "Sena V1 generation completed."
