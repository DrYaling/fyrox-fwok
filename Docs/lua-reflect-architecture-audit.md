# Fyrox Lua Reflect 架构审计与重构方案

## 1. 审计范围与结论

本次审计基于 Rust、Fyrox `Reflect`、mlua 以及 `Docs/tolua-architecture.md`，检查了 `lua-binding`、`game`、`game-dylib` 和 `executor` 的注册、加载、生命周期、对象访问和构建模式。本文只给出设计方案，不修改现有实现。

当前实现可以完成基础 Lua VM 初始化、目录脚本加载、`LuaComponent` 实例化和生命周期调用，现有 9 个单元测试通过，workspace 可以编译。但它还不能被称为“基于 Fyrox Reflect 的通用 Lua 绑定系统”，也不满足“编辑器中未手写绑定的 Fyrox 对象和接口均可调用”的完整目标。

准确结论如下：

| 能力 | 当前状态 | 结论 |
| --- | --- | --- |
| Lua VM 初始化 | 已实现 | 可运行 |
| `data/scripts` 目录加载 | 已实现 | 可运行，但依赖进程当前目录 |
| `LuaComponent` 加载 | 已实现 | 可运行 |
| 生命周期 | 已实现基础调用 | 缺少状态机和局部错误隔离 |
| game 手写 API | 已实现少量 API | 可运行 |
| Button/Node2D/Node3D | 仅句柄包装 | 不能操作真实对象 |
| Fyrox Reflect 字段访问 | 仅 trait 占位 | 未实现 |
| Reflect 方法调用 | Fyrox Reflect 本身不提供通用方法反射 | 需要显式方法描述器 |
| 编辑器反射模式 | 接口外形存在 | 后端没有接到场景/UI 对象仓库 |
| 发布完整绑定 | 模式强制存在 | 当前完整注册表只有 3 个句柄类型 |
| 热重载 | 有配置枚举 | 没有文件监听、事务替换和状态迁移 |

因此，当前代码适合作为概念验证和生命周期原型，不适合作为后续大规模引擎 API 接入的最终架构。

## 2. 当前实际运行链路

```text
executor
  -> 注册 LuaScript loader 和 LuaComponent constructor
  -> 加载 game 插件
game::register
  -> 再次尝试注册 Lua 资源类型
game::init
  -> 异步加载 scene.rgs
game::initialize_scene
  -> 读取 fyrox-lua.toml
  -> 创建 UI 和 Bridge
  -> LuaRuntime::new
       -> 创建 mlua::Lua
       -> 注册 Button/Node2D/Node3D 句柄表
       -> 调用 LuaGameApi::register 或 register_reflection
       -> 按 LuaComponent 引用加载 data/scripts/*.lua
       -> 对每个脚本执行 new 和 on_awake
  -> 扫描场景中的 LuaComponent
       -> 执行组件脚本的 new 和 on_awake
  -> 对全部实例执行 start
game::update
  -> 转发 EventManager 事件
  -> 对全部实例执行 update(dt)
game::on_deinit
  -> 对全部实例执行 on_destroy
```

该流程可以运行，但目录脚本和组件脚本属于两套发现机制，最终混在一个 `Vec<(PathBuf, RegistryKey)>` 中，缺少来源、所属节点、状态、错误和重载版本信息。

## 3. 主要设计问题

### 3.1 Reflect 能力被高估

Fyrox `Reflect` 主要提供字段遍历、类型信息、向下转型和字段读写。它不等价于 C# Reflection，也不会自动提供任意 Rust 方法调用。当前 `LuaReflection::call(object, method, Vec<String>)` 只是由宿主自行解释的字符串协议；`get_field`、`set_field` 默认返回未实现，而且没有真实 backend。

正确边界应为：

- 字段和属性：通过 Fyrox `Reflect` 动态访问；
- 方法：通过手写或生成的 `MethodDescriptor` 注册；
- 构造器：通过 Fyrox constructor container 或显式 `ConstructorDescriptor`；
- 枚举和纯值类型：通过类型转换器注册；
- 不允许把 Rust 任意函数指针或借用直接暴露给 Lua。

