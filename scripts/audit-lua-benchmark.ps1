param(
    [Parameter(Mandatory = $true)]
    [string]$LogPath,
    [string]$OutputPath = "target/audit-performance/benchmark-report.json"
)

$ErrorActionPreference = "Stop"
$text = Get-Content -Raw -Encoding utf8 $LogPath
$pattern = '\[Benchmark\]\[Compare\] category=(\w+) iterations=(\d+) rust_total_ns=(\d+) rust_average_ns=([0-9.]+) lua_total_ns=(\d+) lua_average_ns=([0-9.]+) lua_over_rust=([0-9.]+)x rust_checksum=([-0-9.eE]+) lua_checksum=([-0-9.eE]+)'
$matches = [regex]::Matches($text, $pattern)
$required = @("event", "transform", "numeric", "text", "widgets")
$rows = @()
foreach ($match in $matches) {
    $rows += [ordered]@{
        category = $match.Groups[1].Value
        iterations = [int]$match.Groups[2].Value
        rust_total_ns = [long]$match.Groups[3].Value
        rust_average_ns = [double]$match.Groups[4].Value
        lua_total_ns = [long]$match.Groups[5].Value
        lua_average_ns = [double]$match.Groups[6].Value
        lua_over_rust = [double]$match.Groups[7].Value
        rust_checksum = [double]$match.Groups[8].Value
        lua_checksum = [double]$match.Groups[9].Value
    }
}
$byCategory = @{}
foreach ($row in $rows) { $byCategory[$row.category] = $row }
$missing = @($required | Where-Object { -not $byCategory.ContainsKey($_) })
$badIterations = @($rows | Where-Object { $_.iterations -ne 1000 })
$checksumMismatch = @($rows | Where-Object { [math]::Abs($_.rust_checksum - $_.lua_checksum) -gt 0.0001 })
$clickRouted = $text.Contains("[Lua] UI click routed: id=lua_benchmark_button")
$drawCommandsPositive = [regex]::IsMatch($text, 'draw_commands=(?:[1-9][0-9]*)')
$complete = ($missing.Count -eq 0 -and $badIterations.Count -eq 0 -and $checksumMismatch.Count -eq 0 -and $clickRouted -and $drawCommandsPositive)
$report = [ordered]@{
    log = (Resolve-Path $LogPath).Path
    required_categories = $required
    matched_rows = $rows.Count
    missing_categories = $missing
    bad_iterations = $badIterations
    checksum_mismatches = $checksumMismatch
    click_routed = $clickRouted
    draw_commands_positive = $drawCommandsPositive
    complete = $complete
    rows = $rows
}
$parent = Split-Path -Parent $OutputPath
New-Item -ItemType Directory -Force $parent | Out-Null
$report | ConvertTo-Json -Depth 8 | Set-Content -Encoding utf8 $OutputPath
if (-not $report.complete) {
    Write-Error "Lua/Rust benchmark evidence is incomplete. See $OutputPath"
}
Write-Output ("Benchmark evidence complete: " + $report.complete)
