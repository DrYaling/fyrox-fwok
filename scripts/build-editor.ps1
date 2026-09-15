param([ValidateSet('release')][string]$Profile = 'release')
$ErrorActionPreference = 'Stop'; $fyrox = (Resolve-Path (Join-Path $PSScriptRoot '..\..\Fyrox')).Path
$env:RUSTFLAGS = '-C prefer-dynamic=yes'
cargo build --manifest-path "$fyrox\Cargo.toml" -p fyroxed --release
if ($LASTEXITCODE -ne 0) { throw 'Editor build failed.' }
