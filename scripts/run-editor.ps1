param(
    [string]$Project = '',
    [switch]$RebuildEditor,
    [ValidateSet('dev-hot-reload','release-hot-reload')][string]$GameProfile = 'dev-hot-reload'
)
$ErrorActionPreference = 'Stop'
$fyrox = (Resolve-Path (Join-Path $PSScriptRoot '..\..\Fyrox')).Path
$exe = Join-Path $fyrox 'target\release\fyroxed.exe'
$root = (Resolve-Path (Join-Path $PSScriptRoot '..')).Path
$projectInput = if ([string]::IsNullOrWhiteSpace($Project)) { $root } else { $Project }
$projectPath = (Resolve-Path $projectInput).Path
if (!(Test-Path (Join-Path $projectPath 'Cargo.toml'))) {
    throw "Project directory does not contain Cargo.toml: $projectPath"
}
$rustLib = (& rustc --print target-libdir).Trim(); if (Test-Path $rustLib) { $env:PATH = "$rustLib;$env:PATH" }
$env:CARGO_TARGET_DIR = (Join-Path $fyrox 'target')
$env:PATH = "$fyrox\target\release;$root\target\dev-hot-reload;$env:PATH"
$env:RUSTFLAGS = '-C prefer-dynamic=yes'
$env:FYROX_GAME_DYLIB = (Join-Path $fyrox 'target\release\game_dylib.dll')
# 先构建游戏热重载库，再重建 Editor，确保 fyrox_dylib.dll 与 fyroxed.exe 来自同一源码状态。
& (Join-Path $PSScriptRoot 'build-game-hot-reload.ps1') -Profile $GameProfile
if ($RebuildEditor -or !(Test-Path $exe)) {
    & (Join-Path $PSScriptRoot 'build-editor.ps1')
} else {
    Write-Host 'Reusing existing Fyrox Editor binary; pass -RebuildEditor to rebuild it.'
}
if (!(Test-Path $exe)) { throw 'Editor build did not produce fyroxed.exe.' }
$editorArgs = @('--project-directory', $projectPath)
Write-Host ("Starting Fyrox Editor: {0} {1}" -f $exe, ($editorArgs -join ' '))
Start-Process -FilePath $exe -ArgumentList $editorArgs -WorkingDirectory $projectPath
