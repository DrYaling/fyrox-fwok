# Fyrox Lua 集成方案

## 目标

`lua-binding` 提供独立 Lua 运行时，不修改或重新编译 Fyrox Editor；`game` 继续作为 Rust 游戏库，通过 `LuaGameApi` 注册游戏对象与函数。脚本位于 `data/scripts`，新增或修改 `.lua` 文件只触发脚本层加载。

## 配置

建议在项目根放置 `fyrox-lua.toml`：

```toml
script_root = "data/scripts"
jit = false
reload_mode = "Immediate" # 或 "OnStop"
enabled = true
binding_mode = "EditorReflection" # 编辑器默认；可改为 PackageFull
```

## 编辑器反射与发布绑定

绑定分为两条路径：编辑器/Debug 默认使用 `EditorReflection`，也可在配置中切换为 `PackageFull`；Release/发布构建无论配置文件如何设置，都会强制使用 `PackageFull`。`EditorReflection` 仅注册 `reflection.call(object, method, ...)`，由实现 `LuaReflection` 的编辑器适配器通过 Fyrox 反射系统按名称解析；`PackageFull` 注册完整稳定 API，供发布包和独立运行器使用。

反射层应限制对象和方法白名单以及参数类型，禁止暴露任意文件系统、裸指针或完整 SceneGraph。

## Tolua 风格热更

`LuaRuntime::reload_script(path)` 会先调用旧实例 `on_destroy`，移除 registry 引用，再加载新 chunk、校验 `new(class)` 与 `on_awake` 并创建新实例。脚本错误会返回 `mlua::Error`；生产环境应先在临时 Lua 状态验证，再交换实例。跨版本状态迁移可通过 `serialize_state`/`restore_state` 实现，不应保存旧 Rust 指针或闭包。

Editor 工具菜单应读写该文件，提供脚本根目录、JIT、重载时机和启用开关。第一版可先由 Project Manager/Editor 外部脚本编辑，第二版再实现 Editor 原生菜单窗口。

## 运行模型

`LuaRuntime` 启动时扫描根目录。每个脚本必须返回 class table，必须包含 `new(class)` 构造函数和 `on_awake(self)`；缺少任意一项都会拒绝加载。实例按顺序调用 `on_awake`、`start`、`update(dt)`、`on_event(name,payload)`、`on_destroy`。文件监听器检测变更后，Immediate 立即替换 chunk，OnStop 延迟到运行停止时处理。

## 固定脚本格式

```lua
local Player = {}; Player.__index = Player
function Player.new(class) return setmetatable({ speed = 180 }, class) end
function Player:on_awake() end
function Player:start() end
function Player:update(dt) end
function Player:on_event(name, payload) end
function Player:on_destroy() end
return Player
```

`new` 是构造函数，`on_destroy` 是析构阶段回调；Lua GC 仍由 VM 管理，业务资源释放必须放在 `on_destroy` 中。`on_awake` 是唯一强制生命周期函数。

## EventManager

Rust 使用 `EventManager::emit` 发布事件，运行时每帧 `drain` 并调用脚本 `on_event`。事件 payload 第一版采用 JSON/RON 字符串，后续可升级为 Lua table。脚本不直接持有 Fyrox 内部对象，只通过稳定 API 操作 Handle。

## 当前 UI 示例

`game` 示例创建了背包标签、背包切换按钮、聊天窗口、输入框和发送按钮。控件事件通过 `EventManager` 转换为 `inventory.toggle` 与 `chat.send`，由 `data/scripts/ui_controller.lua` 处理。Lua 可调用：

```lua
ui.set_text("inventory_label", "背包：红宝石")
ui.show("inventory_label", false)
chat.append("你好")
inventory.items()
events.emit("custom.event", "payload")
```

### 审计结果

- 生命周期顺序为 `on_awake`（加载时一次）、`start`（启动时一次）、`update(dt)`（每帧）、`on_event`（事件到达）、`on_destroy`（插件退出）。
- `new(class)` 与 `on_awake` 缺失会拒绝脚本；实例状态隔离并由 Lua VM 管理。
- Rust/Lua 事件和 UI API 已有单元测试覆盖；文件热重载、JIT 后端和编辑器 Inspector 挂载仍属于后续阶段。

## Rust API 暴露原则

只暴露稳定、可验证的函数，例如 `game.position()`、`game.add_item(name)`、`input.is_down(action)`、`inventory.add(id,count)`。禁止直接暴露完整 Scene Graph、任意文件系统和裸指针；删除节点时必须让脚本对象失效。

## 分阶段计划

1. MVP：加载 Lua、执行 `on_update`、捕获错误（1-2 天）。
2. ScriptTrait 适配与节点挂载（3-5 天）。
3. EventManager、Transform/Input/Inventory API（3-7 天）。
4. Editor 资源发现、脚本选择器、导出属性（5-10 天）。
5. 文件监听、Immediate/OnStop、状态保存与恢复（5-10 天）。
6. 调试器、补全、沙箱、发布构建（2-4 周）。

## 评估结论

优先使用 Lua 5.4 + `mlua`；JIT 作为可选实验开关，不能假定所有平台可用。Rhai 可作为纯 Rust 备选，Zust/Rune/WASM 先做独立 PoC，不进入第一版核心路径。
