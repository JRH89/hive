# Packages Windows release binaries into <out>/hive-<target>.zip plus a
# .sha256 file in sha256sum format (same naming as package.sh).
#   pwsh scripts/package.ps1 x86_64-pc-windows-msvc dist
param([Parameter(Mandatory)][string]$Target, [Parameter(Mandatory)][string]$Out)
$ErrorActionPreference = 'Stop'
$name = "hive-$Target"
$stage = Join-Path $Out $name
Remove-Item -Recurse -Force $stage -ErrorAction SilentlyContinue
New-Item -ItemType Directory -Force $stage | Out-Null
Copy-Item "target/$Target/release/hive.exe", "target/$Target/release/hived.exe", README.md, LICENSE $stage
Copy-Item examples/config.toml (Join-Path $stage 'config.example.toml')
Copy-Item -Recurse docs $stage
$zip = Join-Path $Out "$name.zip"
Remove-Item -Force $zip -ErrorAction SilentlyContinue
Compress-Archive -Path $stage -DestinationPath $zip
Remove-Item -Recurse -Force $stage
$hash = (Get-FileHash -Algorithm SHA256 $zip).Hash.ToLower()
Set-Content -NoNewline -Path "$zip.sha256" -Value "$hash  $name.zip`n"
Write-Host $zip
