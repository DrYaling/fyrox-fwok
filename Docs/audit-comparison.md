# 当前实现审计与三方对比

日期：2026-09-16。以本地源码为证据，不引用宣传性能数字。
请求的 ../tolua 实际不存在，采用同级 ../tolua-master（Unity/C# ToLua，不是 C/C++ tolua++）。另一份为 ../Fyrox-Lua。
本次对参考工程做静态源码审计，未构建 Unity 工程或运行 Fyrox-Lua；因此不对它们在当前平台的可运行性和速度做结论。
工作区原有未提交修改，本文针对工作树，不是某个干净发行版本。

## 结论

FWOK 目前是专用命令桥和受控模板绑定，不能称为完整的自动 Rust/Fyrox API 绑定器。
其优势是生命周期、资源与命令边界清晰，主要路径不用跨 Lua 保存引擎可变引用；短板是能力覆盖有限、元数据容易漂移、身份和生命周期契约尚不完整。
ToLua 的自动导出体系明显更完整，但 C# 反射、重载、对象 GC、原生库和平台适配使整体复杂度高；不能将其生成方式直接照搬 Rust。
Fyrox-Lua 的反射路径方案在广泛字段访问和编辑器联动方面更直接，但当前快照存在未完成实现与数值转换风险，不能把概念优势等同于产品完成度。

## 原理与工程取舍

| 维度 | FWOK | ToLua | Fyrox-Lua |
|---|---|---|---|
| 语言/VM | Rust + mlua 0.12.1；默认 Lua54 | C# + Lua C API / 原生插件，仓库含 LuaJIT 工具 | Rust + mlua 0.9.1 luau |
| 导出机制 | syn 扫描目录；common profile 与固定 wrapper 模板 | 编辑期 C# Type/MethodInfo 反射生成 wrapper 与 Binder | ReflectUserData 的 Index/NewIndex 解析字段路径，辅以手写 userdata 和试验 bindgen |
| 对象访问 | 名称/作用域代理，命令队列，宿主缓存 | ObjectTranslator 对象池与反向映射 | 节点句柄 + Reflect 字段路径 + ScriptContext |
| 覆盖 | 少量值类型、通用 UI、场景变换；类型别名不等于专用 API | 字段、属性、方法、重载、委托等有大量生成逻辑 | Reflect 字段面较宽；任意 public 方法不会自动变为反射方法 |
| 复杂度 | 核心较小，但增加命令通常跨多个层 | 生成器、运行时、GC/委托与平台层均复杂 | 表面 API 简洁，复杂度转移至路径、类型转换和上下文有效期 |
| 扩展成本 | 每能力维护 profile、userdata、命令、宿主消费及测试 | 新类型配置成本低，特殊签名/生命周期仍需适配 | 新 Reflect 字段访问成本低，新类型转换和方法仍需适配 |
| 错误时机 | 参数检查在 Lua 调用时；目标错误可能推迟到消费时 | 生成期与调用期类型检查；异常跨边界处理 | 字段拼写、路径、类型通常运行时失败 |
| 热路径成本 | userdata、字符串/HashMap、队列及宿主查找 | 生成 wrapper 的栈转换、对象映射、重载分派 | 路径复制/遍历、多次 downcast 与动态值转换 |
| 身份/生命周期 | generation 有帮助，但缺 world/scene epoch | 对象池、反向映射、延迟 GC；复杂且 Unity 特有 | 句柄和上下文约束；当前代码未展示完整生命周期闭环 |
| 可移植性 | 本地 Fyrox 版本耦合 | Unity/C# 与平台原生库耦合 | 老版 Fyrox API、Luau 和实验生成工具耦合 |

三者没有同环境基准，不能给出“FWOK 更快”或“反射必然不可用”的排名。
ToLua 的 wrapper 避免每次通过 C# 反射调用方法，但并非没有转换开销；FWOK 队列适合写操作，却不天然适合要求同步返回值的方法。

## FWOK 发现（按优先级）

