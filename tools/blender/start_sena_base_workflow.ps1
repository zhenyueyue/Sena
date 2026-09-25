param(
    [string]$Blender = "D:\Blender\blender.exe",
    [string]$VRoidStudio = "",
    [switch]$NoLaunch
)

$ErrorActionPreference = "Stop"

$projectRoot = (Resolve-Path (Join-Path $PSScriptRoot "..\..")).Path
$candidateDir = Join-Path $projectRoot "pets\sena\models\base_candidates"

if (-not (Test-Path $Blender)) {
    throw "Blender not found: $Blender"
}

if (-not $VRoidStudio) {
    $uninstallKey = "HKCU:\SOFTWARE\Microsoft\Windows\CurrentVersion\Uninstall\*"
    $registered = Get-ItemProperty $uninstallKey -ErrorAction SilentlyContinue |
        Where-Object { $_.DisplayName -match "^VRoidStudio" } |
        Select-Object -First 1

    if ($registered -and $registered.InstallLocation) {
        $candidateExe = Join-Path $registered.InstallLocation "VRoidStudio.exe"
        if (Test-Path $candidateExe) {
            $VRoidStudio = $candidateExe
        }
    }
}

if (-not $VRoidStudio) {
    foreach ($candidateExe in @(
        "D:\VRoidStudio\VRoidStudio.exe",
        "C:\Program Files\VRoidStudio\VRoidStudio.exe"
    )) {
        if (Test-Path $candidateExe) {
            $VRoidStudio = $candidateExe
            break
        }
    }
}

if (-not $VRoidStudio -or -not (Test-Path $VRoidStudio)) {
    throw "VRoid Studio executable was not found."
}

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
    throw "Blender VRM Add-on is not installed/enabled."
}

New-Item -ItemType Directory -Path $candidateDir -Force | Out-Null

$version = (Get-Item $VRoidStudio).VersionInfo.ProductVersion
Write-Host "Sena Base Mesh workflow ready."
Write-Host "  Blender     : $Blender"
Write-Host "  VRM Add-on  : enabled"
Write-Host "  VRoid Studio: $VRoidStudio"
Write-Host "  Version     : $version"
Write-Host "  Export to   : $candidateDir\sena_base_01.vrm"
Write-Host ""
Write-Host "Keep hair and clothes simple. Judge face and base proportions first."

if (-not $NoLaunch) {
    Start-Process -FilePath $VRoidStudio
    Start-Process explorer.exe -ArgumentList $candidateDir
    Write-Host ""
    Write-Host "VRoid Studio and the candidate folder have been opened."
}
