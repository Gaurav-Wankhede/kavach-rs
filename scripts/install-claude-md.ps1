# Install the global engineering directives to the user-global Claude config path.
# Path is derived from $env:USERPROFILE at runtime — nothing is hardcoded. Windows.
$ErrorActionPreference = 'Stop'

$scriptDir = Split-Path -Parent $MyInvocation.MyCommand.Path
$src = Join-Path $scriptDir '..\assets\claude\CLAUDE.global.md'

if (-not (Test-Path $src)) {
    Write-Error "source not found: $src"
    exit 1
}

# NB: $HOME is a read-only automatic variable in PowerShell — assigning to it
# throws. Use a distinct name for the resolved profile path.
$userProfile = $env:USERPROFILE
if ([string]::IsNullOrEmpty($userProfile)) {
    Write-Error 'USERPROFILE is not set'
    exit 1
}

$destDir = Join-Path $userProfile '.claude'
$dest = Join-Path $destDir 'CLAUDE.md'

New-Item -ItemType Directory -Force -Path $destDir | Out-Null
Copy-Item -Path $src -Destination $dest -Force

Write-Output "installed global CLAUDE.md -> $dest"
