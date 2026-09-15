$ErrorActionPreference = 'Stop'
$root = (Resolve-Path (Join-Path $PSScriptRoot '..')).Path
$rustLib = (& rustc --print target-libdir).Trim(); if (Test-Path $rustLib) { $env:PATH = "$rustLib;$env:PATH" }
$fyrox = (Resolve-Path (Join-Path $PSScriptRoot '..\..\Fyrox')).Path
$env:CARGO_TARGET_DIR = (Join-Path $fyrox 'target')
$env:PATH = "$fyrox\target\release;$root\target\dev-hot-reload;$env:PATH"
$env:RUSTFLAGS = '-C prefer-dynamic=yes'
Push-Location $root
try {
    cargo build -p game_dylib --no-default-features --features "dylib-engine,lua-editor" --release
    if ($LASTEXITCODE -ne 0) { throw 'Hot Reload dynamic library build failed.' }
    $std = Get-ChildItem (rustc --print target-libdir) -Filter 'std-*.dll' | Select-Object -First 1
    if ($null -eq $std) { throw 'Rust std runtime DLL was not found.' }
    foreach ($output in @((Join-Path $fyrox 'target\release'), (Join-Path $root 'target\dev-hot-reload'))) {
        if (Test-Path $output) {
            Copy-Item -LiteralPath $std.FullName -Destination (Join-Path $output $std.Name) -Force
        }
    }
    Write-Host 'Hot Reload dynamic library built.'
} finally { Pop-Location }
