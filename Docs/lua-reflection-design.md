# Lua 编辑器反射设计

## 目的

编辑器默认使用轻量反射，让脚本可以访问尚未生成完整 userdata 的 Fyrox 类型；发布包使用 `PackageFull` 静态绑定。两种模式共享 Lua 表面 API，切换只改变后端注册器。

## 与 tolua 的对应关系

tolua 在 CLR 中通过 `findtype`、`getmethod`、`getfield` 和 `MethodInfo.Invoke` 完成运行时调用。Rust 没有 CLR 元数据，也不能根据字符串调用任意 `pub fn`。本项目采用以下等价层：

| tolua 概念 | Rust/Fyrox 实现 |
| --- | --- |
| Type | 类型名 + `Reflect::type_info()` + 宿主类型目录 |
| MethodInfo | `ReflectionMethod` 描述符和显式调用 shim |
| FieldInfo/PropertyInfo | Fyrox `Reflect` 字段元数据及宿主 setter |
| object | 类型名 + generational handle，不保存 Rust 借用 |
| 缓存 | `reflection.get_method` 返回可重复调用的 Lua 闭包 |

## Lua API

```lua
local t = reflection.findtype("fyrox.scene.Node")
local method = reflection.getmethod("Node", "set_name")
method(node, "Player")
local value = reflection.getfield("Node", "name")
reflection.setfield("Node", "name", "Box")
```

带下划线的 `find_type`、`get_method`、`get_field`、`set_field`、`get_property` 与无下划线名称完全等价。`reflection.call(object, method, ...)` 保留为一次性调用入口。

## 后端约束

1. 类型查找和字段访问可以使用 Fyrox `Reflect`；字段写入必须尊重 `read_only`、自定义 setter 和引擎线程约束。
2. 方法必须显式登记 descriptor，包括 Lua 名称、签名、参数转换、句柄校验和调用 shim。未登记方法返回错误。
3. 场景图、UI 图和资源库由宿主上下文持有；Lua 只保存句柄代理，句柄失效时安全报错。
4. 反射只用于编辑器、Inspector 和调试。高频逻辑使用静态 userdata 或批量 API。
5. 发布构建不编译反射模块；若配置误写为 `EditorReflection`，运行时输出 warning 并强制 `PackageFull`。

## 注册生命周期

每个 `Lua` VM 启动时调用一次 `register_engine_bindings`，随后调用游戏库的反射或完整注册器。方法闭包可在 Lua 侧缓存，避免重复查找；注册不应在每帧或每个节点执行。新增 Fyrox API 必须修改对应 Rust 模块、补充 descriptor/测试和中文注释。

## 性能与安全

反射路径的成本包括方法查找、参数转换、句柄代数校验和引擎消息提交。缓存方法只能消除查找，不能消除类型检查。禁止把裸指针、`SceneGraph` 可变引用或 `UserInterface` 锁守卫存入 Lua；UI 修改应通过宿主消息适配器完成。

