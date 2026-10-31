# Smoke-tests built Windows binaries against a private daemon:
#   pwsh scripts/smoke.ps1 target/release
param([Parameter(Mandatory)][string]$Bin)
$ErrorActionPreference = 'Stop'
$bin = (Resolve-Path $Bin).Path
$tmp = Join-Path ([IO.Path]::GetTempPath()) "hive-smoke-$PID"
New-Item -ItemType Directory -Force $tmp | Out-Null
$env:HIVE_RUNTIME_DIR = "$tmp\rt"
$env:HIVE_STATE_DIR = "$tmp\state"
$env:HIVE_CONFIG = "$tmp\config.toml"
$env:SHELL = 'cmd.exe'

function hive {
    $out = & "$bin\hive.exe" @args 2>&1 | Out-String
    if ($LASTEXITCODE -ne 0) { throw "hive $args failed ($LASTEXITCODE): $out" }
    Write-Host $out.TrimEnd()
    $out
}

try {
    hive --version | Out-Null
    hive doctor | Out-Null
    hive new shell -d -l smoke | Out-Null
    if ((hive list) -notmatch 'smoke') { throw 'session missing from list' }
    if ((hive status) -notmatch '1 active') { throw 'status does not count the session' }
    hive kill 1 | Out-Null
    if ((hive list) -notmatch 'no sessions') { throw 'session not removed' }
    hive daemon stop | Out-Null
    Write-Host 'smoke test passed'
} finally {
    & "$bin\hive.exe" daemon stop 2>&1 | Out-Null
    Remove-Item -Recurse -Force $tmp -ErrorAction SilentlyContinue
}
