param(
    [string]$ConfigPath = "data/editor/lua/lua-bindings.toml"
)

$ErrorActionPreference = "Stop"
$repoRoot = (Resolve-Path (Join-Path $PSScriptRoot ".." )).Path
$resolveRepoPath = {
    param([string]$PathValue)
    if ([IO.Path]::IsPathRooted($PathValue)) { return $PathValue }
    return (Join-Path $repoRoot $PathValue)
}
$configResolved = & $resolveRepoPath $ConfigPath
if (-not (Test-Path -LiteralPath $configResolved)) {
    throw "Business Lua binding config was not found: $configResolved. Start the Editor once to materialize the embedded default template, or create the project config manually."
}
rtk cargo run --bin lua-tool --manifest-path (Join-Path $repoRoot "lua/Cargo.toml") -- `
    --config $configResolved
if ($LASTEXITCODE -ne 0) {
    throw "Offline Lua binding generation failed."
}

rtk cargo fmt --manifest-path (Join-Path $repoRoot "lua/Cargo.toml") --all
if ($LASTEXITCODE -ne 0) {
    throw "Generated binding formatting failed."
}

Write-Host "Business Lua bindings generated from: $configResolved"
