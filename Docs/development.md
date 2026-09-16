# 引擎、Lua 与脚本开发

审计日期：2026-09-16。本文描述当前实现；下一阶段设计见 binding-roadmap.md。

## 工程边界

| 工程 | 当前职责 |
|---|---|
| game | Fyrox 游戏插件、资源装载、逐帧推进、命令应用、UI 消息路由 |
| executor / game-dylib | 独立执行入口 / 游戏动态库 |
| lua-plugin | Lua VM、生命周期、资源组件、userdata、命令桥、节点查找缓存 |
| lua-tool | Rust 源码扫描、目录 JSON、受控模板生成、离线 UI 资源准备 |

本工程没有独立 editor member。编辑器依赖相邻 Fyrox 工程。引擎路径依赖意味着仅复制本仓库不足以构建。

## 构建与生成

```powershell
rtk cargo run -p executor
rtk cargo test -p lua-plugin -p lua-tool --lib --bins
rtk cargo run -p lua-tool --bin lua-tool -- --input lua-plugin/src --output target/lua-bindings --runtime-output lua-plugin/src/bindings/generated.rs --profile common
```

`scripts/generate-lua-bindings.ps1` 包装生成和格式化过程。工具有多个 binary，必须显式指定 `--bin lua-tool`。
输出包含 catalog.json、curated-catalog.json、unsupported.json、generated.rs、api-diff.md。
curated-catalog 是迁移后的历史人工说明，不是 VM 能力清单；catalog 是语法扫描和 profile 信息，不代表全部可调用。
api-diff.md 目前只是数量汇总，不是真正的版本差异。
generated.rs 只包含可执行构造器、方法注册及别名，不再包含描述注册表。

## 注册与执行顺序

`LuaRuntime::new_inner` 创建 VM，配置 package.path，注册值类型，再调用 LuaGameApi::register，最后注册组件别名。
Api 建立 ui / scene 命名空间与共享 Bridge。Lua 方法写入命令队列，宿主消费队列并更新引擎。
UI 使用 Fyrox 消息机制，调用 setter 成功只意味着排队成功；不意味着目标已找到或引擎状态已改变。
读取文本等值依赖宿主同步，不能假定 set 后立即 read 得到新值。

## 脚本与资源

业务 Lua 放在 `data/scripts`，公共模块放在 `data/scripts/modules`。场景 LuaComponent 引用外部 LuaScript 资源。
生产流程不使用 source_override 或动态创建业务节点；该字段当前仍存在于序列化兼容实现和测试中。
UI 资源是 `data/unnamed.ui`。HUD 名称包括 hud_title、task、time、chat_log、chat_input、send_button、inventory_button、skill_button。
查找不证明对象存在或可见；应检查资源内容和运行日志。

脚本返回 class table，必须定义 `new(class, params)` 并返回 instance table。可选生命周期为 on_awake、start、update(dt)、on_event(name,payload)、on_destroy。
exported 参数当前以字符串传入，脚本自行 tonumber；params.components 包含配置的类型和 index/generation 信息。
`new_for_scene` 不扫描全局脚本目录，随后扫描场景 LuaComponent。`new` 会扫描脚本目录，不能与场景实例化混用而不考虑重复实例。
模块使用 `require('modules.ui_shared')` 等，package.loaded 缓存不会因 reload_script 自动失效。

## 查找和事件语义

scene.find 在脚本所属节点子树查找，scene.global_find 明确使用全场景范围。作用域由生命周期调用注入，缓存键包含作用域。
ui.find 使用 UI 名称代理；ui.text、ui.button 等为别名，不是独立类型检查器。
真实 ButtonMessage::Click 经 game 路由至 Lua 回调。每个 UI id 的回调表并不是完整的多订阅事件系统。
场景和 UI 缓存必须在资源更换时清理；index/generation 只在同一个对象池内标识对象。

## 配置的真实边界

fyrox-lua.toml 包含 script_root、font_path、jit、reload_mode、enabled、binding_mode。
Lua54 / LuaJIT 由 Cargo feature 决定，jit 布尔值不是运行时切换 VM 的开关。
EditorReflection / PackageFull 当前不能理解为两套已实现的反射/静态绑定体系；effective_binding_mode 有 feature 映射，但注册执行路径共用。
reload_script 是文件级重建，尚无事务回滚、模块依赖图或场景实例批量迁移保证。

## 验证

单元测试覆盖值类型、Lua 生命周期、require、命令排队、句柄、查找等；不能替代真实图形运行。
UI 验收还需 executor 的 draw_commands > 0、资源解析成功、真实 ButtonMessage::Click 进入 Lua。没有这些证据时不得声称完成 UI 端到端验收。
