param([ValidateSet('dev-hot-reload','release-hot-reload')][string]$Profile = 'dev-hot-reload')
$ErrorActionPreference = 'Stop'
$root = (Resolve-Path (Join-Path $PSScriptRoot '..')).Path
$rustLib = (& rustc --print target-libdir).Trim(); if (Test-Path $rustLib) { $env:PATH = "$rustLib;$env:PATH" }
$fyrox = (Resolve-Path (Join-Path $PSScriptRoot '..\..\Fyrox')).Path
$env:CARGO_TARGET_DIR = (Join-Path $fyrox 'target')
$env:PATH = "$fyrox\target\release;$root\target\dev-hot-reload;$env:PATH"
$env:RUSTFLAGS = '-C prefer-dynamic=yes'
Push-Location $root
try {
    $buildArgs = @('build', '-p', 'game_dylib', '--no-default-features', '--features', 'dylib-engine,lua-editor', '--profile', $Profile)
    & cargo @buildArgs
    if ($LASTEXITCODE -ne 0) { throw 'Hot Reload dynamic library build failed.' }
    $builtDll = Join-Path $fyrox ("target\{0}\game_dylib.dll" -f $Profile)
    if (!(Test-Path $builtDll)) { throw "Hot Reload DLL was not produced: $builtDll" }
    $releaseDll = Join-Path $fyrox 'target\release\game_dylib.dll'
    Copy-Item -LiteralPath $builtDll -Destination $releaseDll -Force
    $std = Get-ChildItem (rustc --print target-libdir) -Filter 'std-*.dll' | Select-Object -First 1
    if ($null -eq $std) { throw 'Rust std runtime DLL was not found.' }
    foreach ($output in @((Join-Path $fyrox 'target\release'), (Join-Path $root 'target\dev-hot-reload'))) {
        if (Test-Path $output) {
            Copy-Item -LiteralPath $std.FullName -Destination (Join-Path $output $std.Name) -Force
        }
    }
    Write-Host ("Hot Reload game DLL built with profile {0}; lua-plugin is not a hot-reload target." -f $Profile)
} finally { Pop-Location }
