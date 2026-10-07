@echo off
setlocal
for %%I in ("%~dp0.") do set "PACKAGE_ROOT=%%~fI"
if not exist "%PACKAGE_ROOT%\editor.exe" (
    echo editor.exe was not found beside this launcher.
    pause
    exit /b 1
)
if not exist "%PACKAGE_ROOT%\data\rpg_level.rgs" (
    echo Packaged project data was not found beside this launcher.
    pause
    exit /b 1
)
start "" /D "%PACKAGE_ROOT%" "%PACKAGE_ROOT%\editor.exe" --project-directory "%PACKAGE_ROOT%" %*
exit /b 0
