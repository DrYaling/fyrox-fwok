param([ValidateSet('release')][string]$Profile = 'release')

$ErrorActionPreference = 'Stop'
$projectRoot = (Resolve-Path (Join-Path $PSScriptRoot '..')).Path
$targetDir = Join-Path $projectRoot 'target'
$releaseDir = Join-Path $projectRoot 'release'
$editorExe = Join-Path $targetDir 'release\editor.exe'
$fyroxRoot = (Resolve-Path (Join-Path $projectRoot '..\Fyrox')).Path
$projectManagerExe = Join-Path $fyroxRoot 'target\release\fyrox-project-manager.exe'

Push-Location $projectRoot
try {
    & rtk cargo build --manifest-path (Join-Path $projectRoot 'Cargo.toml') --target-dir $targetDir --package editor --release
    if ($LASTEXITCODE -ne 0) { throw 'FWOK editor release build failed.' }
} finally {
    Pop-Location
}

if (!(Test-Path -LiteralPath $editorExe)) {
    throw "Editor build did not produce editor.exe: $editorExe"
}
if (!(Test-Path -LiteralPath $projectManagerExe)) {
    throw "Fyrox Project Manager release executable was not found: $projectManagerExe"
}

New-Item -ItemType Directory -Force -Path $releaseDir | Out-Null
Copy-Item -LiteralPath $editorExe -Destination (Join-Path $releaseDir 'editor.exe') -Force
Copy-Item -LiteralPath $projectManagerExe -Destination (Join-Path $releaseDir 'project-manager.exe') -Force

# Rust's MSVC release binaries use the Visual C++ runtime. Ship the small
# runtime DLL beside the exe and beside the MCP bridge so a clean Windows
# machine does not need a Rust or C++ development installation. The Universal
# CRT is part of supported Windows versions; VCRUNTIME140 is copied from the
# build machine when available.
$vcRuntime = Join-Path $env:WINDIR 'System32\vcruntime140.dll'
if (!(Test-Path -LiteralPath $vcRuntime -PathType Leaf)) {
    throw "Required Visual C++ runtime was not found: $vcRuntime"
}
Copy-Item -LiteralPath $vcRuntime -Destination (Join-Path $releaseDir 'vcruntime140.dll') -Force

# Ship the runtime project data beside the executable so a double-clicked exe
# can open the same project without a Rust toolchain or source checkout.
$releaseData = Join-Path $releaseDir 'data'
New-Item -ItemType Directory -Force -Path $releaseData | Out-Null
Copy-Item -Path (Join-Path $projectRoot 'data\*') -Destination $releaseData -Recurse -Force
$bridgeDir = Join-Path $releaseData 'editor'
New-Item -ItemType Directory -Force -Path $bridgeDir | Out-Null
Copy-Item -LiteralPath $vcRuntime -Destination (Join-Path $bridgeDir 'vcruntime140.dll') -Force

$settings = Join-Path $projectRoot 'settings.ron'
if (Test-Path -LiteralPath $settings) {
    Copy-Item -LiteralPath $settings -Destination $releaseDir -Force
}

# Put the MCP stdio bridge in the packaged project, where the editor plugin
# discovers it at runtime. This builds the bridge with the same release profile.
& (Join-Path $PSScriptRoot 'build-mcp-bridge.ps1') -Profile release -DestinationDirectory (Join-Path $releaseData 'editor')
foreach ($launcher in @('start.cmd', 'start-project-manager.cmd', 'start-editor.cmd', 'start-editor-workspace.cmd')) {
    Copy-Item -LiteralPath (Join-Path $PSScriptRoot $launcher) -Destination (Join-Path $releaseDir $launcher) -Force
}

foreach ($required in @(
    (Join-Path $releaseDir 'editor.exe'),
    (Join-Path $releaseDir 'project-manager.exe'),
    (Join-Path $releaseDir 'vcruntime140.dll'),
    (Join-Path $releaseData 'rpg_level.rgs'),
    (Join-Path $releaseData 'resources.registry'),
    (Join-Path $releaseData 'editor\mcp-bridge.exe'),
    (Join-Path $releaseData 'editor\vcruntime140.dll'),
    (Join-Path $releaseDir 'start-project-manager.cmd'),
    (Join-Path $releaseDir 'start.cmd'),
    (Join-Path $releaseDir 'start-editor.cmd'),
    (Join-Path $releaseDir 'start-editor-workspace.cmd')
)) {
    if (!(Test-Path -LiteralPath $required -PathType Leaf)) {
        throw "Release package is incomplete; missing: $required"
    }
}

Write-Host "FWOK release package is ready: $releaseDir"
Write-Host 'Start with start-project-manager.cmd. Use start-editor.cmd for the packaged project or start-editor-workspace.cmd for the source workspace.'
