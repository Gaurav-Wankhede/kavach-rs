<#
.SYNOPSIS
    Install the Kavach harness on Windows from a published GitHub Release.

.DESCRIPTION
    Downloads the prebuilt kavach.exe (no Rust toolchain needed), puts it on PATH,
    and wires the native hook edges for Claude Code, Cursor, and Codex against one
    shared kavach-db. Optionally provisions the Rust CLI toolbelt.

    ARCHITECTURE NOTE (honest limit): the kavach-rpc daemon is Unix-socket only.
    On Windows there is no daemon; every gate/db call opens SurrealDB directly
    (the CLI's built-in fallback). Gates, memory, kanban, the 3-witness stop loop
    and the mistake ledger all WORK — you only lose the single-writer daemon
    multiplexing. Functionally complete, architecturally degraded vs macOS.

.PARAMETER Arch
    CPU arch of this machine: amd64 (default) or arm64.

.PARAMETER Harnesses
    Which harnesses to wire. Default: all three. e.g. -Harnesses claude,cursor

.PARAMETER NoToolbelt
    Skip `kavach toolbelt install`.

.PARAMETER Repo
    owner/repo to pull the release from. Default: Wankhede-Brothers/kavach-rs.

.EXAMPLE
    .\install-kavach-windows.ps1
    .\install-kavach-windows.ps1 -Arch arm64 -Harnesses claude,codex -NoToolbelt
#>

[CmdletBinding()]
param(
    [ValidateSet('amd64', 'arm64')]
    [string]$Arch = 'amd64',

    [ValidateSet('claude', 'cursor', 'codex')]
    [string[]]$Harnesses = @('claude', 'cursor', 'codex'),

    [switch]$NoToolbelt,

    [string]$Repo = 'Wankhede-Brothers/kavach-rs'
)

$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest

function Write-Step($n, $msg) { Write-Host "`n[$n] $msg" -ForegroundColor Cyan }
function Write-Ok($msg)       { Write-Host "  OK  $msg" -ForegroundColor Green }
function Write-Warn2($msg)    { Write-Host "  !!  $msg" -ForegroundColor Yellow }

# TLS 1.2 for older PowerShell 5.1 hosts (Invoke-WebRequest defaults can be SSL3).
[Net.ServicePointManager]::SecurityProtocol = [Net.SecurityProtocolType]::Tls12

# --- paths -----------------------------------------------------------------
$installDir = Join-Path $env:LOCALAPPDATA 'kavach'
$exePath    = Join-Path $installDir 'kavach.exe'
$claudeDir  = Join-Path $env:USERPROFILE '.claude'
$cursorDir  = Join-Path $env:USERPROFILE '.cursor'
$codexDir   = Join-Path $env:USERPROFILE '.codex'

Write-Host "========================================" -ForegroundColor Cyan
Write-Host " Kavach — Windows install ($Arch)" -ForegroundColor Cyan
Write-Host "========================================" -ForegroundColor Cyan

# --- 1. download prebuilt kavach.exe from the latest release ---------------
Write-Step 1 "Downloading kavach.exe ($Arch) from latest release of $Repo"

$asset = "kavach-windows-$Arch.zip"
$url   = "https://github.com/$Repo/releases/latest/download/$asset"
$tmp   = Join-Path $env:TEMP "kavach-dl"
New-Item -ItemType Directory -Force -Path $tmp | Out-Null
$zip   = Join-Path $tmp $asset

try {
    Invoke-WebRequest -Uri $url -OutFile $zip -UseBasicParsing
} catch {
    Write-Warn2 "Could not download $url"
    Write-Warn2 "A published release with Windows assets must exist first."
    Write-Warn2 "Check: https://github.com/$Repo/releases/latest"
    throw "Release asset '$asset' not reachable: $($_.Exception.Message)"
}
Write-Ok "Fetched $asset"

New-Item -ItemType Directory -Force -Path $installDir | Out-Null
Expand-Archive -Path $zip -DestinationPath $installDir -Force
if (-not (Test-Path $exePath)) {
    throw "kavach.exe not found in archive after extraction (expected $exePath)"
}
Write-Ok "Installed $exePath"

# --- 2. PATH ---------------------------------------------------------------
Write-Step 2 "Adding $installDir to user PATH"
$userPath = [Environment]::GetEnvironmentVariable('Path', 'User')
if ($userPath -notlike "*$installDir*") {
    [Environment]::SetEnvironmentVariable('Path', "$userPath;$installDir", 'User')
    Write-Ok "PATH updated (restart shells to pick it up)"
} else {
    Write-Ok "Already on PATH"
}
# Make kavach callable in THIS session.
$env:Path = "$installDir;$env:Path"

# Smoke-test the binary before wiring anything to it.
$ver = & $exePath --version 2>&1
Write-Ok "kavach reports: $ver"

# --- helper: deep-merge the kavach hooks block into an existing settings.json
function Merge-ClaudeHooks($settingsPath, $hooksObj) {
    $root = if (Test-Path $settingsPath) {
        Get-Content $settingsPath -Raw | ConvertFrom-Json
    } else {
        [pscustomobject]@{}
    }
    # Overwrite the whole `hooks` key — kavach owns the lifecycle wiring.
    $root | Add-Member -NotePropertyName 'hooks' -NotePropertyValue $hooksObj -Force
    $root | ConvertTo-Json -Depth 20 | Set-Content -Path $settingsPath -Encoding UTF8
}

# --- helper: resolve a repo template either from a local clone (next to this
# script) or, when the script was downloaded standalone, from the repo's raw
# default branch. Returns the file's text, or $null if neither source has it.
function Get-Template($relPath) {
    $local = Join-Path $PSScriptRoot (Join-Path '..' $relPath)
    if (Test-Path $local) { return Get-Content $local -Raw }
    $raw = "https://raw.githubusercontent.com/$Repo/main/$($relPath -replace '\\','/')"
    try {
        return (Invoke-WebRequest -Uri $raw -UseBasicParsing).Content
    } catch {
        Write-Warn2 "Template not found locally or at $raw"
        return $null
    }
}

# --- 3. Claude Code --------------------------------------------------------
if ($Harnesses -contains 'claude') {
    Write-Step 3 "Wiring Claude Code (~/.claude)"
    New-Item -ItemType Directory -Force -Path $claudeDir | Out-Null

    $hooks = [ordered]@{
        UserPromptSubmit = @(@{ hooks = @(@{ type = 'command'; command = 'kavach gates intent --hook' }) })
        PreToolUse  = @(
            @{ matcher = 'Write|Edit|NotebookEdit'; hooks = @(@{ type = 'command'; command = 'kavach gates pre-write --hook' }) },
            @{ matcher = '*';                       hooks = @(@{ type = 'command'; command = 'kavach gates pre-tool --hook' }) }
        )
        PostToolUse = @(
            @{ matcher = 'Write|Edit|NotebookEdit'; hooks = @(@{ type = 'command'; command = 'kavach gates post-write --hook' }) },
            @{ matcher = '*';                       hooks = @(@{ type = 'command'; command = 'kavach gates post-tool --hook' }) }
        )
        SessionStart = @(@{ hooks = @(@{ type = 'command'; command = 'kavach gates session-start --hook' }) })
        Stop         = @(@{ hooks = @(@{ type = 'command'; command = 'kavach gates stop --hook' }) })
    }
    Merge-ClaudeHooks (Join-Path $claudeDir 'settings.json') $hooks
    Write-Ok "Merged 7 gates into ~/.claude/settings.json"

    # Global rules: install the engineering-directives CLAUDE.md only if the user
    # has none (never clobber an existing one).
    $dstClaudeMd = Join-Path $claudeDir 'CLAUDE.md'
    if (Test-Path $dstClaudeMd) {
        Write-Warn2 "~/.claude/CLAUDE.md already exists — left as-is"
    } else {
        $md = Get-Template 'transfer-package\CLAUDE.md'
        if ($md) {
            Set-Content -Path $dstClaudeMd -Value $md -Encoding UTF8
            Write-Ok "Installed global CLAUDE.md"
        }
    }
}

# --- 4. Cursor -------------------------------------------------------------
if ($Harnesses -contains 'cursor') {
    Write-Step 4 "Wiring Cursor (~/.cursor)"
    New-Item -ItemType Directory -Force -Path $cursorDir | Out-Null
    New-Item -ItemType Directory -Force -Path (Join-Path $cursorDir 'rules') | Out-Null

    $cursorHooks = @{
        version = 1
        hooks = [ordered]@{
            beforeSubmitPrompt    = @(@{ command = 'kavach gates intent --hook --vendor cursor' })
            beforeShellExecution  = @(@{ command = 'kavach gates pre-tool --hook --vendor cursor'; failClosed = $true })
            beforeMCPExecution    = @(@{ command = 'kavach gates pre-tool --hook --vendor cursor'; failClosed = $true })
            beforeReadFile        = @(@{ command = 'kavach gates pre-tool --hook --vendor cursor' })
            afterFileEdit         = @(@{ command = 'kavach gates post-write --hook --vendor cursor' })
            stop                  = @(@{ command = 'kavach gates stop --hook --vendor cursor' })
        }
    }
    $cursorHooks | ConvertTo-Json -Depth 20 | Set-Content -Path (Join-Path $cursorDir 'hooks.json') -Encoding UTF8
    Write-Ok "Wrote ~/.cursor/hooks.json"

    $mdc = Get-Template 'crates\kavach-cli\templates\harness\kavach.mdc'
    if ($mdc) {
        Set-Content -Path (Join-Path $cursorDir 'rules\kavach.mdc') -Value $mdc -Encoding UTF8
        Write-Ok "Installed ~/.cursor/rules/kavach.mdc"
    }
}

# --- 5. Codex --------------------------------------------------------------
if ($Harnesses -contains 'codex') {
    Write-Step 5 "Wiring Codex (~/.codex/config.toml)"
    New-Item -ItemType Directory -Force -Path $codexDir | Out-Null
    $codexCfg = Join-Path $codexDir 'config.toml'
    $block = Get-Template 'crates\kavach-cli\templates\harness\codex.config.toml'

    if ($block) {
        $existing = if (Test-Path $codexCfg) { Get-Content $codexCfg -Raw } else { '' }
        if ($existing -match 'kavach gates') {
            Write-Warn2 "config.toml already references kavach — left as-is"
        } else {
            # Ensure hooks feature is on, then append the gate block.
            if ($existing -notmatch '(?ms)^\[features\][^\[]*?hooks\s*=\s*true') {
                Add-Content -Path $codexCfg -Value "`n[features]`nhooks = true`n" -Encoding UTF8
            }
            Add-Content -Path $codexCfg -Value "`n$block" -Encoding UTF8
            Write-Ok "Appended kavach hooks to ~/.codex/config.toml"
        }
        $dstAgents = Join-Path $codexDir 'AGENTS.md'
        if (-not (Test-Path $dstAgents)) {
            $agents = Get-Template 'crates\kavach-cli\templates\harness\AGENTS.md'
            if ($agents) {
                Set-Content -Path $dstAgents -Value $agents -Encoding UTF8
                Write-Ok "Installed ~/.codex/AGENTS.md"
            }
        }
    } else {
        Write-Warn2 "codex.config.toml template unavailable — skipped"
    }
}

# --- 6. Toolbelt -----------------------------------------------------------
if (-not $NoToolbelt) {
    Write-Step 6 "Provisioning the Rust CLI toolbelt"
    if (Get-Command cargo -ErrorAction SilentlyContinue) {
        if (-not (Get-Command cargo-binstall -ErrorAction SilentlyContinue)) {
            Write-Host "  Installing cargo-binstall first..." -ForegroundColor Gray
            cargo install cargo-binstall
        }
        & $exePath toolbelt install --yes
        Write-Ok "Toolbelt installed (kavach toolbelt list to audit)"
    } else {
        Write-Warn2 "cargo not found — toolbelt needs a Rust toolchain (https://rustup.rs)."
        Write-Warn2 "Skipping. Re-run later: kavach toolbelt install --yes"
    }
}

# --- 7. verify -------------------------------------------------------------
Write-Step 7 "Verifying"
& $exePath status

Write-Host "`n========================================" -ForegroundColor Green
Write-Host " Kavach install complete" -ForegroundColor Green
Write-Host "========================================" -ForegroundColor Green
Write-Host "  Binary : $exePath" -ForegroundColor Gray
Write-Host "  Wired  : $($Harnesses -join ', ')" -ForegroundColor Gray
Write-Host "  Note   : daemon is Unix-only; Windows uses the direct-DB fallback." -ForegroundColor Gray
Write-Host "`nRestart your terminal so PATH takes effect, then run: claude" -ForegroundColor White
