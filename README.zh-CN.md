# FWOK

FWOK 是一个基于 Fyrox 的游戏与编辑器工程，集成 Lua 脚本运行时和 MCP 桥接层，用于检查和编辑运行中的项目。仓库由根目录 Rust 工作区以及独立的 `lua/`、`mcp/` 工作区组成。

[English](README.md)

## 工程结构

| 路径 | 用途 |
| --- | --- |
| `game/` | 游戏插件，负责 Lua 运行时启动、每帧更新和输入/UI 事件接入。 |
| `game-dylib/` | 用于 Fyrox 热重载的游戏动态库构建。 |
| `executor/` | 独立游戏启动器，可动态加载游戏插件。 |
| `executor-android/` | `cargo-apk` 使用的 Android native activity 启动器。 |
| `editor/` | FWOK 编辑器程序，预置游戏插件、Lua 编辑器绑定和 MCP 编辑器插件。 |
| `lua/lua-plugin/` | Lua 资源、组件/运行时宿主，以及场景和 UI 操作绑定。 |
| `lua/lua-tool/` | 离线绑定元数据和业务注册器生成工具。 |
| `mcp/mcp-bridge/` | MCP stdio 服务端，向 MCP 客户端提供工具，并通过本地桥接转发编辑器命令。 |
| `mcp/fyrox-mcp/` | Fyrox `EditorPlugin` 桥接端，执行场景、节点、资源、UI、摄像机、脚本和编辑器操作。 |
| `data/` | 项目资源、序列化场景/UI、运行时配置、字体和 Lua 脚本；项目脚本位于 `data/scripts/`。 |
| `scripts/` | Windows PowerShell 构建、启动、MCP 部署和 Lua 工具脚本。 |

`lua/` 和 `mcp/` 是 Git 子模块，也是独立 Cargo 工作区；根工作区不会把它们纳入成员。当前所有工作区都从 `https://github.com/DrYaling/Fyrox.git` 的 `master` 分支解析 Fyrox 源码；根工作区使用带 vendored Lua 5.4 的 `mlua` 0.12.1。

FWOK 是公开的 Lua/MCP 验证项目。公开的 `game/` 和 `data/` 只保留通用运行时实验、基准与压力测试以及 MCP 测试资源。Rocky 商业项目代码和资源保存在本地 Git 忽略目录 `game_rocky/`、`data_rocky/` 中，不属于根 Cargo workspace，也不能加入版本控制。本地启动 Rocky 客户端：`rtk cargo run --manifest-path game_rocky/Cargo.toml --example executor-rocky`。

## 环境要求

- 与本工程兼容的 Rust 工具链和 Cargo。
- 可访问 Fyrox Git 仓库的网络环境。
- 提供的 PowerShell 启动和热重载脚本面向 Windows；热重载脚本仍使用同级目录 `../Fyrox` 的共享产物目录并从该目录构建 `fyroxed`。各 Rust crate 可在其依赖支持的平台单独构建。

## 构建与运行

运行独立游戏启动器：

```powershell
rtk cargo run -p executor
rtk powershell -NoProfile -ExecutionPolicy Bypass -File scripts/build-game-package.ps1 -Profile release -RunSmoke
```

`build-game-package.ps1` 会在 `game-package/release/` 生成真正的独立游戏包，
其中包含 `executor.exe`、完整 `data/`、可用时复制的 Visual C++ 运行库和
`start-game.cmd`。启动器以包目录作为工作目录，不依赖源码目录或编辑器。

构建 FWOK 编辑器并生成可直接分发的 `release/` 目录。目录包含 `project-manager.exe`、`editor.exe`、项目 `data/`、MCP bridge、`vcruntime140.dll` 和 CMD 启动脚本：

```powershell
rtk powershell -NoProfile -ExecutionPolicy Bypass -File scripts/build-editor.ps1
```

美术和策划只需复制 `release/` 目录，在 Windows 上双击 `release/start.cmd` 或 `release/start-project-manager.cmd` 即可打开 Fyrox Project Manager，不需要安装 Rust、Cargo 或 Fyrox 源码。

