Param(
    [switch]$GUI
)
$ErrorActionPreference = 'Continue'
$root = Split-Path -Parent $PSScriptRoot
Push-Location $root
try {
    $env:CARGO_TARGET_X86_64_BLOG_OS_JSON_RUSTFLAGS='-Z unstable-options -Z panic_abort_tests -C panic=immediate-abort'
    $args = @('test','-p','blog_os','--features','df_test','-Z','build-std=core,compiler_builtins','-Z','build-std-features=compiler-builtins-mem','--target','x86_64-blog_os.json','-vv')
    if ($GUI) {
        # Ensure GUI is used by the runner
        (Get-Content .\.cargo\config.toml).Replace('--gui','--gui') | Out-Null
    }
    & "$env:USERPROFILE\.cargo\bin\cargo.exe" @args
    exit $LASTEXITCODE
}
finally {
    Pop-Location
}

