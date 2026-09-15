# Lua 不同模式工作流程与实现原理

本文说明 FWOK 当前 Lua 集成在编辑器开发、完整绑定调试和发布打包三种场景中的真实工作流程。引擎 API 采用手写注册，不依赖 Lua 源码扫描；入口和基础类型见 `Docs/lua-manual-bindings.md`。这里的“模式”由 `fyrox-lua.toml` 的 `binding_mode` 配置和 Rust 编译目标共同决定。

## 1. 模式总览

| 场景 | 配置值 | 实际模式 | 绑定策略 | 热更 |
| --- | --- | --- | --- | --- |
| Editor/Debug 默认 | `EditorReflection` 或缺省 | `EditorReflection` | 轻量反射入口；当前游戏示例保留少量兼容 API | 支持脚本级重载入口 |
| Editor/Debug 可选 | `PackageFull` | `PackageFull` | 注册 `LuaGameApi::register` 的完整稳定 API | 支持脚本级重载入口 |
| Release/发布 | 任意 | **强制 `PackageFull`** | 完整绑定，配置不能降级为反射 | 不依赖编辑器热更 |

强制规则位于 `LuaConfig::effective_binding_mode()`：编译了 `editor` feature 的编辑器产物返回配置值；没有 `editor` feature 的产物始终返回 `PackageFull`。因此 Editor 可以使用 Release 优化构建而仍保持反射模式，发布包也不会因为误留 `EditorReflection` 配置而缺少 API。

### 模块与编译 feature

`lua-binding/src/lib.rs` 只声明模块和 re-export 公共类型，具体实现分布如下：

| 模块 | 职责 | feature |
| --- | --- | --- |
| `config.rs` | TOML 配置、绑定模式、重载模式 | 公共 |
| `events.rs` | Rust/Lua 事件队列 | 公共 |
| `bindings/` | 手写引擎类型注册和分类元数据 | 公共 |
| `api.rs` | 完整/反射绑定注册协议 | 公共，反射方法受 `editor` 控制 |
| `reflection.rs` | `reflection.call` 适配器 | 仅 `editor` |
| `runtime.rs` | Lua VM、生命周期、事件分发和脚本重载 | 公共，按 feature 选择注册路径 |

编辑器构建启用 `lua-editor`，它同时包含 `lua-binding/editor` 与 `lua-binding/package`，所以编辑器内可以在反射和完整绑定之间切换。发布构建仅启用 `lua-package`，不会编译 `reflection.rs`：

```powershell
.\scripts\build-package.ps1
```

单独验证绑定 crate 时必须同时选择 Lua 后端：

```powershell
rtk cargo check -p lua-binding --no-default-features --features "lua54,editor"
rtk cargo check -p lua-binding --no-default-features --features "lua54,package" --release
```

## 2. 配置加载流程

`game` 插件初始化时读取项目根目录的 `fyrox-lua.toml`：

```toml
script_root = "data/scripts"
font_path = "data/SimHei.ttf"
jit = false
reload_mode = "Immediate"
enabled = true
binding_mode = "EditorReflection"
```

流程如下：

1. 文件不存在时使用 `LuaConfig::default()`。
2. TOML 缺少字段时使用 serde 默认值；`binding_mode` 缺省为 `EditorReflection`。
3. `script_root` 作为 Lua 文件发现根目录；这里只枚举脚本文件，不分析 API 或生成绑定。
4. `effective_binding_mode()` 根据是否编译 `editor` feature 计算最终模式。
5. `enabled`、`jit`、`reload_mode` 当前已进入配置模型，但其中 JIT 和自动文件监听尚未连接到运行时行为，见“当前限制”。

## 3. Lua VM 初始化

`LuaRuntime::new(config, api)` 执行以下步骤：

1. 创建独立的 `mlua::Lua` 状态。每个 `LuaRuntime` 拥有自己的全局表、registry 和脚本实例。
2. 创建线程安全的 `EventManager`，并把它传给 API 注册器。
3. 根据最终绑定模式调用：
   - `EditorReflection`：调用 `LuaGameApi::register_reflection`。
   - `PackageFull`：调用 `LuaGameApi::register`。
