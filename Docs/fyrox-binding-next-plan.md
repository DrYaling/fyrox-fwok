# Fyrox 大范围 Lua 绑定下一步技术方案

本文将“绑定大部分 Fyrox 结构体、函数和字段”拆成可审计的工程阶段。目标是高覆盖率、可回滚、可维护和可验证，不以生成文件行数作为完成标准。

## 目标和非目标

目标：覆盖常用数学/值类型、场景图、节点组件、UI、资源、动画、音频、输入和引擎服务；为每个 API 提供稳定 Lua 名称、参数转换、错误策略、线程约束、文档和行为测试。

非目标：把全部 Rust `pub` 符号无条件暴露；向 Lua 暴露 Rust 引用或锁 guard；在 Lua 中直接替代 Fyrox 编辑器和渲染后端；用反射绕过引擎消息/继承/缓存规则。

## 阶段 0：可重复的 API 事实源（P0）

1. 固定 Fyrox git commit、Cargo.lock、features、rustc 和生成器版本；生成报告记录它们的 SHA。
2. 对 rustdoc JSON 做原型验证后作为主要语义候选来源；`syn` 用于源码位置、文档和宏辅助。解析 public re-export、trait impl、cfg 和 feature。固定 nightly 和 JSON schema，用 fixture 检查升级差异；工具环境与运行时稳定 Rust 隔离。跨 crate 外部 ID 必须解析到依赖 JSON 和可导入路径，不能只扫描 facade。候选分析不能代替最终 Rust 编译。
3. 定义版本化 Binding IR：

   `rust_path`、`lua_path`、kind、crate、module、receiver、参数/返回类型、字段策略、ownership、threading、mutability、error、capability、status、docs、source_span、test_ids。

4. 状态使用 `discovered / classified / approved / generated / verified / unsupported / deprecated`，禁止单一 `Implemented`。
5. 用全限定路径消除 Lua 名称冲突；冲突和版本删除必须生成失败。

验收：相同输入字节生成结果一致；改变签名/API 可由 diff 检测；扫描阶段不需要链接游戏；unsupported 每项都有原因。

## 阶段 1：安全类型系统和转换（P0）

建立 Lua ↔ Rust 转换矩阵：nil、bool、整数、浮点、字符串、枚举、数组、Option、Result、Vec、Map、值 userdata、句柄 userdata。每个转换记录溢出、NaN、未知枚举、nil 和错误行为。

值 userdata 只保存拥有的数据副本。句柄 userdata 保存 `{world_epoch, domain_id, index, generation, capability}`；每次使用验证当前宿主。Lua 不能构造伪造的有效句柄。

禁止生成含借用返回、锁 guard、trait object、泛型未实例化和裸指针的 wrapper。为这些类型生成结构化 unsupported 报告，人工编写适配器后再纳入 IR。

验收：转换 property tests、边界值、错误值、跨 epoch、删除重建和线程错误测试。

## 阶段 2：领域适配器（P1）

按领域拆分生成输出与手写适配器：`core/value`、`scene`、`gui`、`resource`、`animation`、`audio`、`physics`、`engine`。生成器只生成参数解包、结果打包、注册表和文档；所有引擎语义由适配器实现。

场景和 UI 写操作统一进入带 epoch 的 command buffer；消费时验证目标、类型和权限。同步查询从宿主提供的 frame snapshot 读取，避免把引擎可变借用跨 Lua 调用保存。

对 `Transform`、`Widget` 等含 `InheritableVariable` 或消息语义的类型只生成稳定 accessor，不生成裸字段写入。`Resource<T>` 暴露异步状态、错误和取消，不暴露内部加载器锁。

每个领域先实现 20~30 个高价值类型作为垂直切片，再扩大到同类类型。每个新增类型必须同时提交 IR、生成代码、Lua 示例、宿主消费测试和错误测试。

## 阶段 3：覆盖扩展和编辑器工具（P1）

从 API diff 中按调用频率、稳定性、线程安全和适配成本排序。优先公共值类型和稳定组件，延后渲染后端、编辑器内部结构和高度泛型 API。

生成：LuaLS annotations、Markdown API、签名快照、按 crate/领域覆盖率、unsupported 清单和变更摘要。编辑器只读取 catalog，不从运行时 VM 反推 API。

增加 `lua-tool --check`：临时生成并比较；CI 禁止手改 generated.rs。增加 `--crate`、`--feature`、`--profile` 和 `--allowlist`，避免一次生成全部 API 导致编译和审查失控。

