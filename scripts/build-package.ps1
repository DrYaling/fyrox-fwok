$ErrorActionPreference = 'Stop'
$root = (Resolve-Path (Join-Path $PSScriptRoot '..')).Path
Push-Location $root
try {
    # 发布版只启用 lua-package，反射模块不会编译进最终产物。
    rtk cargo build -p executor --release --no-default-features --features lua-package
    if ($LASTEXITCODE -ne 0) { throw '发布构建失败' }
} finally { Pop-Location }
