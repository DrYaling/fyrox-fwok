$ErrorActionPreference = 'Stop'

$profiles = @('dev-hot-reload', 'release-hot-reload')
$Profile = 'dev-hot-reload'
$ExecutorArgs = @($args)

# Do not use a param block here. The editor appends `-- --override-scene path`
# to the configured command, and PowerShell otherwise tries to bind these as
# script parameters before they can be forwarded to the executor.
if ($ExecutorArgs.Count -ge 2 -and $ExecutorArgs[0] -eq '-Profile') {
    $Profile = $ExecutorArgs[1]
    $ExecutorArgs = @($ExecutorArgs | Select-Object -Skip 2)
} elseif ($ExecutorArgs.Count -gt 0 -and $profiles -contains $ExecutorArgs[0]) {
    $Profile = $ExecutorArgs[0]
    $ExecutorArgs = @($ExecutorArgs | Select-Object -Skip 1)
}
if ($profiles -notcontains $Profile) {
    throw "Unsupported hot-reload profile: $Profile"
}
if ($ExecutorArgs.Count -gt 0 -and $ExecutorArgs[0] -eq '--') {
    $ExecutorArgs = @($ExecutorArgs | Select-Object -Skip 1)
}
$fyrox = (Resolve-Path (Join-Path $PSScriptRoot '..\..\Fyrox')).Path
$profileDir = Join-Path $fyrox "target\$Profile"
$executor = Join-Path $profileDir 'executor.exe'

foreach ($artifact in @('executor.exe', 'fyrox_dylib.dll', 'game_dylib.dll')) {
    $artifactPath = Join-Path $profileDir $artifact
    if (!(Test-Path $artifactPath)) {
        throw "Missing $Profile artifact: $artifactPath. Rebuild the matching hot-reload host first."
    }
}

$rustLib = (& rustc --print target-libdir).Trim()
$env:PATH = "$profileDir;$rustLib;$env:PATH"
$env:FYROX_GAME_DYLIB = Join-Path $profileDir 'game_dylib.dll'

Write-Host "Starting matched $Profile executor: $executor"
& $executor @ExecutorArgs
exit $LASTEXITCODE
