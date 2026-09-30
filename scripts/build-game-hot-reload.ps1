param([ValidateSet('dev-hot-reload','release-hot-reload')][string]$Profile = 'dev-hot-reload')
$ErrorActionPreference = 'Stop'
$root = (Resolve-Path (Join-Path $PSScriptRoot '..')).Path
$rustLib = (& rustc --print target-libdir).Trim(); if (Test-Path $rustLib) { $env:PATH = "$rustLib;$env:PATH" }
$fyrox = (Resolve-Path (Join-Path $PSScriptRoot '..\..\Fyrox')).Path
$env:CARGO_TARGET_DIR = (Join-Path $fyrox 'target')
$profileDir = Join-Path $fyrox "target\$Profile"
$env:PATH = "$profileDir;$env:PATH"
$env:RUSTFLAGS = '-C prefer-dynamic=yes'
Push-Location $root
try {
    $buildArgs = @('build', '-p', 'game_dylib', '--no-default-features', '--features', 'dylib-engine,lua-editor', '--profile', $Profile)
    & cargo @buildArgs
    if ($LASTEXITCODE -ne 0) { throw 'Hot Reload dynamic library build failed.' }
    $builtDll = Join-Path $fyrox ("target\{0}\game_dylib.dll" -f $Profile)
    if (!(Test-Path $builtDll)) { throw "Hot Reload DLL was not produced: $builtDll" }
    $std = Get-ChildItem (rustc --print target-libdir) -Filter 'std-*.dll' | Select-Object -First 1
    if ($null -eq $std) { throw 'Rust std runtime DLL was not found.' }
    if (!(Test-Path (Join-Path $profileDir $std.Name))) {
        Copy-Item -LiteralPath $std.FullName -Destination (Join-Path $profileDir $std.Name) -Force
    }
    Write-Host ("Hot Reload game DLL built with profile {0}; lua-plugin is linked and is not a hot-reload target." -f $Profile)
} finally { Pop-Location }