## 阶段 4：生命周期、重载与发布质量（P1）

脚本实例使用 scene epoch + node generation + component slot。reload 采用 prepare/commit：新脚本完成解析、构造和无副作用检查后才替换旧实例；失败保留旧实例。模块依赖图决定 package.loaded 失效范围。

事件返回 SubscriptionId，支持取消、多订阅和实例所有权。销毁、reload、场景卸载必须取消回调并释放 RegistryKey。错误默认隔离脚本实例，提供开发期 fail-fast 模式。

发布构建只编译 approved + verified 绑定；编辑器可额外编译诊断绑定。当前先验证 Lua54；已有 LuaJIT feature 需要独立构建矩阵。Luau 仅为后续可选路线，当前项目没有承诺支持。不能把 feature 名称当作运行时切换。

## 量化评估

每个版本生成以下指标：

| 指标 | 定义 |
|---|---|
| 候选覆盖率 | IR discovered / rustdoc public reachable declarations |
| 批准覆盖率 | approved / discovered |
| 生成覆盖率 | generated / approved |
| 行为覆盖率 | verified methods / generated methods |
| 安全拒绝率 | unsupported with reason / discovered |
| 运行时成本 | VM 初始化、注册分配、每帧命令、查询 p50/p95、错误率 |
| 兼容性 | API diff 中 breaking/behavioral changes |

基准场景固定 1k/10k 次值转换、节点查询、Transform 写入、UI 消息、资源状态查询；报告 Lua/Rust 分配和失败路径。不能比较不同 VM/引擎的绝对 FPS，只比较相同环境下的策略。

## 建议优先级和时间盒

第一阶段先完成 IR、rustdoc JSON、转换矩阵和 `Vector/Color/Rect/Handle/Transform/Widget` 垂直切片；第二阶段扩展 UI/Scene 常用组件；第三阶段进入 Resource/Animation/Audio；最后评估 Physics/Rendering/Editor。

每个阶段设置两周时间盒和可交付门槛：生成器可重复、至少一个真实 executor 场景、错误/重载/删除重建测试、API 文档和覆盖率报告。若某类 API 无法满足 ownership 或线程契约，保持 unsupported，不为追求数量生成不安全桥接。

## 覆盖目标和分母冻结

阶段 0 冻结 U = 指定引擎版本、目标平台和 feature 下从 fyrox 可达的 API 身份集合。区分结构体/枚举、自由函数/关联函数/方法、字段 getter/setter；trait 方法去重并保留 impl 关系。重导出别名不重复计数；泛型定义计一次，每个批准实例化另列，不允许用实例数量扩大覆盖率。

同时报告 verified/U（总体覆盖）和 verified/E（适配后允许绑定集合 E 的覆盖）。unsupported 仍留在 U；排除项、原因和数量公开。建议 E 达到 90% 行为验证，且 U 的结构体/可调用项/字段访问三类分别争取 80%；这些是立项目标而非已测结果。若底层后端、泛型和借用项导致 U 无法达到目标，提交差距表与适配成本，不修改分母掩盖差距。

字段写覆盖率只以引擎允许修改的字段/属性为分母，同时披露被禁止写入的字段数。只读属性的 getter 成功不能同时算作 setter 覆盖。alias、目录登记和 wrapper 编译通过均不计行为验证。

## 类型与调用策略的细化

| Rust 形态 | 生成策略 | 拒绝或验证条件 |
|---|---|---|
| `Vector3<f32>` 等依赖泛型 | 白名单单态化，生成本地 newtype 实现 mlua UserData | 外部类型不能直接违反 orphan rule 实现外部 trait |
| `Handle<Node>` / `Handle<UiNode>` | 分域 token，不允许互换 | 域、epoch、generation 全部校验 |
| `&T` 返回 | 小值复制；引擎子对象用 owner token + 访问路径代理 | 不保留 Rust 借用；明确复制后的修改不影响原对象 |
| `&mut Self` 返回 | 值 userdata 可链式返回身份；命令代理返回自身或 nil，文档标记异步 | 不声称调用完成即引擎写入完成 |
| 泛型函数 / `impl Trait` | 指定实例化和参数转换规则；其余报告不支持 | 编译通过才计 generated |
| Iterator / slice / Vec | 有界 snapshot 或分页游标，定义集合变化时的失效 | 不允许迭代器持有图借用跨 Lua yield |
| i64/u64 | Lua54 integer 按范围转换，超范围无损 userdata；LuaJIT 另测 | 禁止无条件经 f64 丢失大整数精度 |
| Result / Option | Result 错误转带 code 的 Lua 错误；Option→nil，避免成功 nil 与错误混淆 | 错误含 source/instance/operation，禁止 unwrap |
| 同时借用多个节点 | 专用 Graph 操作一次性验证和借用 | `get_two_mut` 不导出为两个长期可变 userdata |
| 回调 / Future | 受宿主管理的订阅/任务，完成在主线程投递 | 锁内不回调 Lua；取消后忽略迟到结果 |

