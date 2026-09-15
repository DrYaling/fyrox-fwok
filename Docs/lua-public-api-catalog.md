# Fyrox Lua 公开 API 注册目录

## 目标

Lua 绑定采用 Rust 手写注册。`lua-binding/src/bindings/catalog.rs` 是 Fyrox 公开类型的分类目录，记录模块、用途和后续绑定边界；它不扫描 Lua 源码，也不根据脚本猜测接口。

目录状态分为两类：

- `Implemented`：已存在 Rust wrapper，并且在 `register_engine_bindings` 中创建了 Lua API。当前为 `Button`、`Node2D`、`Node3D`。
- `CatalogOnly`：已登记公开类型，但尚未提供 wrapper。此类不会自动创建 Lua 全局表，避免出现“看起来能调用、运行时才失败”的接口。

## 分类

| 分类 | Fyrox 模块范围 | 目录示例 | 访问边界 |
| --- | --- | --- | --- |
| `Core` | `fyrox::core` | `Vector2`、`Vector3`、`Color`、`Handle` | 无引擎借用的值类型优先实现为 userdata |
| `Engine` | `engine`、`plugin` | `Engine`、`Plugin` | 需要宿主生命周期上下文 |
| `Scene` | `scene`、`scene::graph` | `Scene`、`Graph`、`Node`、`Transform` | 只能保存 generational handle，访问时解析场景 |
| `Scene2D` | `scene::sprite`、`rectangle` | `Sprite`、`Rectangle`、`Node2D` | 通过场景上下文读写 |
| `Scene3D` | `scene::camera`、`mesh` | `Camera`、`Mesh`、`Node3D` | 通过场景上下文读写 |
| `Ui` | `gui::*` | `Button`、`Widget`、`Text`、`TextBox` | 通过 `UserInterface` 消息队列访问 |
| `Input` | `gui::input`、输入服务 | `Input` | 每帧从宿主采样，不保存 Rust 引用 |
| `Resource` | `resource` | `Resource`、`ResourceManager` | 返回资源句柄，异步加载由宿主驱动 |
| `Rendering` | `renderer`、纹理 | `Texture` | 仅暴露稳定资源参数 |
| `Material` | `material` | `Material` | 参数写入需渲染上下文 |
| `Physics` | 场景物理组件 | `RigidBody`、`Collider` | 通过物理世界上下文执行 |
| `Animation` | 场景动画 | `AnimationPlayer` | 播放控制需要场景/动画上下文 |
| `Audio` | 场景声音 | `AudioSource` | 由音频服务管理句柄和播放状态 |
| `Scripting` | `script` | `Script` | 仅用于脚本生命周期和宿主桥接 |

## 注册规则

1. 唯一入口是 `register_engine_bindings(lua)`；每个领域在独立模块中注册，禁止把所有实现塞进 `lib.rs`。
2. 每个公开类型必须填写 `BindingType` 的模块、分类、中文说明和 `BindingStatus`。
3. 每个真实 wrapper 必须使用 typed userdata/句柄，不能把 Rust 借用保存到 Lua，也不能通过字符串反射调用任意方法。
4. 需要场景、UI、资源或物理访问的接口必须增加显式 Context 适配器，并将句柄失效转换为 Lua 错误。
5. 新增发布接口必须同时更新目录、wrapper、中文文档和测试；未完成 wrapper 的条目保持 `CatalogOnly`。

## 审计接口

`BindingRegistry::types()` 返回全部目录项；`by_category(category)` 用于编辑器按领域展示；`implemented_len()` 用于发布构建检查真实 wrapper 数量。目录项不会改变 Lua 的权限边界，权限只由手写注册函数决定。

## 当前基础 Lua 用法

```lua
local button = fyrox.ui.Button.from_handle(index, generation)
local node2d = fyrox.scene.Node2D.from_handle(index, generation)
local node3d = fyrox.scene.Node3D.from_handle(index, generation)

assert(button:index() == index)
assert(node2d:generation() == generation)
assert(not node3d:is_none())
```

这些代理只携带句柄值；真正的 Button、Node2D、Node3D 操作应在后续 Context wrapper 中完成。
