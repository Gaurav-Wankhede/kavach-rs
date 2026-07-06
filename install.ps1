# Kavach installer (Windows): detect arch, download the matching release binary, install to %USERPROFILE%\.local\bin.
$ErrorActionPreference = "Stop"

$repo = "Wankhede-Brothers/kavach-rs"
$base = "https://github.com/$repo/releases/latest/download"
$dest = if ($env:KAVACH_INSTALL_DIR) { $env:KAVACH_INSTALL_DIR } else { "$env:USERPROFILE\.local\bin" }

$cpu = switch ($env:PROCESSOR_ARCHITECTURE) {
  "AMD64" { "amd64" }
  "ARM64" { "arm64" }
  default { throw "kavach: unsupported arch '$env:PROCESSOR_ARCHITECTURE'" }
}

$asset = "kavach-windows-$cpu.zip"
$url   = "$base/$asset"
$tmp   = New-Item -ItemType Directory -Path (Join-Path $env:TEMP ([System.Guid]::NewGuid()))

try {
  Write-Host "kavach: downloading $asset ..."
  Invoke-WebRequest -Uri $url -OutFile "$tmp\k.zip"
  Expand-Archive -Path "$tmp\k.zip" -DestinationPath $tmp -Force
  New-Item -ItemType Directory -Force -Path $dest | Out-Null
  Copy-Item "$tmp\kavach.exe" "$dest\kavach.exe" -Force
  Write-Host "kavach: installed to $dest\kavach.exe"
  if ($env:PATH -notlike "*$dest*") { Write-Host "kavach: add $dest to PATH" }
  & "$dest\kavach.exe" --version
} finally {
  Remove-Item -Recurse -Force $tmp -ErrorAction SilentlyContinue
}
