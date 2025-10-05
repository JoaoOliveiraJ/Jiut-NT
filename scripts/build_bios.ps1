Param(
    [switch]$Release
)

$ErrorActionPreference = 'Stop'
$root = Split-Path -Parent $PSScriptRoot
Push-Location $root
try {
    $profile = if ($Release) { '--release' } else { '' }
    Write-Host 'Building host helper (xtask) ...'
    $prev = $ErrorActionPreference; $ErrorActionPreference = 'Continue'
    cargo build -p xtask 2>&1 | Write-Host
    if ($LASTEXITCODE -ne 0) { $ErrorActionPreference = $prev; throw "cargo build xtask failed ($LASTEXITCODE)" }
    Write-Host 'Building BIOS image via bootloader...'
    cargo run -p xtask -- build --bios $profile 2>&1 | Write-Host
    if ($LASTEXITCODE -ne 0) { $ErrorActionPreference = $prev; throw "xtask build image failed ($LASTEXITCODE)" }
    $ErrorActionPreference = $prev
    $imgOut = Join-Path $root 'target/bios.img'
    if (-not (Test-Path $imgOut)) { throw "failed to build BIOS image: $imgOut not found" }
    Write-Host "OK: $imgOut"
}
finally {
    Pop-Location
}