### 3.2 句柄不等于对象绑定

当前 Button、Node2D、Node3D wrapper 仅保存 `index + generation`，Lua 只能调用 `index`、`generation`、`is_none`。它没有 `SceneGraph` 或 `UserInterface` 上下文，不能验证句柄是否仍有效，也不能访问名称、Transform、可见性、文本或按钮状态。

句柄应由对象仓库解析，而不是直接伪造。`from_handle(index, generation)` 不应作为普通 Lua 公共 API，否则脚本可以构造任意无效或跨场景句柄。

### 3.3 对象身份和生命周期模型缺失

tolua 使用 `ObjectTranslator` 维护 CLR 对象和 userdata 的稳定映射。Rust/Fyrox 不应复制该实现，但需要等价的安全层。目前没有：

- 对象所属域（Scene、UI、Resource、Game service）；
- scene/ui 实例标识；
- 句柄有效性校验；
- 跨场景错误检查；
- userdata 缓存与相等性规则；
- 对象销毁后的确定错误；
- Rust 借用只在单次调用内存在的约束。

### 3.4 生命周期不是严格状态机

`new` 和 `on_awake` 在加载时立即执行，`start` 在场景扫描后统一执行。这个基本顺序合理，但当前没有防止重复 `start`；新热加载实例也没有明确何时进入 `start`。任意一个脚本报错会中断其他脚本和整个 game update。`on_destroy` 失败也会中断后续销毁。

此外，当前把 `update(dt)` 作为通用 `call_all(name, dt)`，没有 `Awakened / Started / Faulted / Destroyed` 状态，无法保证每个实例的生命周期不变量。

### 3.5 目录脚本与组件脚本语义混淆

扫描 `data/scripts` 下所有 `.lua` 并把每个文件都当作组件 class 实例化，会错误加载模块、工具库、配置文件和 require 依赖。场景中的同一脚本又可能作为 `LuaComponent` 再实例化一次。

应区分：

- `data/scripts/modules`：仅由 `require` 加载，不实例化；
- `data/scripts/components`：由 `LuaComponent` 引用后实例化；
- `data/scripts/bootstrap`：每个 VM 只执行一次；
- `data/scripts/tests`：仅测试环境加载。

运行时不应递归实例化所有 Lua 文件。

### 3.6 配置和路径不稳定

`fyrox-lua.toml`、`data/scripts` 和 `data/SimHei.ttf` 当前以进程工作目录解析。Editor、Project Manager、独立 executor 和发布包的工作目录可能不同。正确做法是由宿主提供规范化的 `ProjectPaths`，所有相对路径都以项目根或资源根解析，并在日志中输出规范化路径。

### 3.7 模式与 feature 容易错配

运行模式同时由 TOML 的 `binding_mode`、`lua-binding` 的 `editor/package` feature、game 的 `lua-editor/lua-package` feature控制。三个来源可能组合出不可理解的状态。发布构建虽然会强制 `PackageFull`，但“完整”目前只表示调用 `LuaGameApi::register`，并不验证 API manifest 是否完整。

### 3.8 参数系统类型过弱

Inspector 参数统一保存为字符串，Lua 再自行 `tonumber`，不能表达 Vector、Color、Handle、Resource、枚举、数组和可空值，也不能在 Inspector 做类型校验。后续应使用可序列化的 `LuaPropertyValue` 枚举，并保留稳定字段 ID。

### 3.9 热重载尚未实现

`ReloadMode` 目前只参与配置和日志。完整热重载至少需要 watcher、变更去抖、编译/语法预检、旧实例状态导出、新实例构造、状态迁移、成功后原子替换、失败回滚以及 `on_destroy` 顺序。

## 4. 目标架构

