# Downloads libmpv (shinchiro's Windows build) into vendor/mpv and generates mpv.lib for the MSVC linker.
# Usage: powershell -ExecutionPolicy Bypass -File scripts/get-mpv.ps1 [-Variant x86_64-v3]
# x86_64 (default) works on all x64 CPUs; x86_64-v3 requires AVX2 (slightly faster, not for a release).
param([ValidateSet('x86_64', 'x86_64-v3')] [string]$Variant = 'x86_64')
$ErrorActionPreference = 'Stop'
$tag  = '20261111'
$file = "mpv-dev-$Variant-20261111-git-bcde6b93cc.7z"
$url  = "https://github.com/shinchiro/mpv-winbuild-cmake/releases/download/$tag/$file"

$root = Split-Path $PSScriptRoot -Parent
$dest = Join-Path $root 'vendor\mpv'
New-Item -ItemType Directory -Force $dest | Out-Null
$archive = Join-Path $dest $file

Invoke-WebRequest $url -OutFile $archive
& "$env:SystemRoot\System32\tar.exe" -xf $archive -C $dest   # Windows' tar reads the 7z
Remove-Item $archive

# MSVC import library built from the DLL exports
$vswhere = "${env:ProgramFiles(x86)}\Microsoft Visual Studio\Installer\vswhere.exe"
$vs = & $vswhere -latest -products * -requires Microsoft.VisualStudio.Component.VC.Tools.x86.x64 -property installationPath
$bin = Get-ChildItem "$vs\VC\Tools\MSVC\*\bin\Hostx64\x64" | Select-Object -Last 1
$exports = & "$bin\dumpbin.exe" /nologo /exports "$dest\libmpv-2.dll"
$names = $exports | ForEach-Object { if ($_ -match '^\s+\d+\s+[0-9A-F]+\s+[0-9A-F]+\s+(mpv_\w+)') { "    $($Matches[1])" } }
Set-Content "$dest\mpv.def" (@('LIBRARY libmpv-2', 'EXPORTS') + $names) -Encoding ascii
& "$bin\lib.exe" /nologo /def:"$dest\mpv.def" /machine:x64 /out:"$dest\mpv.lib"
Write-Host "libmpv ready in $dest ($($names.Count) functions)"
