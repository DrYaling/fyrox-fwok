# FWOK

FWOK is a Fyrox game and editor project with an embedded Lua runtime and an MCP bridge. It contains root Rust crates plus independent lua/ and mcp/ Cargo workspaces.

[Chinese version](README.zh-CN.md)

## Components

- game and game-dylib: game plugin and hot-reload library.
- executor: standalone game runner.
- editor: FWOK editor with Lua bindings and MCP plugin.
- lua/lua-plugin: Lua resources, runtime host, components, and scene/UI bindings.
- lua/lua-tool: offline binding generator and UI demo tool.
- mcp/mcp-bridge: MCP stdio server.
- mcp/fyrox-mcp: Fyrox editor command plugin.
- data/scripts: project Lua scripts.

The workspaces resolve Fyrox from https://github.com/DrYaling/Fyrox.git (master) and use mlua 0.12.1 with vendored Lua 5.4.

## Requirements

- Compatible Rust toolchain and Cargo.
- Network access to the Fyrox Git repository.
- Windows for the PowerShell scripts. The hot-reload scripts still use a sibling `../Fyrox` checkout for their shared target directory and `fyroxed` build.

## Build and run

```powershell
cargo run -p executor
cargo build -p editor
.\target\debug\editor.exe --project-directory .
```

Use `.\scripts\run-editor.ps1` for hot reload. The editor searches for `mcp-bridge.exe` in `data/editor`, `target/debug`, and `target/release`. Set `MCP_BRIDGE_BIN` to override the path. The bridge listens on `127.0.0.1:6501`.

## Lua

The game plugin reads `data/fyrox-lua.toml` and defaults to `data/scripts`. Lua scripts use registered `LuaComponent` and Lua resources.

```powershell
cargo run --manifest-path lua/Cargo.toml --bin lua-tool -- --help
cargo run --manifest-path lua/Cargo.toml --bin prepare_ui_demo
```

## MCP

`mcp-bridge` provides MCP JSON-RPC over stdio and forwards scene, node, camera, UI, asset, script, editor, log, screenshot, and batch operations to `McpEditorPlugin`.

```powershell
cargo build --manifest-path mcp/Cargo.toml -p mcp-bridge
```

## Tests

```powershell
cargo test --workspace
cargo test --manifest-path lua/Cargo.toml --workspace
cargo test --manifest-path mcp/Cargo.toml --workspace
```