建议采用“Reflect 字段桥 + 显式方法绑定 + 安全对象仓库 + 双模式注册”的混合架构。它保留 tolua 的可审计注册入口，但适应 Rust 的所有权和 Fyrox generational handle。

```text
Host (executor/editor/game)
  ProjectPaths + RuntimeServices + EngineAccess
                  |
             LuaRuntimeHost
      +-----------+------------+
      |           |            |
 ModuleLoader  InstanceStore  EventBus
      |           |            |
      +------ LuaContext -------+
                  |
       BindingRegistry / Manifest
        |                     |
 EditorReflectBackend    PackageBindingBackend
        |                     |
 Reflect fields          Generated/manual methods
 Method descriptors      Static converters
        +----------+----------+
                   |
              ObjectStore
   SceneNodeRef / UiNodeRef / ResourceRef / ServiceRef
```

### 4.1 crate/module 边界

建议后续拆分为以下模块，初期可仍放在 `lua-binding` crate 内：

- `runtime/host.rs`：VM 创建、启动、停止，不依赖具体 game；
- `runtime/instance.rs`：`ScriptInstance` 状态机和生命周期；
- `runtime/reload.rs`：watcher、事务热重载和回滚；
- `loader/module_loader.rs`：`require` 搜索路径、UTF-8、缓存；
- `component/schema.rs`：脚本参数 schema 与 Inspector 数据；
- `object/store.rs`：Lua userdata 到 Fyrox 对象引用的安全解析；
- `object/id.rs`：带域和所有者的稳定 `ObjectId`；
- `reflect/backend.rs`：Fyrox `Reflect` 字段读写；
- `binding/descriptor.rs`：类型、方法、构造器和转换器描述；
- `binding/editor.rs`：编辑器动态查找与缓存；
- `binding/package.rs`：发布静态注册；
- `binding/categories/*`：core、scene、ui、resource、physics 等分类；
- `conversion/*`：Lua 与 Rust 基础值、数学值、资源和对象引用转换；
- `diagnostics.rs`：结构化日志和脚本错误。

`game` 只注册游戏领域服务和事件，不直接实现通用 Fyrox 反射；`executor/editor` 提供场景、UI、资源管理器和项目路径上下文。

### 4.2 安全对象引用

Lua userdata 应保存逻辑对象 ID，而不是 Rust 引用：

```rust
struct LuaObjectRef {
    domain: ObjectDomain,
    owner: OwnerId,
    handle: ErasedHandle,
    expected_type: TypeId,
}
```

每次 Lua 调用时，`ObjectStore` 根据 domain 找到 SceneGraph、UserInterface 或 ResourceManager，校验 owner、generation 和类型，临时取得借用，执行一次调用后立即释放。不得跨 Lua yield、回调或帧保存 `&mut T`。

对象来源应是可信查询，例如 `scene:find_node("Player")`、事件参数或 Rust 返回值。仅调试 API 可以从原始 index/generation 构造，并默认关闭。

### 4.3 Reflect 字段桥

字段访问流程：

```text
Lua object.name
  -> __index
  -> TypeDescriptor 查找字段别名
  -> ObjectStore 临时解析对象
  -> Reflect::field / field_mut
  -> ValueConverter 转为 Lua 值
  -> 返回，释放 Rust 借用
```

字段写入需要白名单、只读标记、类型转换和范围校验。建议缓存“类型 + Lua 字段名 -> Reflect 字段路径”，但不能缓存对象借用。

Fyrox 的继承变量、资源引用、Option、集合和数学类型需要专门 converter；未知类型应返回包含类型名和字段路径的错误，不能静默字符串化。

### 4.4 方法描述器

Rust 方法不能单靠 Fyrox Reflect 枚举，因此采用 tolua 风格显式注册：

```rust
MethodDescriptor {
    lua_name: "set_visible",
    rust_type: TypeId::of::<Widget>(),
    mutability: Mutable,
    availability: Both,
    invoke: invoke_widget_set_visible,
}
```