4. 枚举 `script_root` 下全部 `.lua` 文件并加载脚本。
5. 每个文件执行 chunk，要求返回 class table。
6. 从 class 读取 `new`，用 `new(class)` 创建独立实例。
7. 校验并调用强制生命周期 `on_awake(self)`。
8. 将实例保存为 `RegistryKey`，避免 Rust 每帧持有 Lua table 的借用。

脚本错误会以 `mlua::Error` 返回，插件层将其转换为 Fyrox `GameError`。

## 4. 编辑器反射模式

### 4.1 设计目标

编辑器模式不要求为所有 Fyrox 节点、组件、资源和方法生成 mlua `UserData`。宿主只需实现：

```rust
pub trait LuaReflection {
    fn call(&self, object: &str, method: &str, args: Vec<String>)
        -> Result<String, String>;
}
```

`get_method` 会返回可缓存的 Lua 闭包，脚本可以按 tolua 习惯先查找方法再重复调用；`findtype`、`getmethod`、`getfield`、`setfield`、`getproperty` 是对应的无下划线兼容别名。方法闭包背后仍必须由宿主提供显式的参数校验和 Rust 调用 shim。

运行时可注册：

```lua
local result = reflection.call("Scene/Player", "set_position", "10", "2")
```

反射调用的原理是：Lua 只传递对象标识、方法名和参数；Rust 适配器通过对象注册表/反射信息查找目标，再做白名单校验、参数转换和调用。推荐对象标识由类型名、generational handle 索引和代数组成。Lua 不保存 Rust 指针，也不直接访问完整 `SceneGraph`。

Fyrox 的 `Reflect` 提供类型 UUID、字段枚举、字段读写和类型替换，不提供类似 CLR `MethodInfo.Invoke` 的任意 Rust 方法调用。因此“未绑定对象可调用”只能理解为：对象类型可以不生成完整 userdata，但该类型的方法必须已在编辑器反射后端登记 descriptor/shim；没有 descriptor 的 Rust public 方法不能凭字符串安全调用。字段访问也必须经过宿主上下文，不能把 `SceneGraph` 或 `UserInterface` 的可变借用存进 Lua。

### 4.2 当前项目状态

`lua-binding` 已提供 `LuaReflection` 和 `reflection.call` 注册能力。`game` 当前的 `register_reflection` 为了保持背包/聊天示例可运行，暂时复用了少量稳定 API 注册；这属于兼容层，不代表已经完成全部 Fyrox 反射方法映射。真正的 Editor 适配器应在 Editor 插件侧实现对象路径解析和方法白名单。

### 4.3 适用范围

- 修改 Inspector 中选中节点的少量属性。
- 调试工具调用场景对象方法。
- 编辑器预览脚本，不要求发布包携带完整元数据。

反射模式不适合依赖高频调用的游戏主循环；高频逻辑应使用发布绑定或批量 API。

## 5. 完整绑定模式

完整模式调用 `LuaGameApi::register`，由游戏库显式注册稳定 API。目前示例包括：

```lua
game.position()
game.add_item("coin")
ui.set_text("inventory_label", "背包")
ui.show("inventory_label", true)
chat.append("hello")
inventory.items()
events.emit("custom.event", "payload")
```

完整绑定的特点：

- API 名称和参数由 Rust 编译期确定。
- 不依赖 Editor 进程或反射元数据。
- 发布包只携带运行时所需绑定，调用路径更稳定、可审计。
- 仍然禁止裸指针、任意文件系统和无限制场景图访问。

编辑器中把 `binding_mode` 改为 `PackageFull` 即可用同一套完整绑定进行调试；该选择不会改变 Rust 游戏库的编译方式。

## 6. 发布/打包流程

发布流程必须使用 Release 配置构建 `game` 或 `game_dylib`。即使项目配置仍为：

```toml
binding_mode = "EditorReflection"
```

package-only 构建下 `effective_binding_mode()` 仍返回 `PackageFull`。建议发布脚本额外执行配置审计，明确把配置写成：

```toml
binding_mode = "PackageFull"
```

这样可以让项目文件、构建日志和实际行为保持一致。发布包不应依赖 `reflection.call`、Editor 反射元数据或 Editor 进程。

## 7. 生命周期和事件时序

单个 Lua class 的时序为：

