param([string]$Project = '')

$ErrorActionPreference = 'Stop'
$projectRoot = (Resolve-Path (Join-Path $PSScriptRoot '..')).Path
$releaseDir = Join-Path $projectRoot 'release'
$editorExe = Join-Path $releaseDir 'editor.exe'

if (!(Test-Path -LiteralPath $editorExe -PathType Leaf)) {
    throw "Packaged editor was not found at $editorExe. Build it first with scripts\build-editor.ps1."
}

$projectInput = if ([string]::IsNullOrWhiteSpace($Project)) { $releaseDir } else { $Project }
$projectPath = (Resolve-Path -LiteralPath $projectInput).Path
if (!(Test-Path -LiteralPath (Join-Path $projectPath 'data\rpg_level.rgs') -PathType Leaf)) {
    throw "Project directory must contain data\rpg_level.rgs: $projectPath"
}

$quotedProjectPath = '"{0}"' -f $projectPath
Start-Process -FilePath $editorExe `
    -ArgumentList @('--project-directory', $quotedProjectPath) `
    -WorkingDirectory $projectPath
