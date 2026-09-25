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
    $roots = @(
        "C:\Program Files\Blender Foundation",
        "D:\Blender",
        "D:\Program Files\Blender Foundation"
    )
    foreach ($root in $roots) {
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

if (-not $Blender) {
    $uninstallKeys = @(
        "HKLM:\SOFTWARE\Microsoft\Windows\CurrentVersion\Uninstall\*",
        "HKLM:\SOFTWARE\WOW6432Node\Microsoft\Windows\CurrentVersion\Uninstall\*",
        "HKCU:\SOFTWARE\Microsoft\Windows\CurrentVersion\Uninstall\*"
    )
    foreach ($key in $uninstallKeys) {
        $install = Get-ItemProperty $key -ErrorAction SilentlyContinue |
            Where-Object { $_.DisplayName -like "Blender*" -and $_.InstallLocation } |
            Select-Object -First 1
        if ($install) {
            $candidate = Join-Path $install.InstallLocation "blender.exe"
            if (Test-Path $candidate) {
                $Blender = $candidate
                break
            }
        }
    }
}

if (-not $Blender -or -not (Test-Path $Blender)) {
    throw "Blender was not found. Install Blender 4.x or pass -Blender 'C:\path\to\blender.exe'."
}

$script = Join-Path $PSScriptRoot "generate_sena_v1.py"

Write-Host "Generating Sena chibi 3D V1 with:"
Write-Host "  Blender: $Blender"
Write-Host "  Output : $Output"

$blendOutput = Join-Path $Output "sena_v1.blend"
$glbOutput = Join-Path $Output "sena_v1.glb"

Remove-Item $blendOutput, $glbOutput -Force -ErrorAction SilentlyContinue

& $Blender --background --python $script -- --output $Output
if ($LASTEXITCODE -ne 0) {
    throw "Blender generation failed with exit code $LASTEXITCODE."
}

# Blender may still return zero when a background Python script raises. Treat
# missing expected artifacts as a hard failure instead of reporting success.
if (-not (Test-Path $blendOutput) -or -not (Test-Path $glbOutput)) {
    throw "Blender exited without producing sena_v1.blend and sena_v1.glb. Check the Python traceback above."
}

Write-Host "Sena V1 generation completed."
Write-Host "  BLEND: $blendOutput"
Write-Host "  GLB  : $glbOutput"
