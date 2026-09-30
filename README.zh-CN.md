# FWOK

FWOK 是一个基于 Fyrox 的游戏与编辑器工程，集成 Lua 脚本运行时和 MCP 桥接层，用于检查和编辑运行中的项目。仓库由根目录 Rust 工作区以及独立的 `lua/`、`mcp/` 工作区组成。

[English](README.md)

## 工程结构

| 路径 | 用途 |
| --- | --- |
| `game/` | 游戏插件，负责 Lua 运行时启动、每帧更新和输入/UI 事件接入。 |
| `game-dylib/` | 用于 Fyrox 热重载的游戏动态库构建。 |
| `executor/` | 独立游戏启动器，可动态加载游戏插件。 |
| `editor/` | FWOK 编辑器程序，预置游戏插件、Lua 编辑器绑定和 MCP 编辑器插件。 |
| `lua/lua-plugin/` | Lua 资源、组件/运行时宿主，以及场景和 UI 操作绑定。 |
| `lua/lua-tool/` | 离线绑定元数据/生成器和 Lua UI 示例资源准备工具。 |
| `mcp/mcp-bridge/` | MCP stdio 服务端，向 MCP 客户端提供工具，并通过本地桥接转发编辑器命令。 |
| `mcp/fyrox-mcp/` | Fyrox `EditorPlugin` 桥接端，执行场景、节点、资源、UI、摄像机、脚本和编辑器操作。 |
| `data/` | 项目资源、序列化场景/UI、运行时配置、字体和 Lua 脚本；项目脚本位于 `data/scripts/`。 |
| `scripts/` | Windows PowerShell 构建、启动、MCP 部署和 Lua 工具脚本。 |

`lua/` 和 `mcp/` 是 Git 子模块，也是独立 Cargo 工作区；根工作区不会把它们纳入成员。当前所有工作区都从 `https://github.com/DrYaling/Fyrox.git` 的 `master` 分支解析 Fyrox 源码；根工作区使用带 vendored Lua 5.4 的 `mlua` 0.12.1。

## 环境要求

- 与本工程兼容的 Rust 工具链和 Cargo。
- 可访问 Fyrox Git 仓库的网络环境。
- 提供的 PowerShell 启动和热重载脚本面向 Windows；各 Rust crate 可在其依赖支持的平台单独构建。

## 构建与运行

运行独立游戏启动器：

```powershell
cargo run -p executor
```

构建并启动 FWOK 编辑器。编辑器默认打开 `data/rpg_level.rgs`：

```powershell
cargo build -p editor
.\target\debug\editor.exe --project-directory .
```

Windows 热重载流程使用脚本构建匹配的 Fyrox 和游戏产物：

```powershell
.\scripts\run-editor.ps1
```

编辑器会依次尝试从 `data/editor/mcp-bridge.exe`、`target/debug/mcp-bridge.exe` 和 `target/release/mcp-bridge.exe` 启动 MCP stdio 服务端。可设置 `MCP_BRIDGE_BIN` 指定其他路径。MCP 服务端与编辑器插件通过 `127.0.0.1:6501` 的本地桥接通信。构建并部署服务端：

```powershell
.\scripts\build-mcp-bridge.ps1
```

脚本会把可执行文件复制到 `data/editor/mcp-bridge.exe`。

## Lua 运行时

游戏插件从 `data/fyrox-lua.toml` 读取 Lua 项目配置；未配置脚本根目录时默认使用 `data/scripts`。Lua 脚本通过已注册的 `LuaComponent` 和 Lua 脚本资源挂接到场景节点，运行时由游戏插件托管，并接入游戏更新和输入/UI 事件流程。

绑定层通过经过校验的句柄解析已有 Fyrox 场景和 UI 资源。应先在 Fyrox 资源中配置节点和控件，再由 Lua 查找并操作它们。`lua-tool` 提供离线绑定生成和序列化 Lua UI 示例准备功能：

```powershell
cargo run --manifest-path lua/Cargo.toml --bin lua-tool -- --help
cargo run --manifest-path lua/Cargo.toml --bin prepare_ui_demo
```

## MCP 集成

MCP 服务端通过 stdio 提供 MCP JSON-RPC，并把编辑器工作转发给编辑器内的 `McpEditorPlugin`。工具覆盖项目和场景检查、节点与摄像机管理、场景/Prefab/UI/资源操作、`data/scripts` 下的脚本文件操作、编辑器撤销/重做/播放/停止、日志、截图以及顺序批处理。需要编辑器状态的操作都会经过编辑器插件和主线程命令路径执行。

直接构建服务端：

```powershell
cargo build --manifest-path mcp/Cargo.toml -p mcp-bridge
```

直接连接 MCP 客户端时，应在启用桥接插件的 FWOK 编辑器运行期间，以 stdio 方式启动 `mcp-bridge`。

## 测试

各 Cargo 工作区分别测试：

```powershell
cargo test --workspace
cargo test --manifest-path lua/Cargo.toml --workspace
cargo test --manifest-path mcp/Cargo.toml --workspace
```

根工作区测试不会包含独立的 Lua 和 MCP 工作区。