可用宏减少样板代码，但宏输入必须是明确白名单，不能分析 Lua 源码。编辑器模式和发布模式共享同一 descriptor/manifest，区别只在调用后端，而不是维护两套 API 名称。

### 4.5 两种运行模式

编辑器模式：

1. 加载共享 API manifest；
2. 已有静态 wrapper 优先走快速路径；
3. 未生成 wrapper 的字段通过 Reflect backend；
4. 未生成 wrapper 的方法只能通过已登记 `MethodDescriptor` 调用；
5. 首次解析后缓存 descriptor；
6. 输出动态回退计数，帮助决定哪些高频 API 应进入静态绑定。

发布模式：

1. 只编译 `PackageBindingBackend`；
2. 禁止 Reflect 动态回退和原始句柄构造；
3. 启动时校验 manifest hash、绑定版本和必需类型；
4. 任意缺失绑定直接启动失败，不允许名为 PackageFull 但只有部分 API；
5. 高频调用使用 typed userdata 和直接 Rust closure。

配置中的模式仅能在编译产物允许的集合内选择。建议最终使用一个 `RuntimeFlavor` 枚举作为唯一真相，Cargo feature 只决定哪些 flavor 被编译。

### 4.6 ScriptInstance 状态机

```text
Loaded -> Constructed -> Awakened -> Started -> Destroyed
                         |             |
                         +-> Faulted <-+
```

规则：

- `on_awake` 必需，其他生命周期可选；
- `start` 每个实例最多一次；
- 只有 Started 实例执行 update/on_event；
- 单脚本错误将该实例标记 Faulted，并记录错误，不默认终止其他实例；
- 可配置 critical 脚本在错误时终止场景；
- destroy 使用 best-effort，所有实例都尝试销毁并汇总错误；
- 实例 ID 包含 scene、node、component slot 和 generation，不能仅用拼接路径。

### 4.7 脚本加载模型

不再自动实例化 `script_root` 下所有 `.lua` 文件。建议配置：

```toml
script_root = "data/scripts"
bootstrap = ["bootstrap/main.lua"]
module_paths = ["modules/?.lua", "modules/?/init.lua"]
component_root = "components"
```

`LuaComponent` 使用相对 `component_root` 的 asset path。bootstrap 每 VM 执行一次；modules 由 `require` 缓存；components 每个节点独立实例化。资源管理器与文件系统必须共享同一个规范路径模型。

### 4.8 热重载事务

Immediate 模式的推荐顺序：

1. watcher 收到路径变化并去抖；
2. 新建临时 Lua environment，解析并验证 class contract；
3. 对旧实例调用可选 `on_serialize`；
4. 在隔离环境构造新实例并调用 `on_awake`；
5. 调用可选 `on_deserialize(state)`；
6. 若场景已启动，则调用新实例 `start` 或 `on_reload`；
7. 全部成功后原子替换 registry key；
8. 最后调用旧实例 `on_destroy`；
9. 失败则保留旧实例并记录完整错误。

OnStop 模式只记录 dirty 文件，在停止运行和下次启动之间重载。

## 5. API 分层建议

不要尝试一次性暴露 Fyrox 的全部公开接口。建议按稳定性和调用频率分层：

- Tier 0：基础值、Vector/Quaternion/Color、日志、时间和事件；
- Tier 1：Scene/Node/Transform、UI Widget/Text/Button/TextBox；
- Tier 2：资源、音频、动画、物理查询；
- Tier 3：渲染、材质、粒子等高级 API；
- Game API：背包、任务、技能等项目领域服务，与引擎 API 分开命名。

每个分类必须有模块注册函数、manifest 条目、正向调用测试、无效句柄测试、类型转换失败测试和发布模式可用性测试。

## 6. 建议的 Lua API 外形

