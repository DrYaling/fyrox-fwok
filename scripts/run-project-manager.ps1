param([switch]$Build)
$ErrorActionPreference = 'Stop'; $fyrox = (Resolve-Path (Join-Path $PSScriptRoot '..\..\Fyrox')).Path
$env:RUSTFLAGS = '-C prefer-dynamic=yes'
$rustLib = (& rustc --print target-libdir).Trim(); if (Test-Path $rustLib) { $env:PATH = "$rustLib;$env:PATH" }
$env:CARGO_TARGET_DIR = (Join-Path $fyrox 'target')
$env:PATH = "$fyrox\target\release;$env:PATH"
if ($Build) { cargo build --manifest-path "$fyrox\Cargo.toml" -p fyrox-project-manager --release }
$exe = Join-Path $fyrox 'target\release\fyrox-project-manager.exe'; if (!(Test-Path $exe)) { throw 'Project Manager not found. Run this script with -Build first.' }; Start-Process $exe -WorkingDirectory (Resolve-Path (Join-Path $PSScriptRoot '..')).Path