| 优先级 | 证据 | 缺陷及影响 |
|---|---|---|
| P1 | lua-plugin/src/runtime.rs::reload_script | 先移除/销毁旧实例再加载新文件，失败无法回滚；场景实例 id 也不是脚本文件路径 |
| P1 | lua-plugin/src/game_api.rs::Api::register、SceneNodeRef；scene.rs::resolve_scoped | 代理排队保存名称，目标消费时再查找；删除后同名对象可能成为新目标，不能宣传为稳定对象引用 |
| P1 | handles.rs::HandleToken | 只有 index/generation，无 scene/world epoch；换对象池后存在身份碰撞可能 |
| P1 | scene.rs::resolve_scoped | 缓存只检查 handle 仍存在，不检查重命名或移出原子树；作用域语义可能过期 |
| P1 | runtime.rs::call_all / lifecycle 分发 | 错误用 ? 向外返回，单个脚本故障可能中断后续脚本；需定义隔离策略 |
| P2 | config.rs / runtime.rs::new_inner | EditorReflection 名称与实际执行策略不一致；jit 配置不能切换编译后的 Lua 引擎 |
| P2 | lua-tool/src/main.rs::parse_file | 语法提取不等于 Rust 名称解析；模块路径、cfg、宏、泛型、trait impl 等不能由简单 syn 扫描可靠解决 |
| P2 | main.rs::common_components / generated 别名 | ui.button 等返回通用 ui.find 代理，不验证 Button 类型；Implemented 不能表示该引擎类型所有方法可用 |
| P2 | runtime.rs::with_scope、game_api.rs::on_click | 作用域和回调表在可写 globals 中；回调所有权、取消订阅和重载清理缺少统一契约 |
| P2 | runtime.rs::method_key | get::<Function>().ok() 混淆缺失方法与非函数错误，隐藏脚本错误 |
| P2 | resource.rs / runtime.rs | 外部资源与 source_override 两种来源，易出现编辑内容与实际运行内容不同 |
| P2 | lua-tool api-diff.md 输出 | 只有计数，不能发现签名或行为破坏性变化 |

## 本次已完成的职责调整

原 bindings/catalog.rs、manual.rs、registry.rs 迁至 lua-tool/src/metadata，序列化到 curated-catalog.json。
插件取消 BindingRegistry 字段、bindings() 访问器及元数据类型 re-export；register_engine_bindings 返回 Result<()> 并实际注册值类型。
生成器删除描述 Rust 代码的输出，保留 JSON 目录和可执行 wrapper。插件不再因创建 VM 分配描述 BTreeMap/Vec。
这是 Rust API 破坏性调整；外部调用方如使用 bindings()，应改为读取工具输出。Lua 业务调用 API 保留。
人工目录保留历史信息以免丢失，但其状态仍可能过时；不得当作能力真相。下一步应由同一份绑定 IR 替代该目录。

没有迁出的内容：LuaConfig 运行时加载、脚本资源加载、组件 Reflect/Visit、句柄、UI 消息与场景操作，都参与运行时或引擎序列化。移动它们只会制造反向依赖。
archive 中未参与编译的旧实现尚未移走：它们应在独立清理提交中审查删除，不能与可执行代码混淆。

## 参考源码证据

ToLua：Assets/ToLua/Editor/ToLuaExport.cs::Generate 调用 GetMethods/GetProperties/GetFields；生成 CheckTypes/参数数目分派；Assets/Editor/Custom/CustomSettings.cs 定义导出配置；Assets/Source/Generate/LuaBinder.cs 与 DelegateFactory.cs 为生成注册/委托产物。
Assets/ToLua/Core/ObjectTranslator.cs 使用 objectsBackMap、LuaObjectPool、DelayGC；非 MULTI_STATE 下还有静态 translator。优点是显式对象身份管理，代价是复杂 GC 和多 VM 条件分支。

Fyrox-Lua：game/src/lua_reflect_bindings.rs::populate_reflect_lua_bindings 建立 Index/NewIndex；路径读取复制 Vec 并依次试 downcast。fyrox_and_lua_numbers 将整数读成 Lua Number，写入用 as 转换，宽整数精度和截断/范围语义需要修正。
game/src/script_context.rs 的 TLS 保存 Option<&'static mut ScriptContext>，此快照没有在该模块提供完整的设置/恢复生命周期；不能据类型声明断言其已经安全实现。
game/src/script.rs::ReflectUserData 的 address/create_sibling 存在 todo!；on_update 主要处理 Packed→Unpacked，不能据此声称完整 Lua update 已接通。
bindgen 下同时有 expand_parser 与 rustdoc_based 试验路线，不能把试验目录算成成熟的自动绑定交付。

## 文档清理范围

旧 Docs/*.md、README.md、lua-tool/README.md、scripts/README.md 被新版统一替换；旧项目文档不再作为当前说明入口。
AGENTS.md、RTK.md、.codex skills/rules 属于执行约束而非产品文档，保留。第三方/生成目录不在清理范围。
删除前已备份旧 Docs 和源代码至 target/audit-backup-20260916-210329；target 可被 clean 清除，长期保存应另行归档。
