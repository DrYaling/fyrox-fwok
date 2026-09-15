param(
    [string]$InputPath = "lua-plugin/src",
    [string]$OutputPath = "target/lua-bindings",
    [string]$RuntimeOutput = "lua-plugin/src/bindings/generated.rs"
)

$ErrorActionPreference = "Stop"
rtk cargo run -p lua-tool -- `
    --input $InputPath `
    --output $OutputPath `
    --runtime-output $RuntimeOutput `
    --profile common
if ($LASTEXITCODE -ne 0) {
    throw "Offline Lua binding generation failed."
}

rtk cargo fmt --manifest-path lua-plugin/Cargo.toml
if ($LASTEXITCODE -ne 0) {
    throw "Generated binding formatting failed."
}

Write-Host "Lua bindings generated: $OutputPath"
