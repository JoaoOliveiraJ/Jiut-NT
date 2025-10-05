Param(
    [switch]$UEFI,
    [switch]$Release
)

$ErrorActionPreference = 'Continue'
$root = Split-Path -Parent $PSScriptRoot
Push-Location $root
try {
    $profile = if ($Release) { '--release' } else { '' }
    $label = if ([string]::IsNullOrWhiteSpace($profile)) { 'debug' } else { 'release' }
    Write-Host "Building kernel image ($label)..."
    if ($UEFI) {
        & "$env:USERPROFILE\.cargo\bin\cargo.exe" run --target x86_64-pc-windows-msvc -p xtask -- build $profile 2>&1 | Write-Host
        $img = Join-Path $root 'target\uefi.img'
        $ovmf = 'C:\Program Files\qemu\share\edk2-x86_64-code.fd'
        & 'C:\Program Files\qemu\qemu-system-x86_64.exe' -drive if=pflash,format=raw,readonly=on,file=$ovmf -drive format=raw,file=$img -m 256M -serial stdio
    }
    else {
        & "$env:USERPROFILE\.cargo\bin\cargo.exe" run --target x86_64-pc-windows-msvc -p xtask -- build --bios $profile 2>&1 | Write-Host
        $img = Join-Path $root 'target\bios.img'
        & 'C:\Program Files\qemu\qemu-system-x86_64.exe' -drive format=raw,file=$img -m 256M -serial stdio
    }
}
finally {
    Pop-Location
}
