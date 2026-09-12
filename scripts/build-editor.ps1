param([ValidateSet('debug','release')][string]$Profile = 'release')
$ErrorActionPreference = 'Stop'; $fyrox = (Resolve-Path (Join-Path $PSScriptRoot '..\..\Fyrox')).Path
if ($Profile -eq 'release') { cargo build --manifest-path "$fyrox\Cargo.toml" -p fyroxed --features dylib --release } else { cargo build --manifest-path "$fyrox\Cargo.toml" -p fyroxed --features dylib }
