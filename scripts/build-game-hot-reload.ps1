$ErrorActionPreference = 'Stop'
$root = (Resolve-Path (Join-Path $PSScriptRoot '..')).Path
$rustLib = (& rustc --print target-libdir).Trim(); if (Test-Path $rustLib) { $env:PATH = "$rustLib;$env:PATH" }
$fyrox = (Resolve-Path (Join-Path $PSScriptRoot '..\..\Fyrox')).Path
$env:PATH = "$fyrox\target\release;$env:PATH"
$env:RUSTFLAGS = '-C prefer-dynamic=yes'
Push-Location $root
try {
    cargo build -p game_dylib --no-default-features --features dylib-engine --profile dev-hot-reload
    if ($LASTEXITCODE -ne 0) { throw "Hot Reload 动态库构建失败" }
    Write-Host 'Hot Reload 动态库已构建。'
} finally { Pop-Location }
