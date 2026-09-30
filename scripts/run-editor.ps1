param(
    [string]$Project = '',
    [switch]$RebuildEditor,
    [ValidateSet('dev-hot-reload','release-hot-reload')][string]$GameProfile = 'dev-hot-reload'
)
$ErrorActionPreference = 'Stop'
$root = (Resolve-Path (Join-Path $PSScriptRoot '..')).Path
$fyrox = (Resolve-Path (Join-Path $PSScriptRoot '..\..\Fyrox')).Path
$exe = Join-Path $root 'target\debug\editor.exe'
$projectInput = if ([string]::IsNullOrWhiteSpace($Project)) { $root } else { $Project }
$projectPath = (Resolve-Path $projectInput).Path
if (!(Test-Path (Join-Path $projectPath 'Cargo.toml'))) {
    throw "Project directory does not contain Cargo.toml: $projectPath"
}
$rustLib = (& rustc --print target-libdir).Trim(); if (Test-Path $rustLib) { $env:PATH = "$rustLib;$env:PATH" }
$env:CARGO_TARGET_DIR = (Join-Path $fyrox 'target')
$env:PATH = "$fyrox\target\release;$env:PATH"
$env:RUSTFLAGS = '-C prefer-dynamic=yes'
# 先构建游戏热重载库，再重建 Editor，确保 fyrox_dylib.dll 与 fyroxed.exe 来自同一源码状态。
# Only game_dylib.dll is rebuilt on this path. Lua bindings remain linked into
# that DLL and are not watched as an independent Fyrox dynamic plugin.
& (Join-Path $PSScriptRoot 'build-game-hot-reload.ps1') -Profile $GameProfile
$profileDir = Join-Path $fyrox "target\$GameProfile"
$hostMissing = @('executor.exe', 'fyrox_dylib.dll') | Where-Object { !(Test-Path (Join-Path $profileDir $_)) }
if ($hostMissing.Count -gt 0) {
    if ($GameProfile -ne 'dev-hot-reload') {
        throw "Missing $GameProfile host artifact(s): $($hostMissing -join ', '). Build the matching host profile first."
    }
    & (Join-Path $PSScriptRoot 'build-debug-hot-reload-host.ps1')
}
foreach ($artifact in @('executor.exe', 'fyrox_dylib.dll', 'game_dylib.dll')) {
    $artifactPath = Join-Path $profileDir $artifact
    if (!(Test-Path $artifactPath)) {
        throw "Missing $GameProfile host artifact: $artifactPath. Run scripts\build-debug-hot-reload-host.ps1 first."
    }
}
if ($RebuildEditor -or !(Test-Path $exe)) {
    cargo build --manifest-path (Join-Path $root 'Cargo.toml') --target-dir (Join-Path $root 'target') -p editor
    if ($LASTEXITCODE -ne 0) { throw 'FWOK editor build failed.' }
} else {
    Write-Host 'Reusing existing FWOK editor binary; pass -RebuildEditor to rebuild it.'
}
if (!(Test-Path $exe)) { throw 'Editor build did not produce editor.exe.' }
# Keep the temporary editor-launched MCP copy synchronized even when the Editor binary is reused.
& (Join-Path $PSScriptRoot 'build-mcp-bridge.ps1') -Profile debug
$editorArgs = @('--project-directory', $projectPath)
Write-Host ("Starting Fyrox Editor: {0} {1}" -f $exe, ($editorArgs -join ' '))
Start-Process -FilePath $exe -ArgumentList $editorArgs -WorkingDirectory $projectPath
