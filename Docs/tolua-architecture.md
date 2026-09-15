# tolua 绑定架构与 FWOK 对照

本文整理 `F:/WorkSpaceNew/Fyrox/tolua-master` 的目录、初始化顺序和绑定模型，作为 FWOK Lua 接入的设计依据。

## 目录职责

- `Assets/ToLua/Core/LuaState.cs`：Lua VM 生命周期、加载 chunk、错误处理和全局环境。
- `Assets/ToLua/Core/ObjectTranslator.cs`：C# 对象与 Lua userdata 的双向映射、引用计数、GC 延迟释放。
- `Assets/ToLua/Core/`：底层栈操作、类型转换和调用辅助代码。
- `Assets/ToLua/Reflection/LuaReflection.cs`：显式反射 API（`findtype`、`getmethod`、`getconstructor`、字段/属性访问和实例化）。它不是对所有 API 的隐式扫描器。
- `Assets/Source/Generate/`：生成器输出目录；`LuaBinder.cs` 是所有静态 wrapper 的统一注册入口。
- `Assets/ToLua/Misc/LuaClient.cs`：Unity 宿主初始化、场景加载和 Lua 全局环境设置。
- `Assets/ToLua/Misc/LuaLooper.cs`：把 Unity 的 Update、LateUpdate、FixedUpdate 转发到 Lua。
- `Assets/ToLua/Misc/LuaResLoader.cs`：按 Lua 模块名解析资源路径并读取 chunk。

## 静态绑定流程

1. 在 `CustomSetting.cs` 声明允许导出的类型、成员和委托。
2. 生成器为每个类型输出 wrapper，并将类型注册函数汇总到 `LuaBinder.Bind`。
3. 宿主创建 `LuaState`，初始化基础库和 `ObjectTranslator`。
4. 宿主调用 `LuaBinder.Bind(luaState)`；wrapper 在此时把构造函数、方法、属性和枚举放入 Lua 表。
5. 执行启动脚本，脚本通过已注册的类型创建对象并调用方法。
6. `LuaLooper` 每帧调用脚本生命周期；销毁时由 translator 清理 userdata。

静态 wrapper 的优点是启动和调用开销可预测、发布包不依赖反射元数据；缺点是 API 变更后必须重新生成并编译。

## 反射流程

`LuaReflection` 只在脚本明确请求时解析类型：查找类型 -> 获取构造函数/方法/字段/属性 -> 创建实例或调用。解析结果可以缓存，但调用仍需做参数转换和安全检查。该模式适合编辑器预览、热更新和快速迭代，不应作为发布模式的全部 API。

## FWOK 的对应实现

`lua-binding` 将入口拆分为 `config`、`runtime`、`bindings`、`reflection`、`events` 和 `resource`：

- `LuaRuntime::new` 创建 VM，调用 `bindings::register_engine_bindings`，按配置选择编辑器反射或发布完整绑定，然后扫描 `script_root`。
- 发布构建强制 `PackageFull`；若配置降级为反射会输出 warning。
- 当前绑定采用手写注册入口，禁止根据 Lua 源码自动分析 API。这样注册集合可审计、可测试，且不会因动态代码误判。
- 场景中的 `LuaComponent` 只保存脚本资源和参数；实例状态集中在宿主 `LuaRuntime`，避免 DLL 热重载后组件持有失效 VM。
- 生命周期顺序为 `new -> on_awake -> start -> update(dt) / on_event(name,payload) -> on_destroy`。

## 配置与启动

`fyrox-lua.toml` 的 `script_root` 是项目 Lua 根目录（当前为 `data/scripts`），`font_path` 指定项目 UI 字体。宿主加载配置后创建 UI 和 Lua runtime；runtime 会记录根目录、发现数量、每个脚本的加载结果和生命周期错误。Lua 源码不再从其他目录回退加载。

当前项目配置为 `data/SimHei.ttf`。该字体在 UI 创建前请求并写入 `UserInterface.default_font`，所以 Text、Button、TextBox 等控件默认继承它，中文不再依赖 Fyrox 内置 Roboto。字体加载失败时资源管理器会保留错误状态，日志中的路径可用于定位资源登记或打包问题。

## 设计取舍

编辑器默认反射模式，允许未绑定的 Fyrox 接口通过显式反射调用；发布只能使用完整手写绑定。引擎核心不创建 Lua VM，也不注册 Lua 资源，所有初始化发生在 executor/game/lua-binding 宿主层，从而保持 Fyrox 核心无脚本运行时依赖。
