param(
    [string]$InputPath = "lua-plugin/src",
    [string]$OutputPath = "target/lua-bindings",
    [string]$RuntimeOutput = "lua-plugin/src/bindings/generated.rs"
)

$ErrorActionPreference = "Stop"
$repoRoot = (Resolve-Path (Join-Path $PSScriptRoot ".." )).Path
$resolveRepoPath = {
    param([string]$PathValue)
    if ([IO.Path]::IsPathRooted($PathValue)) { return $PathValue }
    return (Join-Path $repoRoot $PathValue)
}
$inputResolved = & $resolveRepoPath $InputPath
$outputResolved = & $resolveRepoPath $OutputPath
$runtimeResolved = & $resolveRepoPath $RuntimeOutput
rtk cargo run --bin lua-tool --manifest-path (Join-Path $repoRoot "lua-tool/Cargo.toml") -- `
    --input $inputResolved `
    --output $outputResolved `
    --runtime-output $runtimeResolved `
    --profile common
if ($LASTEXITCODE -ne 0) {
    throw "Offline Lua binding generation failed."
}

rtk cargo fmt --manifest-path (Join-Path $repoRoot "lua-plugin/Cargo.toml")
if ($LASTEXITCODE -ne 0) {
    throw "Generated binding formatting failed."
}

Write-Host "Lua bindings generated: $outputResolved"
