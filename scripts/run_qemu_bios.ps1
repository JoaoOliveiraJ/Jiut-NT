Param(
    [switch]$Release
)

$ErrorActionPreference = 'Stop'
$root = Split-Path -Parent $PSScriptRoot

& (Join-Path $root 'scripts/build_bios.ps1') @PSBoundParameters | Write-Host

$img = Join-Path $root 'target/bios.img'
if (-not (Test-Path $img)) { throw "image not found: $img" }

$qemuCandidates = @(
    'C:\Program Files\qemu\qemu-system-x86_64.exe',
    'C:\Program Files (x86)\qemu\qemu-system-x86_64.exe',
    'qemu-system-x86_64'
)
$qemu = $qemuCandidates | Where-Object { Get-Command $_ -ErrorAction SilentlyContinue } | Select-Object -First 1
if (-not $qemu) { throw "QEMU not found (tried: $($qemuCandidates -join ', '))" }

& $qemu -drive format=raw,file=$img -m 256M -serial stdio