快照不能支撑所有同步函数。对于必须返回最新引擎状态的调用，优先提供领域组合操作；必要时引入显式 dispatch scope，在一次 Rust 调用内短期访问宿主，使用借用门禁阻止递归可变访问，禁止 yield 和引用逃逸。若实现需要延长引用至 static，拒绝该实现。基于命令的 API 必须命名/文档明确延迟语义，避免将同步 Rust 方法机械映射为同名异步 Lua 方法。

反射提供 editor 下的字段发现和受控低频读取，setter 仍通过批准适配器。全静态生成适合热路径；全反射不能覆盖普通 Rust 方法，也不能代替类型/生命周期分析。最终采用静态生成 + 手写领域适配器 + 编辑器反射辅助的混合路线。

## 成本、里程碑和决策门

以下为粗略人周预算，按熟悉 Rust/Fyrox 的工程师估算，不是交付承诺；任务相互依赖，不能直接除以人数推算工期。

| 工作包 | 估计投入 | 交付和退出门槛 |
|---|---:|---|
| API 可达性、IR、schema/feature 固定 | 2–4 人周 | 全限定身份、字段类型/可见性、重导出 fixture；确定 U |
| 转换器、静态生成和垂直切片 | 3–5 人周 | 值、句柄、Transform、UI 四条链路编译与行为测试 |
| epoch、调用上下文、错误/事件/重载 | 4–7 人周 | 删除重建、跨场景、重入、清理、回滚测试 |
| 扩展场景/UI/资源/动画/音频 | 6–12 人周 | 按领域覆盖率和实际脚本回归；逐包审查 |
| 兼容、性能、文档与发布 | 2–4 人周 | 生成一致性、API diff、真实 executor 验收 |

合计约 17–32 人周，未包含完整渲染后端/编辑器绑定、Luau 新后端和 Fyrox 大版本迁移。首个垂直切片后按“每新增适配类别成本”重新估算；绝大部分 U 的覆盖可能超出该预算，必须以阶段 0 清单确认。

最先实施的具体工单：修复字段可见性/类型模型；锁定 feature+rustdoc schema；建立可达性测试；设计 IR schema v1；生成一个值类型和一个 scene token；生成 Transform setter 与真实 Graph 状态测试；生成 Widget message 与真实 UI 状态测试。随后才能用同一链路批量扩展，不先把数千个目录项标记为支持。

工具按 `discovery / ir / policy / lowering / emit / report` 模块拆分；运行时按领域拆分生成文件，目录信息不进入插件。tool 的 Fyrox 离线资源编辑 binary 使用可选依赖，核心扫描工具保持轻量。生成 manifest 记录每个文件和 hash，更新仅替换自身产物；CI 既检查生成一致性，也检查误删/孤立绑定。

每个领域包有负责人、API snapshot 和版本说明。破坏性变更必须给旧 Lua 名称兼容期或明确迁移步骤；不能只依赖 Rust 编译错误发现 Lua 用户受到的影响。

## 验收清单

- [ ] 引擎 commit、features、rustc、mlua 后端可复现
- [ ] rustdoc JSON 与重导出闭包已纳入
- [ ] IR 状态和 allowlist 可审查
- [ ] 字段策略区分值复制、setter/message、句柄和禁止访问
- [ ] 每个命令携带 epoch/capability 并在宿主校验
- [ ] wrapper 不保存 Rust 借用、锁 guard 或裸指针
- [ ] reload 可回滚，事件可取消，RegistryKey 无泄漏
- [ ] 生成代码、LuaLS、Markdown、API diff 来自同一 IR
- [ ] unsupported 原因可追踪到 source span
- [ ] executor 有真实 UI 绘制、资源解析和 ButtonMessage::Click 验收
- [ ] workspace check/test/fmt 与生成 `--check` 通过