`release/start-editor.cmd` 启动发布包内的 editor，`release/start-editor-workspace.cmd` 启动同一个 editor 并把当前工作区根目录作为项目目录。这些 CMD 只启动已有 exe，不会编译。需要从开发机启动已打包编辑器时可以使用：

```powershell
.\scripts\run-editor.ps1
```

`run-editor.ps1` 默认启动 `release/editor.exe`，也可以通过 `-Project <path>` 打开包含 `data/rpg_level.rgs` 的其他项目目录。

Fyrox Android 使用 `executor-android/` 中的 native activity 入口，不能把
Windows exe 改名为 APK。准备 Android SDK、`cargo-apk` 和设备/模拟器后执行：

```powershell
rtk rustup target add aarch64-linux-android
rtk cargo install cargo-apk
rtk powershell -NoProfile -ExecutionPolicy Bypass -File scripts/build-android-package.ps1 -Run
```

`McpEditorPlugin` 启动时会自动搜索 MCP 工具并在插件退出时回收自己启动的进程；Editor 可执行入口只需注册插件。除 `MCP_TOOL_BIN`/`MCP_BRIDGE_BIN` 覆盖路径外，默认搜索 `data`、`data/mcp`、`data/editor`、`mcp/target/debug`、`mcp/target/release`、`mcp/mcp-bridge/target/debug`、`mcp/mcp-bridge/target/release`、`target/debug` 和 `target/release` 下的 `mcp-tool.exe`/`mcp-bridge.exe`，并检查相邻参考工程路径。如果 `127.0.0.1:6501` 已有 Editor bridge，插件不会重复拉起工具。设置 `MCP_TOOL_AUTOSTART=0` 可关闭自动启动。MCP 服务端与编辑器插件通过 `127.0.0.1:6501` 的本地桥接通信。构建并部署服务端：

```powershell
.\scripts\build-mcp-bridge.ps1
```

脚本会把可执行文件复制到 `data/editor/mcp-bridge.exe`。

## Lua 运行时

游戏插件从 `data/fyrox-lua.toml` 读取 Lua 项目配置；未配置脚本根目录时默认使用 `data/scripts`。Lua 脚本通过已注册的 `LuaComponent` 和 Lua 脚本资源挂接到场景节点，运行时由游戏插件托管，并接入游戏更新和输入/UI 事件流程。

绑定层通过经过校验的句柄解析已有 Fyrox 场景和 UI 资源。应先在 Fyrox 资源中配置节点和控件，再由 Lua 查找并操作它们。`lua-tool` 提供离线 Rust AST 绑定生成和基于 TOML 的 UI 合同审计功能：

```powershell
rtk cargo run --manifest-path lua/Cargo.toml --bin lua-tool -- --help
rtk cargo run --manifest-path lua/Cargo.toml --bin lua-tool -- --config data/editor/lua/lua-bindings.toml
```

Lua 业务绑定和 UI 合同审计使用配置驱动生成器。项目 API 与 UI 合同由项目自己的 `data/editor/lua/lua-bindings.toml` 描述；`lua-plugin` 只提供通用运行时，不包含游戏模块或 UI 名称。Editor 仅在配置缺失时创建空清单，不覆盖已有配置。完整说明见 [lua-tool 配置驱动绑定](Docs/lua-tool配置驱动绑定.md)：

```powershell
.\scripts\generate-lua-bindings.ps1
```

运行时的通用 Lua Bridge、UI/Scene 命令队列、句柄注册表、队列上限和命令应用均由 `lua-plugin` 的 `LuaPluginHost` 持有。Game 只负责读取项目配置、初始化场景和 Lua 宿主、调用生成的业务注册器，并把 `ui.load` 的资源请求交给 Fyrox 资源系统。