```lua
local PlayerController = class("PlayerController")

function PlayerController:new(params)
    self.speed = params.speed
end

function PlayerController:on_awake(ctx)
    self.node = ctx.node
    self.transform = self.node.transform
end

function PlayerController:start()
end

function PlayerController:update(dt)
    local p = self.transform.position
    self.transform.position = p + Vector3.new(self.speed * dt, 0, 0)
end

function PlayerController:on_event(event)
end

function PlayerController:on_destroy()
end

return PlayerController
```

`ctx` 由 runtime 注入，至少包含 `node`、`scene`、`events`、`resources` 和只读的 runtime 信息。脚本无需通过字符串查找自己的宿主节点。

## 7. 迁移计划

### 阶段 A：建立正确契约

- 固化 `RuntimeFlavor`、API manifest 和脚本目录语义；
- 引入 ScriptInstance 状态机和结构化错误；
- 禁止生产 Lua 使用 `from_handle`；
- 添加 editor/package 两组构建矩阵测试。

验收标准：生命周期不重复；单脚本错误可隔离；模式组合不会静默降级。

### 阶段 B：对象仓库与 Reflect 字段

- 实现 Scene、UI、Resource 三种 domain；
- 实现对象有效性和类型校验；
- 支持基础值、Vector、Color、Option 和资源引用转换；
- 接入真实 `Reflect::field/field_mut`。

验收标准：Lua 可以读取/修改真实 Node 和 Widget 的白名单字段；销毁对象后调用得到确定错误。

### 阶段 C：tolua 风格方法绑定

- 定义 MethodDescriptor 和注册宏；
- 完成 Transform、Node、Widget、Text、Button、TextBox 的基础方法；
- 生成 API manifest 和 Lua stub，用于补全和静态检查；
- 对高频 API 添加直接 closure 快速路径。

验收标准：编辑器动态路径与发布静态路径具有相同 Lua API 和行为测试。

### 阶段 D：组件与热重载

- 分离 bootstrap/module/component；
- Inspector 参数改为强类型 schema；
- 实现 watcher 和事务热重载；
- 提供状态迁移钩子。

验收标准：修改组件脚本后无需重编 Rust；语法错误不破坏旧实例；恢复正确脚本后自动替换。

### 阶段 E：发布完整性

- 发布启动时验证 manifest hash；
- 移除 editor reflection backend；
- 对资源路径和脚本依赖生成打包清单；
- 基准测试 update、字段访问、方法调用和事件吞吐。

验收标准：发布包不包含动态反射入口；所有声明为可用的 API 均有实际 wrapper 和测试。

## 8. 测试与性能评估

必须新增以下测试层级：

- 单元测试：转换器、descriptor 查找、状态机、路径解析；
- 集成测试：真实 SceneGraph/UserInterface 对象读写；
- 双后端契约测试：同一 Lua 用例分别跑 editor/package；
- 热重载测试：成功替换、语法错误回滚、状态迁移；
- 安全测试：过期句柄、跨场景句柄、错误类型、递归事件；
- 构建测试：`--no-default-features`、editor、package、dylib 组合；
- 性能测试：缓存命中/未命中、Reflect 字段、静态方法、每帧 1K/10K 调用。

性能原则是一切跨语言调用都应可测量。Reflect 适合低频编辑器调用；每帧高频逻辑应升级为静态 wrapper 或批量 API。不要为了“完全自动”而绕过 Rust 类型和借用安全，也不要为低频 API提前生成海量样板代码。

## 9. 最终建议

保留当前 `LuaRuntime`、`LuaComponent`、EventManager 和手写注册入口的方向，但重新定义其职责。最重要的重构不是继续扩充 catalog，而是先建立对象仓库、真实 Reflect 字段桥、显式方法描述器、实例状态机和统一 manifest。

tolua 可借鉴的是“统一注册入口、静态 wrapper、对象 translator、编辑器反射补位和发布完整绑定”的分层思想；不能直接照搬 C# 的任意方法反射。Rust 版本应让 Reflect 负责数据，让 descriptor 负责行为，让 ObjectStore 负责所有权与句柄安全。
