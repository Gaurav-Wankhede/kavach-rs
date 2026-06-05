# Development Tools Installation for Windows Surface Pro 8
# Run as Administrator: Right-click PowerShell -> Run as Administrator

$ErrorActionPreference = "Stop"

Write-Host "========================================" -ForegroundColor Cyan
Write-Host "Development Tools Installation" -ForegroundColor Cyan
Write-Host "========================================" -ForegroundColor Cyan

# Check if running as Administrator
$isAdmin = ([Security.Principal.WindowsPrincipal] [Security.Principal.WindowsIdentity]::GetCurrent()).IsInRole([Security.Principal.WindowsBuiltInRole]::Administrator)
if (-not $isAdmin) {
    Write-Host "ERROR: Please run as Administrator" -ForegroundColor Red
    exit 1
}

Write-Host "`n[1/12] Installing Git..." -ForegroundColor Yellow
winget install Git.Git -e --silent --accept-package-agreements --accept-source-agreements

Write-Host "`n[2/12] Installing GitHub CLI..." -ForegroundColor Yellow
winget install GitHub.cli -e --silent --accept-package-agreements --accept-source-agreements

Write-Host "`n[3/12] Installing Rust..." -ForegroundColor Yellow
winget install Rustlang.Rustup -e --silent --accept-package-agreements --accept-source-agreements

Write-Host "`n[4/12] Installing Visual Studio Build Tools..." -ForegroundColor Yellow
winget install Microsoft.VisualStudio.2022.BuildTools -e --silent --accept-package-agreements --accept-source-agreements

Write-Host "`n[5/12] Installing Node.js LTS..." -ForegroundColor Yellow
winget install OpenJS.NodeJS.LTS -e --silent --accept-package-agreements --accept-source-agreements

Write-Host "`n[6/12] Installing Python..." -ForegroundColor Yellow
winget install Python.Python.3.12 -e --silent --accept-package-agreements --accept-source-agreements

Write-Host "`n[7/12] Installing VS Code..." -ForegroundColor Yellow
winget install Microsoft.VisualStudioCode -e --silent --accept-package-agreements --accept-source-agreements

Write-Host "`n[8/12] Installing Windows Terminal..." -ForegroundColor Yellow
winget install Microsoft.WindowsTerminal -e --silent --accept-package-agreements --accept-source-agreements

Write-Host "`n[9/12] Installing PostgreSQL..." -ForegroundColor Yellow
winget install PostgreSQL.PostgreSQL -e --silent --accept-package-agreements --accept-source-agreements

Write-Host "`n[10/12] Installing DBeaver (Database GUI)..." -ForegroundColor Yellow
winget install dbeaver.dbeaver -e --silent --accept-package-agreements --accept-source-agreements

Write-Host "`n[11/12] Installing Postman (API Testing)..." -ForegroundColor Yellow
winget install Postman.Postman -e --silent --accept-package-agreements --accept-source-agreements

Write-Host "`n[12/12] Installing Claude Code..." -ForegroundColor Yellow
winget install Anthropic.ClaudeCode -e --silent --accept-package-agreements --accept-source-agreements

# Refresh PATH
Write-Host "`nRefreshing PATH..." -ForegroundColor Yellow
$env:Path = [System.Environment]::GetEnvironmentVariable("Path", "Machine") + ";" + [System.Environment]::GetEnvironmentVariable("Path", "User")

# Configure Git (basic settings)
Write-Host "`nConfiguring Git defaults..." -ForegroundColor Yellow
git config --global init.defaultBranch main
git config --global core.autocrlf true
git config --global core.editor "code --wait"

Write-Host "`n========================================" -ForegroundColor Green
Write-Host "Installation Complete!" -ForegroundColor Green
Write-Host "========================================" -ForegroundColor Green

Write-Host "`nInstalled Tools:" -ForegroundColor White
Write-Host "  - Git (version control)" -ForegroundColor Gray
Write-Host "  - GitHub CLI (gh command)" -ForegroundColor Gray
Write-Host "  - Rust + Cargo (Rust toolchain)" -ForegroundColor Gray
Write-Host "  - VS Build Tools (C++ compiler)" -ForegroundColor Gray
Write-Host "  - Node.js LTS (JavaScript runtime)" -ForegroundColor Gray
Write-Host "  - Python 3.12 (Python runtime)" -ForegroundColor Gray
Write-Host "  - VS Code (code editor)" -ForegroundColor Gray
Write-Host "  - Windows Terminal (better terminal)" -ForegroundColor Gray
Write-Host "  - PostgreSQL (database)" -ForegroundColor Gray
Write-Host "  - DBeaver (database GUI)" -ForegroundColor Gray
Write-Host "  - Postman (API testing)" -ForegroundColor Gray
Write-Host "  - Claude Code (AI assistant)" -ForegroundColor Gray

Write-Host "`nNext Steps:" -ForegroundColor Yellow
Write-Host "  1. RESTART your computer to apply PATH changes" -ForegroundColor White
Write-Host "  2. Configure Git user:" -ForegroundColor White
Write-Host "     git config --global user.name 'Your Name'" -ForegroundColor Gray
Write-Host "     git config --global user.email 'your@email.com'" -ForegroundColor Gray
Write-Host "  3. Login to GitHub:" -ForegroundColor White
Write-Host "     gh auth login" -ForegroundColor Gray
Write-Host "  4. Run Kavach setup:" -ForegroundColor White
Write-Host "     .\setup-kavach-tester.ps1" -ForegroundColor Gray

Write-Host "`nHinglish Note:" -ForegroundColor Cyan
Write-Host "  Computer restart karna zaroori hai installation ke baad." -ForegroundColor White
Write-Host "  Phir Git configure karo apne name aur email ke saath." -ForegroundColor White
