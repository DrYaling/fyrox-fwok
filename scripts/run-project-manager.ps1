param([switch]$Build)
$ErrorActionPreference = 'Stop'; $fyrox = (Resolve-Path (Join-Path $PSScriptRoot '..\..\Fyrox')).Path
$rustLib = (& rustc --print target-libdir).Trim(); if (Test-Path $rustLib) { $env:PATH = "$rustLib;$env:PATH" }
if ($Build) { cargo build --manifest-path "$fyrox\Cargo.toml" -p fyrox-project-manager --release }
$exe = Join-Path $fyrox 'target\release\fyrox-project-manager.exe'; if (!(Test-Path $exe)) { throw "未找到 Project Manager，请先使用 -Build" }; Start-Process $exe -WorkingDirectory (Resolve-Path (Join-Path $PSScriptRoot '..')).Path