```text
加载 chunk
  -> new(class)
  -> on_awake(self)
  -> start(self)                 [插件 init 完成后调用一次]
  -> 每帧 dispatch_events()
  -> 每帧 update(self, dt)
  -> 插件退出时 on_destroy(self)
```

Fyrox UI 点击先由 Rust 收到，再通过 `EventManager::emit` 入队；下一帧 `dispatch_events()` 将事件广播给所有实例的 `on_event(name, payload)`。因此事件处理不会在 UI 回调中直接重入 Lua。

## 8. Tolua 风格脚本热更

`LuaRuntime::reload_script(path)` 是单文件替换入口：

1. 找到旧脚本的 `RegistryKey`。
2. 调用旧实例 `on_destroy`。
3. 删除旧 registry 引用，使 Lua GC 可以回收旧实例。
4. 重新执行文件并校验 `new`、`on_awake`。
5. 创建新实例并加入脚本列表。

该接口是“脚本级热更”，不会重新编译 Editor、Rust 游戏库或整个工程。当前尚未接入文件系统 watcher，因此调用方需要由 Editor/工具菜单检测文件变化后主动调用。`reload_mode` 目前用于规划 Immediate/OnStop 语义，自动监听和停止时批量替换仍需后续实现。

## 9. 安全和工程约束

反射入口必须使用对象/方法白名单和明确参数类型；完整绑定也必须只暴露稳定业务 API。Lua 状态中只保存可序列化业务数据，跨版本热更使用 `serialize_state`/`restore_state`，不保存 Rust 指针或临时闭包。脚本文件统一采用 class table，并且必须包含 `new(class)` 与 `on_awake(self)`。

## 10. 当前限制与后续工作

- Editor 真实 Fyrox 反射适配器尚未完成，当前游戏适配器仍保留少量兼容 API。
- `jit` 配置尚未启用 LuaJIT 后端；当前 workspace 使用 vendored Lua 5.4。
- `reload_mode` 尚未连接自动文件监听。
- 节点 Inspector 的 LuaComponent 添加、资源选择和参数编辑已实现；当前仍未实现热更时的实例状态迁移。
- 发布强制完整绑定已经由运行时规则覆盖，仍建议在 CI 中增加 Release 配置审计。

## 11. 手写绑定迁移说明

旧版本曾使用 `lua-binding-tool` 从脚本生成清单；该路径已移除，不再参与构建或运行时绑定。现在新增引擎类型必须修改 `lua-binding/src/bindings/`，并在 `register_engine_bindings` 中显式加入。

```powershell
手写绑定变更后运行 `rtk cargo check --workspace` 验证。
```

等价命令：

```powershell
rtk cargo check --workspace
```

当前不再把 Lua 源码扫描器作为绑定生成或发布裁剪依据。`LuaRuntime::new` 创建 VM 后调用静态 `register_engine_bindings`，再调用 `LuaGameApi::register`；脚本只按目录枚举和加载，不会因为脚本内容变化而改变绑定集合。后续若增加生成工具，也只能读取 Rust 显式导出描述符，不能从变量名猜测 Fyrox 类型。

### 显式导出边界

Lua 代码可以自由使用已注册 API，但不能通过变量名、字符串拼接或动态调用获得未注册接口。完整绑定的权限边界由 Rust 源码、编译 feature 和注册测试共同决定。

## 12. 宿主注册边界

Lua 资源加载器和 `LuaComponent` 构造器不属于 Fyrox 引擎核心初始化。启动顺序由宿主负责：

1. `executor` 或 `fyroxed` 创建引擎后调用 `lua_binding::register_fyrox_resources`；
2. 注册完成后再扫描项目资源注册表并加载游戏插件；
3. `game` 创建 `LuaRuntime`，注册游戏 API，并扫描实际 `SceneGraph` 中的 `LuaComponent`；
4. Fyrox 引擎本身不依赖 `lua-binding`，也不持有 Lua VM、registry 或绑定表。

编辑器 Inspector 的 Lua 类型编辑器只是编辑器 UI 适配层，负责资源选择和参数编辑，不执行 Lua 初始化。资源构造器和资源管理器 vtable 位于宿主进程，游戏 DLL 热重载时不会悬挂到已卸载的插件代码。
