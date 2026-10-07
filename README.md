# FWOK

FWOK is a Fyrox game and editor project with an embedded Lua runtime and an MCP bridge. It contains root Rust crates plus independent lua/ and mcp/ Cargo workspaces.

[Chinese version](README.zh-CN.md)

## Components

- game and game-dylib: game plugin and hot-reload library.
- executor: standalone game runner.
- executor-android: native Android activity launcher used by cargo-apk.
- editor: FWOK editor with Lua bindings and MCP plugin.
- lua/lua-plugin: Lua resources, runtime host, components, and scene/UI bindings.
- lua/lua-tool: offline binding metadata and business registration generator.
- mcp/mcp-bridge: MCP stdio server.
- mcp/fyrox-mcp: Fyrox editor command plugin.
- data/scripts: project Lua scripts.

FWOK is the open Lua/MCP validation project. The public `game/` and `data/`
trees contain generic runtime experiments, benchmarks, stress tests, and MCP
fixtures. Local commercial Rocky code and assets live in the Git-ignored
`game_rocky/` and `data_rocky/` directories; they are outside the root Cargo
workspace and must not be added to source control. Run that local client with
`rtk cargo run --manifest-path game_rocky/Cargo.toml --example executor-rocky`.

The workspaces resolve Fyrox from https://github.com/DrYaling/Fyrox.git (master) and use mlua 0.12.1 with vendored Lua 5.4.

## Requirements

- Compatible Rust toolchain and Cargo.
- Network access to the Fyrox Git repository.
- Windows for the PowerShell scripts. The hot-reload scripts still use a sibling `../Fyrox` checkout for their shared target directory and `fyroxed` build.

## Build and run

```powershell
rtk cargo run -p executor
rtk powershell -NoProfile -ExecutionPolicy Bypass -File scripts/build-game-package.ps1 -Profile release -RunSmoke
rtk powershell -NoProfile -ExecutionPolicy Bypass -File scripts/build-editor.ps1
```

`build-game-package.ps1` creates the standalone game package in
`game-package/release/`. It contains `executor.exe`, the complete `data/`
directory, the Visual C++ runtime when available, and `start-game.cmd`. The
launcher uses the package directory as its working directory, so it does not
depend on the source checkout or an editor installation.

The build script creates a self-contained Windows package in `release/`. It contains `project-manager.exe`, `editor.exe`, the project `data/`, the MCP bridge, `vcruntime140.dll`, and CMD launchers. Artists and designers can copy that directory to a Windows machine and double-click `release/start.cmd` (or `start-project-manager.cmd`) to open the Fyrox Project Manager; Rust, Cargo, and the Fyrox source checkout are not needed there.

`release/start-editor.cmd` opens the packaged project with `release/editor.exe`. `release/start-editor-workspace.cmd` opens `editor.exe` with the current workspace root as `--project-directory`, which is useful for developers. These CMD launchers only start already-built executables and never compile. `.\scripts\run-editor.ps1` remains the developer convenience wrapper; pass `-Project <path>` to open another directory containing `data/rpg_level.rgs`.

Fyrox Android builds use a native activity launcher in `executor-android/`,
not the Windows executable. With the Android SDK, `cargo-apk`, and a device or
emulator configured, build and run it with:

```powershell
rtk rustup target add aarch64-linux-android
rtk cargo install cargo-apk
rtk powershell -NoProfile -ExecutionPolicy Bypass -File scripts/build-android-package.ps1 -Run
```

`McpEditorPlugin` owns MCP tool discovery, startup, and shutdown; the editor executable only registers the plugin. It checks `MCP_TOOL_BIN`/`MCP_BRIDGE_BIN`, the project data and MCP build directories, and the sibling reference checkout. Set `MCP_TOOL_AUTOSTART=0` to disable startup. The bridge listens on `127.0.0.1:6501`.

## Lua

The game plugin reads `data/fyrox-lua.toml` and defaults to `data/scripts`. Lua scripts use registered `LuaComponent` and Lua resources.

