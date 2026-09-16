param(
    [switch]$AllowEditorOpen
)

$ErrorActionPreference = "Stop"
$repoRoot = (Resolve-Path (Join-Path $PSScriptRoot "..")).Path
$editor = Get-Process -Name fyroxed -ErrorAction SilentlyContinue
if ($editor -and -not $AllowEditorOpen) {
    $ids = ($editor | Select-Object -ExpandProperty Id) -join ", "
    throw "Fyrox Editor is running (PID $ids). Save/close it first, or rerun with -AllowEditorOpen."
}

rtk cargo run --manifest-path (Join-Path $repoRoot "lua-tool/Cargo.toml") --bin prepare_ui_demo
if ($LASTEXITCODE -ne 0) {
    throw "Lua UI demo resource preparation failed."
}

Write-Host "Lua UI demo resource is ready: data/unnamed.ui"
