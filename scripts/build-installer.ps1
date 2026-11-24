# Builds the release and the Windows installer: target\StarPlay-<version>-setup.exe
# Usage: powershell -ExecutionPolicy Bypass -File scripts/build-installer.ps1 [-Makensis <path to makensis.exe>]
# Needs NSIS 3 (https://nsis.sourceforge.io) and the generic libmpv (scripts/get-mpv.ps1, default variant).
param([string]$Makensis = 'makensis')
$ErrorActionPreference = 'Stop'
$root = Split-Path $PSScriptRoot -Parent
Push-Location $root
try {
    $version = (Select-String -Path Cargo.toml -Pattern '^version\s*=\s*"([^"]+)"').Matches[0].Groups[1].Value
    cargo build --release
    if ($LASTEXITCODE -ne 0) { throw 'cargo build failed' }
    $src = Join-Path $root 'target\release'
    & $Makensis /V2 "/DVERSION=$version" "/DSRC=$src" installer\starplay.nsi
    if ($LASTEXITCODE -ne 0) { throw 'makensis failed' }
    Write-Host "Installer: target\StarPlay-$version-setup.exe"
} finally { Pop-Location }
