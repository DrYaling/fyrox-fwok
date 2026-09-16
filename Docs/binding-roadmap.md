# Lua 绑定插件优化技术方案

日期：2026-09-16。本文是后续方案，不代表已实现。

大范围结构体、函数和字段覆盖的专项审计见 [fyrox-api-binding-audit.md](fyrox-api-binding-audit.md)，具体实施、覆盖口径和成本见 [fyrox-binding-next-plan.md](fyrox-binding-next-plan.md)。本文保留通用架构原则，专项计划细化大规模扩展。

## 目标架构

lua-tool 负责扫描 → 规范化 IR → 策略校验 → 生成；lua-plugin 只负责值转换、可执行注册、生命周期和引擎适配。
默认保留静态受控绑定。反射仅作为明确标记的编辑器诊断/低频字段通道，不能用反射直接写字段绕过引擎 setter/message 的业务不变量。
借鉴 ToLua 的显式导出列表和生成期校验；借鉴 Fyrox-Lua 的路径/字段工具体验；不复制 C# 对象 GC 模型或未经证明的静态可变上下文。

## P0：绑定事实单一来源

建立 versioned Binding IR：稳定 Rust 全限定路径、Lua 名称、接收者类型、参数/返回类型、可空性、所有权、线程限制、操作策略、错误策略、文档和状态。
状态区分 discovered、approved、generated、verified、unsupported；验证记录关联测试，不能靠一个 Implemented 字符串宣称支持整个类型。
生成 outputs：runtime Rust（无说明目录）、catalog.json、LuaLS 注释、API 签名快照、unsupported 原因报告。全部来自同一 IR。
扫描只是候选发现；人工 allowlist 明确批准，禁止“所有 public 默认导出”。
模块名/cfg/宏首先限制到可证明子集；复杂引擎面评估 rustdoc JSON，隔离 nightly 版本并缓存输出，不把 nightly 带入运行时构建。
生成按全限定路径排序，重复 Lua 名称立即失败，输出先写临时文件再替换。--check 重新生成到临时目录并比较，CI 防止手改 generated.rs。
验收：同一输入字节一致；破坏签名能被 API diff 检出；unsupported 不产生伪 wrapper；纯扫描模式无需构建 Fyrox。

## P1：对象身份与执行契约

将 NodeRef 定义为 {world_epoch, scene_id, handle(index,generation), capability}。find 在明确解析阶段返回 resolved ref 或结构化 MissingTarget；不要让旧引用重新匹配同名节点。
如需异步查找，类型命名为 PendingLookup，不能与 NodeRef 混用。
Scene/UI 更换递增 epoch，队列命令携带 epoch，消费拒绝过期命令。名称查找缓存对 rename/reparent/remove 失效，或限制名称仅用于首次解析。
setter 使用 command 策略；query 明确使用 frame snapshot 或短期 scoped borrow。callback 不能把 &mut Graph/ScriptContext 留在 userdata，更不能用 transmute 延长至 static。
错误含 ScriptInstanceId、目标、方法、参数摘要、阶段；每帧聚合限流但保留计数。
验收：删除重建同名对象、generation 变化、跨场景相同 index、重挂父节点、错误组件类型均有独立回归测试。

## P1：生命周期和事件

用 ScriptInstanceId = scene epoch + node generation + component slot 标识实例，文件路径单独保存。
重载分 prepare/commit：先解析构造新实例，成功才替换旧实例；提交前失败保留旧实例。on_awake 有副作用，准备期必须缓冲命令，失败丢弃，不能仅交换 Table 就声称事务性。
state migration 只通过显式 serialize/restore hook；不复制任意闭包。模块依赖图决定 package.loaded 失效范围。
事件返回 SubscriptionId，支持多个监听，绑定实例所有权，destroy/reload 取消订阅。重入使用 dispatch depth 或延迟变更队列。
错误策略默认隔离实例，允许 fail-fast 开发选项；清理过程中单个 on_destroy 出错也应继续释放其他 registry key。
验收：语法错误回滚、构造错误、副作用回滚、重复 start、重入、移除节点、回调泄漏、模块依赖更新。

## P2：降低代码扩展成本

按领域拆分 game_api.rs 为 value、scene、ui、bridge；先建立行为测试再移动代码，不制造循环依赖。
命令枚举、参数转换和方法表可从 approved IR 生成；宿主语义操作仍由适配器实现。
UiComponentRef 应校验 capability 或提供窄 userdata；ui.button 不能仅换个名字继续接受任意节点。
lua-tool 的 Fyrox 依赖应改为可选 feature，prepare_ui_demo binary 设置 required-features；纯生成器只依赖 syn/serde/quote。结合现有脚本一起迁移，避免默认命令失效。
配置删除虚假模式或明确命名为预留策略；feature 决定后端，启动时校验与 jit 配置一致。Lua54 与 LuaJIT 分别测试，不能用全 feature 联合构建代替互斥后端验证。

## 性能评估与发布门槛

统一测试场景、构建模式、硬件与 VM：测 VM 初始化时间、注册分配量、每帧命令量、查询时延、p50/p95、Lua/Rust 分配量。
分别测直接 wrapper、队列+消费、反射字段路径。预热后测试 1k/10k 操作，记录失败/缺失目标，不仅测成功路径。
不同引擎/VM 的 ToLua 与 Fyrox-Lua 只能做机制对照，不能直接比绝对 FPS。
发布前要求：workspace check、两种后端测试（可用平台）、生成一致性、生命周期回归、executor 真实 UI 绘制和点击、API 兼容差异说明。
路线顺序：事实单一来源 → 身份/生命周期正确性 → 扩大能力覆盖 → 基准优化。没有正确性证据前，不优先做宏化“全自动导出”。
