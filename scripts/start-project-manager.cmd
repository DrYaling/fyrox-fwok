@echo off
setlocal
if not exist "%~dp0project-manager.exe" (
    echo project-manager.exe was not found beside this launcher.
    pause
    exit /b 1
)
start "" /D "%~dp0" "%~dp0project-manager.exe" %*
exit /b 0
