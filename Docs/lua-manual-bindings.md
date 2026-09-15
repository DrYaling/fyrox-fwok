# Lua 手写引擎绑定

## 公开 API 目录

`bindings/catalog.rs` 按 Core、Engine、Scene、Scene2D、Scene3D、Ui、Input、Resource、Physics、Rendering、Material、Animation、Audio、Scripting 分类登记 Fyrox 公开类型。目录项标记为 `CatalogOnly` 时只用于审计和文档，不会自动创建 Lua 全局表；只有手写 Rust wrapper 标记为 `Implemented` 后才能从 Lua 调用。

Lua API 不从脚本源码推断。脚本扫描和自动绑定清单已从运行时移除；新增引擎 API 必须由 Rust 开发者在对应绑定模块中显式注册并重新编译。

唯一基础入口是 `lua-binding/src/bindings/mod.rs::register_engine_bindings`，当前按分类注册：

- `bindings/button.rs`：Fyrox UI Button。
- `bindings/node2d.rs`：二维语义 Node。
- `bindings/node3d.rs`：三维 Node。

当前基础类型提供类型化句柄代理和 `index`、`generation`、`is_none` 方法：

```lua
local button = fyrox.ui.Button.from_handle(index, generation)
local node2d = fyrox.scene.Node2D.from_handle(index, generation)
local node3d = fyrox.scene.Node3D.from_handle(index, generation)
```

代理不持有 Rust 借用。UI 通过通用注册器执行惰性 `Find/GetComponent`：Lua 首次按名称查找，成功后缓存句柄；禁止为 inventory、chat、skill 等业务对象增加 Rust 专用 Host、Interface 或 Bindings 模块。

`LuaRuntime::new` 创建 VM 后只调用一次基础注册入口，再调用游戏库的 `LuaGameApi::register`。运行时不读取 Lua 源码、不生成 manifest、不按脚本调用裁剪绑定。

完整绑定表示批准的手写导出契约都有编译期 wrapper，不表示暴露 Fyrox 全部内部 API。发布版使用 Rust feature 或显式配置裁剪，不能使用脚本扫描结果作为安全边界。

旧 `lua-binding-tool`、`scripts/generate-lua-bindings.ps1` 和 `BindingManifest` 自动扫描路径已移除。
