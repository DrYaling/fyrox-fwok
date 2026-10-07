@echo off
setlocal
for %%I in ("%~dp0..") do set "WORKSPACE_ROOT=%%~fI"
if not exist "%~dp0editor.exe" (
    echo editor.exe was not found beside this launcher.
    pause
    exit /b 1
)
start "" /D "%WORKSPACE_ROOT%" "%~dp0editor.exe" --project-directory "%WORKSPACE_ROOT%" %*
exit /b 0
