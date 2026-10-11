# Kavach source installer (Windows): bootstrap prereqs, clone, build, install to %USERPROFILE%\.local\bin, delete the clone.
$ErrorActionPreference = "Stop"

$repoUrl = "https://github.com/Gaurav-Wankhede/kavach-rs"
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

# NOTE: the SurrealDB bootstrap that used to live here was removed. The memory
# store is Turso/libSQL, EMBEDDED in the kavach binary (kavach-db -> libsql);
# there is no database server to install, download, or keep running. The old
# block fetched a pinned `surreal.exe` into $dest on every Windows install of a
# build that no longer used it.

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
  $datadir = "$env:LOCALAPPDATA\SharedAI"; Write-Host "kavach: memory store will live in $datadir (embedded Turso/libSQL — no server required)"
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
