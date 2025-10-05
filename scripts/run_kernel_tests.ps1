Param(
    [switch]$Offline
)

$ErrorActionPreference = 'Continue'
$start = Get-Date

# Resolve paths
$root = Split-Path -Parent $PSScriptRoot  # -> blog_os
$log = Join-Path $PSScriptRoot 'test_run.log'

"=== Kernel test run @ $(Get-Date -Format 'yyyy-MM-dd HH:mm:ss') ===" | Tee-Object -FilePath $log -Append | Out-Null

Push-Location $root
try {
    Write-Host "CWD: $PWD"

    # Tool versions
    & "$env:USERPROFILE\.cargo\bin\rustup.exe" -V | Tee-Object -FilePath $log -Append | Out-Null
    & "$env:USERPROFILE\.cargo\bin\rustc.exe" -V | Tee-Object -FilePath $log -Append | Out-Null
    & "$env:USERPROFILE\.cargo\bin\cargo.exe" -V | Tee-Object -FilePath $log -Append | Out-Null

    # Target-specific flags (avoid affecting host tools like xtask)
    $env:CARGO_TARGET_X86_64_BLOG_OS_JSON_RUSTFLAGS = '-Z unstable-options -Z panic_abort_tests -C panic=immediate-abort'
    Write-Host "CARGO_TARGET_X86_64_BLOG_OS_JSON_RUSTFLAGS=$env:CARGO_TARGET_X86_64_BLOG_OS_JSON_RUSTFLAGS" | Tee-Object -FilePath $log -Append | Out-Null

    # Compose cargo test args
    $args = @(
        'test','-p','blog_os',
        '-Z','build-std=core,compiler_builtins',
        '-Z','build-std-features=compiler-builtins-mem',
        '--target','x86_64-blog_os.json',
        '-vv'
    )
    if ($Offline) { $args += '--offline' }

    Write-Host ("Running: cargo {0}" -f ($args -join ' ')) | Tee-Object -FilePath $log -Append | Out-Null

    & "$env:USERPROFILE\.cargo\bin\cargo.exe" @args 2>&1 | Tee-Object -FilePath $log -Append
    $code = $LASTEXITCODE

    Write-Host "Exit code: $code" | Tee-Object -FilePath $log -Append | Out-Null
    $elapsed = (Get-Date) - $start
    Write-Host ("Elapsed: {0:N1}s" -f $elapsed.TotalSeconds) | Tee-Object -FilePath $log -Append | Out-Null
    exit $code
}
finally {
    Pop-Location
}

