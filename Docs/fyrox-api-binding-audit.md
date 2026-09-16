# Fyrox 大范围 Lua API 绑定审计

审计日期：2026-09-16。目标是覆盖绝大部分 Fyrox 结构体、函数和可安全暴露的字段，而不是生成一个看似完整、实际无法调用的 API 表。

## 审计对象和规模

当前使用相邻 `../Fyrox` 路径依赖，仓库 HEAD 为 `a41f929403760dcf08a382cf3c8be77deafcd047`；路径依赖并没有锁定工作树内容。本次估算记录了选定源码 SHA256，以补充 HEAD 不能描述未提交修改的问题。`fyrox` crate 主要是重导出入口，实际公开面分散在 `fyrox-impl`、`fyrox-core`、`fyrox-ui`、`fyrox-resource`、`fyrox-graph`、`fyrox-animation`、`fyrox-sound`、`fyrox-material`、`fyrox-texture`、`fyrox-graphics` 和 `fyrox-autotile`。

以下数字来自 [fyrox-api-size-estimate.json](fyrox-api-size-estimate.json)，是词法估算，不是 Rust 可达性分析。它包含 cfg、测试、注释和宏造成的误报，也没有解析 trait、类型别名、重导出和泛型实例化。

| crate | Rust 文件 | pub struct | pub enum | pub fn/方法 | 公开字段行 |
|---|---:|---:|---:|---:|---:|
| fyrox-impl | 159 | 458 | 94 | 2,136 | 1,025 |
| fyrox-ui | 116 | 292 | 123 | 1,227 | 780 |
| fyrox-core | 46 | 74 | 21 | 320 | 53 |
| fyrox-resource | 14 | 31 | 6 | 234 | 30 |
| fyrox-animation | 19 | 39 | 15 | 284 | 63 |
| fyrox-sound | 19 | 27 | 10 | 200 | 13 |
| 其他选定 crate（含 facade） | 34 | 87 | 55 | 420 | 147 |

因此“绝大部分”是数千个候选声明。当前 lua-tool 的 `syn::parse_file` 只读取顶层 public struct/enum/function 和 impl 中的 public method，无法正确处理：

- `pub(crate)`、trait 方法、trait impl 和 re-export 的最终可达性；
- `cfg`、feature、宏生成项、泛型约束、关联类型和 `impl Trait`；
- tuple/unit/匿名字段、字段可见性、文档、deprecated、unsafe 和线程约束；
- 方法返回的借用、锁守卫、资源句柄、事件闭包和引擎主线程要求。

现有 `named_fields` 未过滤字段可见性，且只记录字段名称，丢失字段类型；`parse_file` 未递归处理内联 module。以上均是已确认的扫描器缺陷，不应把字段目录用于生成访问代码。

把当前扫描器扩展成“自动导出所有 public”会产生大量假绑定和生命周期错误，不能作为交付标准。

## 绑定分类

### A. 可直接生成的值类型

适合 `Copy`、无内部引擎引用、字段为已支持标量/值类型的数学、颜色、矩形、简单枚举。生成 userdata、构造器、字段读写和显式转换。字段写入必须检查范围、NaN、无穷和枚举未知值。

### B. 句柄代理类型

Scene、Node、UI 节点、资源和动画对象不能把 `&mut` 或 Rust 引用存进 Lua。使用带 world/scene/resource epoch、index、generation、capability 的 token。每次调用验证身份和线程，并将写操作转成主线程命令。

### C. 适配器类型

Engine、Graph、UserInterface、ResourceManager、Physics、Renderer、Audio 和插件上下文拥有锁、借用或生命周期约束。只生成窄适配器接口，不导出原始字段。查询读取 frame snapshot；写操作使用带 epoch 的命令。

### D. 回调和资源类型

事件、任务、资源加载器和异步结果需要显式 SubscriptionId/TaskId、取消、错误传播和 reload 清理。不允许自动把闭包、trait object、`Arc<Mutex<dyn Any>>` 或 `JoinHandle` 变成透明 userdata。

### E. 明确不自动导出

unsafe API、裸指针、`dyn Trait`、锁 guard、引用返回、泛型无法实例化、宏内部项、编辑器私有状态、渲染后端上下文、跨线程不可证明安全的类型，进入 unsupported 报告并要求人工适配。

## 字段策略

“public 字段”不等于“可以直接写”。

| 字段形态 | Lua 行为 |
|---|---|
| Copy 值、稳定序列化字段 | 值复制读写，生成范围和类型检查 |
| `InheritableVariable<T>`、`RefCell`、消息驱动字段 | 暴露 `get_*`/`set_*` 适配器，保持继承和消息语义 |
| Handle/Resource | 暴露受校验代理，不暴露池内部 index 作为唯一身份 |
| `Arc`、锁、trait object | 只提供领域操作，不暴露容器 |
| 编辑器/调试字段 | editor feature 下诊断访问，发布包默认隐藏 |

Fyrox 的 `Transform` 已显示这种差异：`position()` 返回 `InheritableVariable<Vector3>`，`set_position` 返回 `&mut Self`。Lua 应获得值快照和链式操作结果，不能持有 Rust 借用。

## 当前工程结论

现有生成器适合“候选目录 + common profile”，不具备生成绝大多数可运行绑定的条件。插件当前采用通用 UI/场景代理和命令队列，边界正确但覆盖很窄。扩大覆盖前必须建立 Binding IR、rustdoc JSON 可达性、类型分类、借用/线程分析和逐类型行为测试。

工程风险：

1. 自动生成的签名可能编译成功但语义错误，尤其是 `&mut self`、返回引用和 `Resource<T>`。
2. 字段直写会绕过 Fyrox 的消息、继承、缓存和脏标记机制。
3. 大量类型注册会增加 VM 初始化、代码体积、编译时间和 Lua 全局命名冲突。
4. Fyrox feature/backend 改变时，生成结果可能悄悄变化；必须锁定引擎 commit、feature 和生成器版本。
5. API 数量覆盖率很容易掩盖“可构造但不可更新”“可读取但不可安全写入”等行为缺口。

覆盖率必须同时报告：候选声明数、已批准类型数、已生成方法数、已验证方法数、unsupported 数、运行时行为测试数和按 crate/领域的覆盖率。
