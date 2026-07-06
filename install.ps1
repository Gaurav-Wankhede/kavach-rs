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
  # pin SurrealDB to 3.1.4 via the direct release asset (the iex installer takes no version arg)
  Write-Host "kavach: installing SurrealDB 3.1.4 …"
  New-Item -ItemType Directory -Force -Path $dest | Out-Null
  Invoke-WebRequest -Uri "https://github.com/surrealdb/surrealdb/releases/download/v3.1.4/surreal-v3.1.4.windows-amd64.exe" -OutFile "$dest\surreal.exe"
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
  $datadir = "$env:LOCALAPPDATA\SharedAI"; Write-Host "kavach: memory store will live in $datadir (SurrealDB 3.1.4)"
  Write-Host "kavach: installed to $dest\kavach.exe"
  Write-Host "kavach: provisioning the Rust toolbelt (rg, fd, bat, sd, xh, ...) ..."
  try { & "$dest\kavach.exe" toolbelt install --yes } catch { Write-Host "kavach: toolbelt provisioning skipped — run 'kavach toolbelt install' manually" }
  Write-Host "kavach: wiring hooks into your AI harness settings.json ..."
  try { & "$dest\kavach.exe" install --vendor all } catch { Write-Host "kavach: hook wiring skipped — run 'kavach install --vendor all' manually" }
  if ($env:PATH -notlike "*$dest*") { Write-Host "kavach: add $dest to PATH" }
  & "$dest\kavach.exe" --version
  Write-Host "kavach: update later with ``kavach update`` (no re-clone needed by you)."
} finally {
  Remove-Item -Recurse -Force $tmp -ErrorAction SilentlyContinue
}
