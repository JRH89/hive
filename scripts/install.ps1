# Installs hive and hived on Windows from a GitHub release.
#
#   irm https://github.com/jrh89/hive/releases/latest/download/install.ps1 | iex
#
# Environment:
#   HIVE_VERSION       release tag to install (default: latest), e.g. v0.4.0
#   HIVE_INSTALL_DIR   where to put the binaries (default: %LOCALAPPDATA%\hive\bin)
#   HIVE_DOWNLOAD_URL  base URL holding the release assets (overrides the above)
$ErrorActionPreference = 'Stop'
$ProgressPreference = 'SilentlyContinue'

function Say($msg) { Write-Host "hive-install: $msg" }

$repo = 'jrh89/hive'
$version = if ($env:HIVE_VERSION) { $env:HIVE_VERSION } else { 'latest' }
$dir = if ($env:HIVE_INSTALL_DIR) { $env:HIVE_INSTALL_DIR } else { Join-Path $env:LOCALAPPDATA 'hive\bin' }

# The machine's CPU, also from a 32-bit PowerShell. Not RuntimeInformation:
# Windows PowerShell 5.1 (the default on Windows 10 and 11) often can't load it.
$arch = if ($env:PROCESSOR_ARCHITEW6432) { $env:PROCESSOR_ARCHITEW6432 } else { $env:PROCESSOR_ARCHITECTURE }
$name = switch ($arch) {
    'AMD64' { 'hive-x86_64-pc-windows-msvc' }
    'ARM64' { 'hive-aarch64-pc-windows-msvc' }
    default { throw "no prebuilt binary for this CPU ($arch); build from source instead (see README)" }
}

$base = if ($env:HIVE_DOWNLOAD_URL) { $env:HIVE_DOWNLOAD_URL }
        elseif ($version -eq 'latest') { "https://github.com/$repo/releases/latest/download" }
        else { "https://github.com/$repo/releases/download/$version" }

$tmp = Join-Path ([IO.Path]::GetTempPath()) "hive-install-$PID"
New-Item -ItemType Directory -Force $tmp | Out-Null
try {
    Say "downloading $name ($version)"
    try {
        Invoke-WebRequest "$base/$name.zip" -OutFile "$tmp\$name.zip" -UseBasicParsing
    } catch {
        # Releases before 0.5.4 have no Arm build; Windows on Arm runs the
        # x64 one under emulation.
        if ($name -ne 'hive-aarch64-pc-windows-msvc') { throw }
        $name = 'hive-x86_64-pc-windows-msvc'
        Say "this release has no Arm build; installing $name, which Windows runs under emulation"
        Invoke-WebRequest "$base/$name.zip" -OutFile "$tmp\$name.zip" -UseBasicParsing
    }
    Invoke-WebRequest "$base/$name.zip.sha256" -OutFile "$tmp\$name.zip.sha256" -UseBasicParsing
    $want = ((Get-Content "$tmp\$name.zip.sha256" -Raw).Trim() -split '\s+')[0].ToLower()
    $got = (Get-FileHash -Algorithm SHA256 "$tmp\$name.zip").Hash.ToLower()
    if ($want -ne $got) { throw "checksum mismatch" }

    Expand-Archive -Force "$tmp\$name.zip" $tmp
    New-Item -ItemType Directory -Force $dir | Out-Null
    foreach ($exe in 'hive.exe', 'hived.exe') {
        $dest = Join-Path $dir $exe
        # A running binary can't be overwritten, but it can be renamed.
        if (Test-Path $dest) {
            Remove-Item -Force "$dest.old" -ErrorAction SilentlyContinue
            Move-Item -Force $dest "$dest.old"
        }
        Copy-Item (Join-Path "$tmp\$name" $exe) $dest
    }
    Say "installed hive and hived to $dir"

    $userPath = [Environment]::GetEnvironmentVariable('Path', 'User')
    if (-not (($userPath -split ';') -contains $dir)) {
        [Environment]::SetEnvironmentVariable('Path', "$dir;$userPath", 'User')
        Say "added $dir to your PATH (open a new terminal to use it)"
    }

    $status = & (Join-Path $dir 'hive.exe') daemon status 2>&1 | Out-String
    if ($status -match 'running \(pid') {
        Say "note: an older hived may still be running. 'hive daemon stop' restarts it (this ends running sessions)"
    }
    Say "run 'hive doctor' to check your setup"
} finally {
    Remove-Item -Recurse -Force $tmp -ErrorAction SilentlyContinue
}
