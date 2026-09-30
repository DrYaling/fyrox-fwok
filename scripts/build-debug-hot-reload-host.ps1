$ErrorActionPreference = 'Stop'
$root = (Resolve-Path (Join-Path $PSScriptRoot '..')).Path
$fyrox = (Resolve-Path (Join-Path $PSScriptRoot '..\..\Fyrox')).Path
$env:CARGO_TARGET_DIR = (Join-Path $fyrox 'target')
$env:RUSTFLAGS = '-C prefer-dynamic=yes'

Push-Location $root
try {
    & cargo build -p executor --no-default-features --features "dylib,lua-editor" --profile dev-hot-reload
    if ($LASTEXITCODE -ne 0) { throw 'Stable DebugHR executor build failed.' }
    Write-Host 'Stable DebugHR host built. Rebuild this only after executor, lua-plugin, or engine changes.'
} finally { Pop-Location }
