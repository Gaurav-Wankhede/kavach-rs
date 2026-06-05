# Kavach Tester Setup for Windows (Surface Pro 8)
# Run as Administrator: Right-click PowerShell -> Run as Administrator
# Usage: .\setup-kavach-tester.ps1

param(
    [string]$RepoUrl = "https://github.com/your-org/kavach-rs.git"
)

$ErrorActionPreference = "Stop"

Write-Host "========================================" -ForegroundColor Cyan
Write-Host "Kavach Tester Setup - Windows" -ForegroundColor Cyan
Write-Host "========================================" -ForegroundColor Cyan

# Check if running as Administrator
$isAdmin = ([Security.Principal.WindowsPrincipal] [Security.Principal.WindowsIdentity]::GetCurrent()).IsInRole([Security.Principal.WindowsBuiltInRole]::Administrator)
if (-not $isAdmin) {
    Write-Host "ERROR: Please run as Administrator" -ForegroundColor Red
    exit 1
}

# 1. Install prerequisites
Write-Host "`n[1/8] Installing prerequisites..." -ForegroundColor Yellow

# Check if winget is available
if (-not (Get-Command winget -ErrorAction SilentlyContinue)) {
    Write-Host "ERROR: winget not found. Please install App Installer from Microsoft Store." -ForegroundColor Red
    exit 1
}

# Install Rust
Write-Host "Installing Rust..." -ForegroundColor Gray
winget install Rustlang.Rustup -e --silent --accept-package-agreements --accept-source-agreements

# Install Visual Studio Build Tools
Write-Host "Installing Visual Studio Build Tools..." -ForegroundColor Gray
winget install Microsoft.VisualStudio.2022.BuildTools -e --silent --accept-package-agreements --accept-source-agreements

# Install Git
Write-Host "Installing Git..." -ForegroundColor Gray
winget install Git.Git -e --silent --accept-package-agreements --accept-source-agreements

# 2. Refresh PATH
Write-Host "`n[2/8] Refreshing PATH..." -ForegroundColor Yellow
$env:Path = [System.Environment]::GetEnvironmentVariable("Path", "Machine") + ";" + [System.Environment]::GetEnvironmentVariable("Path", "User")

# Add Cargo to PATH for this session
$cargoPath = "$env:USERPROFILE\.cargo\bin"
if (Test-Path $cargoPath) {
    $env:Path = "$cargoPath;$env:Path"
}

# 3. Clone repository
Write-Host "`n[3/8] Cloning kavach-rs repository..." -ForegroundColor Yellow
$repoDir = "C:\kavach-rs"
if (Test-Path $repoDir) {
    Write-Host "Repository already exists at $repoDir, pulling latest..." -ForegroundColor Gray
    Push-Location $repoDir
    git pull
    Pop-Location
} else {
    git clone $RepoUrl $repoDir
}

# 4. Build kavach
Write-Host "`n[4/8] Building kavach (this may take a few minutes)..." -ForegroundColor Yellow
Push-Location $repoDir
cargo build --release
Pop-Location

# 5. Install binary
Write-Host "`n[5/8] Installing kavach binary..." -ForegroundColor Yellow
$installDir = "$env:LOCALAPPDATA\kavach"
New-Item -ItemType Directory -Force -Path $installDir | Out-Null
Copy-Item "$repoDir\target\release\kavach.exe" "$installDir\kavach.exe" -Force

# Add to PATH permanently
$currentUserPath = [Environment]::GetEnvironmentVariable("Path", "User")
if ($currentUserPath -notlike "*$installDir*") {
    [Environment]::SetEnvironmentVariable("Path", "$currentUserPath;$installDir", "User")
    $env:Path = "$installDir;$env:Path"
    Write-Host "Added $installDir to PATH" -ForegroundColor Green
}

# 6. Create Claude config directories
Write-Host "`n[6/8] Creating Claude config directories..." -ForegroundColor Yellow
$claudeDir = "$env:USERPROFILE\.claude"
$dirs = @(
    "$claudeDir",
    "$claudeDir\rules",
    "$claudeDir\agents",
    "$claudeDir\commands"
)
foreach ($dir in $dirs) {
    New-Item -ItemType Directory -Force -Path $dir | Out-Null
}

# 7. Create SharedAI directories
Write-Host "`n[7/8] Creating SharedAI directories..." -ForegroundColor Yellow
$sharedAiDir = "$env:LOCALAPPDATA\SharedAI"
New-Item -ItemType Directory -Force -Path "$sharedAiDir\state" | Out-Null

# 8. Copy config files from transfer package
Write-Host "`n[8/8] Copying configuration files..." -ForegroundColor Yellow
$transferDir = "$repoDir\transfer-package"

if (Test-Path $transferDir) {
    # Copy main config files
    Copy-Item "$transferDir\CLAUDE.md" "$claudeDir\CLAUDE.md" -Force
    Copy-Item "$transferDir\settings.json" "$claudeDir\settings.json" -Force

    # Copy rules
    Copy-Item "$transferDir\rules\*" "$claudeDir\rules\" -Force

    # Copy agents
    Copy-Item "$transferDir\agents\*" "$claudeDir\agents\" -Force

    # Copy commands
    Copy-Item "$transferDir\commands\*" "$claudeDir\commands\" -Force

    Write-Host "Configuration files copied successfully" -ForegroundColor Green
} else {
    Write-Host "WARNING: transfer-package not found at $transferDir" -ForegroundColor Yellow
    Write-Host "Please copy config files manually" -ForegroundColor Yellow
}

# Initialize database
Write-Host "`n[9/9] Initializing kavach database..." -ForegroundColor Yellow
& "$installDir\kavach.exe" db init

# Verify installation
Write-Host "`n========================================" -ForegroundColor Cyan
Write-Host "Verifying installation..." -ForegroundColor Cyan
Write-Host "========================================" -ForegroundColor Cyan

& "$installDir\kavach.exe" status

Write-Host "`n========================================" -ForegroundColor Green
Write-Host "Setup Complete!" -ForegroundColor Green
Write-Host "========================================" -ForegroundColor Green
Write-Host ""
Write-Host "Installed components:" -ForegroundColor White
Write-Host "  - Kavach binary: $installDir\kavach.exe" -ForegroundColor Gray
Write-Host "  - Config: $claudeDir" -ForegroundColor Gray
Write-Host "  - Database: $sharedAiDir\kavach.db" -ForegroundColor Gray
Write-Host ""
Write-Host "Next steps:" -ForegroundColor White
Write-Host "  1. Install Claude Code: winget install Anthropic.ClaudeCode" -ForegroundColor Gray
Write-Host "  2. Restart terminal to refresh PATH" -ForegroundColor Gray
Write-Host "  3. Run: claude" -ForegroundColor Gray
Write-Host ""
Write-Host "Hinglish mode enabled. Responses mein Hindi+English mix hogi." -ForegroundColor Yellow
