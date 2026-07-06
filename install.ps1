# Kavach source installer (Windows): bootstrap prereqs, clone, build, install to %USERPROFILE%\.local\bin, delete the clone.
$ErrorActionPreference = "Stop"

$repoUrl = "https://github.com/Wankhede-Brothers/kavach-rs"
$dest = if ($env:KAVACH_INSTALL_DIR) { $env:KAVACH_INSTALL_DIR } else { "$env:USERPROFILE\.local\bin" }

if (-not (Get-Command git -ErrorAction SilentlyContinue)) {
  throw "kavach: git is required and was not found — install it first (https://git-scm.com/download/win)"
}

if (-not (Get-Command cargo -ErrorAction SilentlyContinue)) {
  Write-Host "kavach: installing Rust via rustup …"
  Invoke-WebRequest -Uri "https://win.rustup.rs" -OutFile "$env:TEMP\rustup-init.exe"
  & "$env:TEMP\rustup-init.exe" -y
  $env:Path = "$env:USERPROFILE\.cargo\bin;$env:Path"
}
if (Get-Command rustup -ErrorAction SilentlyContinue) { rustup update 2>$null }

if (-not (Get-Command surreal -ErrorAction SilentlyContinue)) {
  Write-Host "kavach: installing SurrealDB …"
  iwr https://windows.surrealdb.com -useb | iex
  Write-Host "kavach: if a specific SurrealDB version is required, install surreal 3.1.4 manually from surrealdb.com/install"
}

$tmp = New-Item -ItemType Directory -Path (Join-Path $env:TEMP ([System.Guid]::NewGuid()))

try {
  Write-Host "kavach: cloning $repoUrl ..."
  git clone --depth 1 $repoUrl "$tmp\src"
  Push-Location "$tmp\src"
  Write-Host "kavach: building kavach-cli (release) ..."
  cargo build --release -p kavach-cli
  Pop-Location
  New-Item -ItemType Directory -Force -Path $dest | Out-Null
  Copy-Item "$tmp\src\target\release\kavach.exe" "$dest\kavach.exe" -Force
  Write-Host "kavach: installed to $dest\kavach.exe"
  if ($env:PATH -notlike "*$dest*") { Write-Host "kavach: add $dest to PATH" }
  & "$dest\kavach.exe" --version
  Write-Host "kavach: update later with ``kavach update`` (no re-clone needed by you)."
} finally {
  Remove-Item -Recurse -Force $tmp -ErrorAction SilentlyContinue
}