```powershell
rtk cargo run --manifest-path lua/Cargo.toml --bin lua-tool -- --help
rtk cargo run --manifest-path lua/Cargo.toml --bin lua-tool -- --config data/editor/lua/lua-bindings.toml
```

The Editor creates an empty project-owned manifest at `data/editor/lua/lua-bindings.toml` when none exists. Add project API bindings and UI contracts there; the runtime plugin contains no game-specific definitions. See [the configuration guide](Docs/lua-tool配置驱动绑定.md).

The runtime `LuaPluginHost` owns the generic Lua bridge, UI/Scene command queues, handle registries, queue limits, and command application. The public game host loads the generic experiment config, initializes the scene and Lua host, and adapts `ui.load` requests to Fyrox's resource system.

The tool is business agnostic: Rust AST scanning, manifest validation, catalog generation, registration adapter generation, and generic UI contract auditing live in separate `lua-tool` modules. Game and product names are supplied by TOML. Generic UI/Scene bridge command application is implemented by `lua-plugin/src/host.rs`; Game keeps project initialization and business API assembly.

## MCP

`mcp-bridge` provides MCP JSON-RPC over stdio and forwards scene, node, camera, UI, asset, script, editor, log, screenshot, and batch operations to `McpEditorPlugin`.

```powershell
rtk cargo build --manifest-path mcp/Cargo.toml -p mcp-bridge
```

## WeChat Mini Game target check

The current release target is Windows desktop. Tencent's custom-engine workflow requires an Emscripten browser export (`game.js`, `game.wasm`, and `game.data`), local HTTP validation, and a `wx-transformer` conversion step; the main JavaScript package must stay within 2 MB.

The local Godot workflow at `F:\WorkSpaceNew\godot` supplies the Emscripten SDK;
the preflight script loads `emsdk_env.bat` automatically. FWOK now has a
separate `executor-wasm` browser entry and the `wasm32-unknown-unknown` package
is generated under `wechat-minigame/browser`. The official Tencent
`wx-transformer-v1.0.26-win.exe` was also downloaded from the QuickStart link
and successfully produced `wechat-minigame/converted/minigame`.

The converted directory passes JSON and JavaScript syntax checks. It still
needs a real AppID in `project.config.json` and verification in the WeChat
Developer Tools and on a device. The optional `-Build` probe is separate: a
direct desktop `wasm32-unknown-emscripten` link currently exposes the
wasm-bindgen `__wbg_*` ABI and is not used as the browser package. Do not treat
the Windows `release/` directory as a Mini Game package.

```powershell
rtk powershell -NoProfile -ExecutionPolicy Bypass -File scripts/check-wechat-minigame.ps1
# Add -Build to run the Emscripten compatibility probe (it is not an export).
rtk powershell -NoProfile -ExecutionPolicy Bypass -File scripts/check-wechat-minigame.ps1 -Build
# With the official transformer downloaded from Tencent's QuickStart page:
rtk powershell -NoProfile -ExecutionPolicy Bypass -File scripts/convert-wechat-minigame.ps1 -TransformerPath F:\\WorkSpaceNew\\wx-transformer-v1.0.26\\wx-transformer-v1.0.26-win.exe -NoCompress -ForceSingleThread
```

Official references:

- [Custom engine export](https://developers.weixin.qq.com/minigame/dev/guide/game-engine/common-adaptation/Design/CustomEngineExport.html)
- [Mini Game quick conversion guide](https://developers.weixin.qq.com/minigame/dev/guide/game-engine/common-adaptation/Design/QuickStart.html)
- [Compatibility evaluation](https://developers.weixin.qq.com/minigame/dev/guide/game-engine/unity-webgl-transform/Design/Evaluation.html)

## Tests

```powershell
rtk cargo test --workspace
rtk cargo test --manifest-path lua/Cargo.toml --workspace
rtk cargo test --manifest-path mcp/Cargo.toml --workspace
```
