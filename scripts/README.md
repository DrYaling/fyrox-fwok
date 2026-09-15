# FWOK Build and Launch Scripts

Scripts set `RUSTFLAGS` only for the current PowerShell process and its child processes.

## Initial build

```powershell
.\Scripts\build-editor.ps1
.\Scripts\run-project-manager.ps1 -Build
```

`build-editor.ps1` builds the dynamically linked editor in `..\Fyrox\target\release`.
The editor, game plugin, and executor share this target directory so their Rust ABI stays consistent.

## Project Manager

```powershell
.\Scripts\run-project-manager.ps1
```

After importing a project, enable **Hot Reload**:

- **Edit** builds `game_dylib` and runs `fyroxed.exe --project-directory <project>`.
- **Run** builds `game_dylib` and starts `executor` with the `dylib` feature.

## Directly launch the editor

```powershell
.\Scripts\run-editor.ps1
```

The script builds the hot-reload library and then starts the editor with the project directory passed as `--project-directory`. With no argument it uses the FWOK repository root. A different project can be supplied:

```powershell
.\Scripts\run-editor.ps1 -Project F:\WorkSpaceNew\Fyrox\another-project
```

## Dependency checks

```powershell
Test-Path ..\Fyrox\target\release\game_dylib.dll
Test-Path ..\Fyrox\target\release\fyroxed.exe
Test-Path ..\Fyrox\target\release\fyrox_dylib.dll
```
若 Windows 提示 `fyroxed.exe - 无法找到入口点`，通常是 `fyroxed.exe` 与 `fyrox_dylib.dll` 版本不一致。重新执行 `.scripts\run-editor.ps1`（脚本会按“游戏 DLL -> Editor”顺序重建），不要单独替换其中一个二进制。
