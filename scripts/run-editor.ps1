param([string]$Project = '.')
$ErrorActionPreference = 'Stop'
$fyrox = (Resolve-Path (Join-Path $PSScriptRoot '..\..\Fyrox')).Path
$exe = Join-Path $fyrox 'target\release\fyroxed.exe'
$projectPath = (Resolve-Path $Project).Path
$rustLib = (& rustc --print target-libdir).Trim(); if (Test-Path $rustLib) { $env:PATH = "$rustLib;$env:PATH" }
$root = (Resolve-Path (Join-Path $PSScriptRoot '..')).Path
$env:PATH = "$fyrox\target\release;$root\target\dev-hot-reload;$env:PATH"
$env:RUSTFLAGS = '-C prefer-dynamic=yes'
$env:FYROX_GAME_DYLIB = (Join-Path $root 'target\dev-hot-reload\game_dylib.dll')
if (!(Test-Path $exe)) { throw "未找到 Editor，请先运行 build-editor.ps1" }
& (Join-Path $PSScriptRoot 'build-game-hot-reload.ps1')
Start-Process -FilePath $exe -ArgumentList @('--project-directory', $projectPath) -WorkingDirectory $projectPath
