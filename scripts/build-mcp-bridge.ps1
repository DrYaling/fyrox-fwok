param([ValidateSet('debug','release')][string]$Profile = 'debug')
$ErrorActionPreference = 'Stop'
$projectRoot = (Resolve-Path (Join-Path $PSScriptRoot '..')).Path
$targetProfile = if ($Profile -eq 'release') { 'release' } else { 'debug' }
$source = Join-Path $projectRoot "target\$targetProfile\mcp-bridge.exe"
$destinationDir = Join-Path $projectRoot 'data\editor'
$destination = Join-Path $destinationDir 'mcp-bridge.exe'

Push-Location $projectRoot
try {
    $cargoArgs = @('build', '--manifest-path', (Join-Path $projectRoot 'mcp/Cargo.toml'), '--target-dir', (Join-Path $projectRoot 'target'), '-p', 'mcp-bridge')
    if ($Profile -eq 'release') { $cargoArgs += '--release' }
    & rtk cargo @cargoArgs
    if ($LASTEXITCODE -ne 0) { throw 'mcp-bridge build failed.' }
} finally {
    Pop-Location
}
if (!(Test-Path $source)) { throw "mcp-bridge build did not produce: $source" }
New-Item -ItemType Directory -Force -Path $destinationDir | Out-Null
Copy-Item -Force -LiteralPath $source -Destination $destination
Write-Host "Copied mcp-bridge.exe to $destination"
