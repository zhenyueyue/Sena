param(
    [string]$Blend = "pets/sena/models/generated/sena_v1.blend",
    [string]$Output = "pets/sena/models/generated/previews",
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
    foreach ($root in @("D:\Blender", "C:\Program Files\Blender Foundation", "D:\Program Files\Blender Foundation")) {
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

$script = Join-Path $PSScriptRoot "render_sena_previews.py"
& $Blender --background $Blend --python $script -- --output $Output

$expected = @("front.png", "three_quarter.png", "side.png")
foreach ($file in $expected) {
    if (-not (Test-Path (Join-Path $Output $file))) {
        throw "Preview render did not produce $file."
    }
}

Write-Host "Sena review views rendered to $Output"