Lua 启动阶段会输出 `[LuaPerf]` 耗时日志，包含 VM/API 初始化、`main.lua` 加载、`main.lua:on_awake`、`main.lua:start`，以及全部场景脚本 `start` 的总耗时。例如：`main_load_ms`、`main_awake_ms`、`main_start_ms`、`lua_start_ms` 和 `lua_init_ms`。回调错误仍会被恢复，并在对应字段中标记 `error_recovered`。详细字段说明见 [Lua Runtime 审计](Docs/lua-binding-runtime-audit.md)。

## MCP 集成

MCP 服务端通过 stdio 提供 MCP JSON-RPC，并把编辑器工作转发给编辑器内的 `McpEditorPlugin`。工具覆盖项目和场景检查、节点与摄像机管理、场景/Prefab/UI/资源操作、`data/scripts` 下的脚本文件操作、编辑器撤销/重做/播放/停止、日志、截图以及顺序批处理。需要编辑器状态的操作都会经过编辑器插件和主线程命令路径执行。

直接构建服务端：

```powershell
rtk cargo build --manifest-path mcp/Cargo.toml -p mcp-bridge
```

直接连接 MCP 客户端时，应在启用桥接插件的 FWOK 编辑器运行期间，以 stdio 方式启动 `mcp-bridge`。

## 微信小游戏目标检查

当前工程的可发布目标是 Windows 桌面版。微信小游戏不是把 `release/` 目录直接上传：腾讯自研引擎文档要求使用 Emscripten 生成浏览器产物 `game.js`、`game.wasm`、`game.data`，先用本地 HTTP 服务验证，再通过 `wx-transformer` 转换为小游戏目录；主包 JavaScript 还必须控制在 2 MB 以内。

`F:\WorkSpaceNew\godot` 中的本地工作流提供了 Emscripten SDK，预检脚本会自动加载
其 `emsdk_env.bat`。FWOK 现在有独立的 `executor-wasm` 浏览器入口，已经生成
`wechat-minigame/browser` 下的 `wasm32-unknown-unknown` 浏览器包。腾讯官方快速开始页
提供的 `wx-transformer-v1.0.26-win.exe` 也已下载并成功转换出
`wechat-minigame/converted/minigame`。

转换目录已经通过 JSON 和 JavaScript 语法检查，但仍需填入真实 AppID，随后在微信开发者
工具和真机上验证。`-Build` 探测是另一条链路：直接把桌面 executor 链接到
`wasm32-unknown-emscripten` 时仍会暴露 wasm-bindgen 的 `__wbg_*` ABI，因此当前浏览器包
不使用这条产物。Windows `release/` 目录不能直接当作小游戏包上传。

```powershell
rtk powershell -NoProfile -ExecutionPolicy Bypass -File scripts/check-wechat-minigame.ps1
# 添加 -Build 可运行 Emscripten 兼容性探测（探测结果不是小游戏导出物）。
rtk powershell -NoProfile -ExecutionPolicy Bypass -File scripts/check-wechat-minigame.ps1 -Build
# 使用腾讯快速开始页下载的官方转换器：
rtk powershell -NoProfile -ExecutionPolicy Bypass -File scripts/convert-wechat-minigame.ps1 -TransformerPath F:\\WorkSpaceNew\\wx-transformer-v1.0.26\\wx-transformer-v1.0.26-win.exe -NoCompress -ForceSingleThread
```

官方参考：

- [自研引擎导出](https://developers.weixin.qq.com/minigame/dev/guide/game-engine/common-adaptation/Design/CustomEngineExport.html)
- [微信小游戏快速转换指南](https://developers.weixin.qq.com/minigame/dev/guide/game-engine/common-adaptation/Design/QuickStart.html)
- [小游戏兼容性评估](https://developers.weixin.qq.com/minigame/dev/guide/game-engine/unity-webgl-transform/Design/Evaluation.html)

## 测试

各 Cargo 工作区分别测试：

```powershell
rtk cargo test --workspace
rtk cargo test --manifest-path lua/Cargo.toml --workspace
rtk cargo test --manifest-path mcp/Cargo.toml --workspace
```

根工作区测试不会包含独立的 Lua 和 MCP 工作区。
