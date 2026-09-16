# AI 执行规格：Fyrox 大范围 Lua 绑定

版本：1.0
适用仓库：`F:/WorkSpaceNew/Fyrox/fwok`
引擎来源：相邻 `../Fyrox`，必须先记录 commit、工作树状态、Rust toolchain、Cargo.lock 和启用 features。

本文是后续 AI agent 的执行协议。每个 agent 必须先读 `AGENTS.md`、`RTK.md`、本文件和相关专项文档；所有命令使用 `rtk` 前缀。不得把“目录生成成功”报告为“运行时绑定完成”。

## 1. 总体执行规则

本文新目录、CLI 子命令和测试 ID 均为待实现规格。当前 legacy generator 不读取 [policy 样例](../config/lua-binding-policy.toml)。先实现命令再执行依赖它的验收；用户当次指令优先，不要求自动委派 agent 或重复审批。

### 1.1 每个任务开始时必须输出

```text
任务 ID:
目标 crate / 领域:
输入 commit、feature、toolchain:
当前生成器版本:
预计修改文件:
不支持的 API 类别:
验证命令:
```

如果工作区有用户未提交修改，先执行 `rtk git status --short` 并记录；不得重置、清理或覆盖无关修改。生成物只允许写入任务声明的输出路径。

### 1.2 完成判定

以下针对运行时绑定任务。Discovery、schema 等基础设施任务以自身 fixture 和接口测试验收；unsupported 表示分类完成，不表示绑定目标完成。

任务只有在以下条件全部满足时才算完成：

1. IR 中的 API 状态为 `verified`，或明确为 `unsupported` 并有原因。
2. 生成文件通过 `cargo fmt` 和 `cargo check`。
3. 至少一个 Lua 脚本测试真实调用；涉及场景/UI/资源时必须由 Fyrox 宿主消费并检查最终状态。
4. 错误路径、删除重建、epoch/generation 和线程约束有测试。
5. API diff、覆盖率和 unsupported 报告已更新。
6. 最终报告包含修改、风险、未完成项和实际命令输出摘要。

## 2. 目录和职责

```text
lua-tool/src/
  discovery/       # rustdoc JSON / 源码候选发现，只读引擎输入
  ir/              # schema、序列化、ID、版本校验
  policy/           # allowlist、类型分类、unsupported 原因
  lowering/         # Rust 类型到 Lua ABI 的中间表示
  emit/              # runtime Rust、LuaLS、Markdown、JSON、diff
  report/            # manifest、coverage、errors、benchmark
  metadata/         # 迁移中的人工目录，最终由 IR 替代
lua-plugin/src/
  bindings/         # 仅可执行注册和生成 wrapper
  value_bindings.rs  # 值 userdata 和转换
  handles.rs         # token、epoch、generation、capability
  scene_adapter.rs   # 场景/节点领域手写语义适配器
  ui_adapter.rs      # UI 消息和查询适配器
  resource_adapter.rs
  binding_errors.rs
  command_buffer.rs
data/scripts/bindings/ # Lua API smoke tests and examples
target/lua-bindings/   # 所有生成输出，不手工编辑
```

当前仓库尚未有上述完整目录。AI 应分阶段创建；不要一次性移动全部 `game_api.rs`。每次移动必须先保留行为测试，保证编译和测试可回归。

## 3. Binding IR v1

### 3.1 JSON 示例

```json
{
  "schema": 1,
  "engine": {"repo": "../Fyrox", "commit": "...", "features": ["backend_opengl"], "rustc": "1.94"},
  "id": "fyrox.core.algebra.Vector3<f32>",
  "rust_path": "fyrox::core::algebra::Vector3<f32>",
  "lua_path": "fyrox.core.Vector3",
  "kind": "value_struct",
  "crate": "fyrox-core",
  "module": "algebra",
  "visibility": "public_reachable",
  "constructors": [{"rust": "new", "lua": "new", "args": ["f32", "f32", "f32"]}],
  "fields": [
    {"rust": "x", "lua": "x", "type": "f32", "read": "copy", "write": "copy", "status": "approved"}
  ],
  "methods": [],
  "ownership": "owned_userdata",
  "threading": "any_lua_thread",
  "error_policy": "lua_error",
  "status": "approved",
  "source": {"file": ".../vector.rs", "line": 42},
  "tests": ["value.vector3.roundtrip", "value.vector3.nan_rejected"]
}
```

### 3.2 强制字段

每个 type/method/field 必须具有：稳定 ID、Rust 完整路径、Lua 完整路径、kind、参数/返回类型、可见性、ownership、threading、错误策略、状态、source span、测试 ID。缺任一项只能为 `discovered` 或 `unsupported`。

### 3.3 状态机

```text
discovered -> classified -> approved -> generated -> verified
verified -> deprecated (引擎标记或兼容期结束)
```

状态转换必须写入 `target/lua-bindings/status.json`。AI 不得直接把 `CatalogOnly` 替换为 `Implemented`；必须提交测试 ID 和宿主语义。

unsupported 可在新增适配器后回到 classified。rejected、compile_error 仅为诊断码，编译失败保持 approved。开发生成允许 approved 进入编译测试，发布仅选择 verified；证据关联输入 hash，输入变化后重新验证，避免验证与生成循环依赖。

## 4. Discovery 实现要求

### 4.1 输入顺序

1. 用固定 nightly 生成引擎及依赖的 rustdoc JSON。
2. 解析 `fyrox` facade 的 re-export，递归到实际 crate。
3. 解析 public impl、trait impl、字段 visibility、cfg、deprecated、文档和 source span。
4. 对泛型保留定义记录；只有 allowlist 中的单态化实例才能进入 `approved`。
5. 用源码 `syn` 做位置和文档补充，不用词法正则决定可见性或类型。

### 4.2 Discovery 禁止事项

- 不把 `pub(crate)`、测试项、注释文本、宏调用文本计作 API。
- 不把同一符号的重导出重复计数。
- 不根据函数名猜测线程安全、同步/异步或字段可写性。
- 不因 Rust 编译成功就自动进入 `verified`。

### 4.3 生成前静态检查

必须拒绝或人工分类以下签名：裸指针、`unsafe fn`、`dyn Trait`、锁 guard、引用逃逸、未实例化泛型、`impl Trait`、外部 trait orphan 实现、跨线程不可证明类型、Future/Stream、回调闭包、渲染后端上下文。报告中必须写 Rust 路径和拒绝原因。

## 5. 类型和 ABI 设计

### 5.1 Lua userdata 约定

值 userdata 持有 Rust 值副本；句柄 userdata 只持有 token；适配器 userdata 持有 `Rc`/主线程宿主的受控索引，不能保存 `&mut`、锁 guard 或 `ScriptContext` 引用。所有 userdata 方法先验证 owner thread 和 capability。

### 5.2 转换规则

| Rust | Lua | 规则 |
|---|---|---|
| bool/string | boolean/string | 直接转换，错误类型返回带参数名的 Lua error |
| f32/f64 | number | 拒绝 NaN/Infinity 的 API 由 IR 标记；否则保留语义 |
| i8..i32 | integer | 范围检查，禁止静默截断 |
| i64/u64 | integer/userdata | Lua54 在范围内用 integer；超范围用无损整数 userdata |
| Option<T> | nil/T | nil 只表示缺失，不表示错误 |
| Result<T,E> | T 或 error | error 包含 code、operation、source、instance |
| Vec/array | snapshot table | 复制快照；不保留 Rust slice |
| enum | string/integer table | 未知值拒绝；序列化名称稳定 |
| Handle<T> | typed token userdata | 校验 domain、epoch、generation、capability |
| Resource<T> | resource proxy | status/data/error/cancel；不暴露内部锁 |

### 5.3 同步与命令

Rust getter 只有在一次受控 dispatch scope 内读取并复制结果；Lua yield 前不得保留借用。setter 若通过 Fyrox message 或 command buffer 执行，Lua 方法命名和文档必须标记延迟执行；命令携带 epoch 和 capability，消费时再次验证。

## 6. 领域执行顺序

### Phase A：值类型

先实现 `Vector2/3/4`、`Quaternion`、`Matrix4`、`Color`、`Rect`、简单枚举和数组。每类包括构造、字段读写、转换、序列化、错误测试和 LuaLS 文档。不得在此阶段引入 Scene 或 UI 借用。

### Phase B：场景 token

新增 `SceneEpoch`、`NodeToken`、`SceneNodeRef`。`scene.find` 只在明确 scope 内解析并返回 token；名称不是长期身份。实现 position/rotation/scale/enabled、父子查询和删除失效测试。跨 scene、同 index 不得互相解析。

### Phase C：UI

按控件类型建立 capability：Widget、Text、TextBox、Button、CheckBox、ListView、Grid、Window。setter 必须转换为 Fyrox UI message；读取通过宿主同步快照。`ui.button(id)` 必须拒绝非 Button 节点。Click 返回 SubscriptionId，支持 cancel 和 reload 清理。

### Phase D：资源/动画/音频

先实现资源状态、路径、加载错误和取消，再暴露 Texture/Material/AnimationPlayer/AudioSource 的窄操作。资源代理不得跨 manager 或 scene 复用；异步完成回调必须回主线程并验证 epoch。

### Phase E：物理/渲染/编辑器

只有完成线程、生命周期和后端 feature 矩阵后才评估。物理世界和渲染资源不能按普通字段生成。编辑器专用绑定使用 editor feature，发布版拒绝链接。

## 7. 生成器输出契约

每次生成必须输出：

```text
target/lua-bindings/
  ir.json
  approved.json
  unsupported.json
  status.json
  catalog.json
  api-diff.md
  coverage.json
  luadoc/*.lua
  generated/*.rs
  manifest.json
```

`manifest.json` 记录输入 commit、features、toolchain、生成器版本、每个文件 SHA256。`--check` 将生成结果写入临时目录并逐文件比较，发现手工修改或非确定性立即失败。生成 Rust 只包含 wrapper 和注册代码；说明、目录和 LuaLS 不进入 `lua-plugin` 二进制。

建议命令接口：

```text
lua-tool discover --engine ../Fyrox --features backend_opengl --out target/lua-bindings
lua-tool classify --ir target/lua-bindings/ir.json --policy config/lua-binding-policy.toml
lua-tool generate --approved target/lua-bindings/approved.json --runtime-out lua-plugin/src/bindings/generated
lua-tool report --manifest target/lua-bindings/manifest.json
lua-tool check --engine ../Fyrox --policy config/lua-binding-policy.toml
```

现有 `lua-tool` 只有单一 main 参数解析；AI 应先兼容旧命令，再逐步拆分子命令，不要一次改写导致脚本失效。`--input lua-plugin/src` 仅能继续作为 legacy profile。

## 8. 测试矩阵

### 8.1 工具测试

- rustdoc fixture：re-export、cfg、trait impl、泛型、deprecated、宏位置。
- IR schema：缺字段、重复 ID、Lua 名冲突、版本升级、确定性排序。
- policy：每种 unsupported 签名均能稳定分类。
- generator：同输入相同 hash；编译失败 wrapper 不进入输出；`--check` 检出手改。

### 8.2 Runtime 测试

- 转换：边界整数、NaN、Infinity、nil、未知 enum、Option/Result。
- token：错误 domain、epoch、generation、capability、跨线程。
- scene：删除重建同名节点、rename/reparent、跨 scene、命令过期。
- UI：真实 Text/Button/CheckBox 消息、类型拒绝、Click/cancel、重载清理。
- resource：加载成功/失败/取消/manager 更换。
- lifecycle：构造失败回滚、on_destroy 继续清理、RegistryKey 无泄漏、回调重入。

### 8.3 真实宿主验收

`executor` 必须运行真实 `.ui`/`.rgs` 资源，日志证明资源解析、`draw_commands > 0`，并将真实 `ButtonMessage::Click` 路由到 Lua。只看到 Lua lifecycle log 不算 UI 验收。

## 9. AI 任务拆分模板

每个子任务使用以下格式创建：

```text
ID: BIND-<领域>-<序号>
前置: <任务 ID>
输入: <engine commit/features/IR 文件>
允许修改: <明确文件列表>
实现: <一个可验证行为>
必须拒绝: <不支持签名>
测试: <unit/integration/executor 命令>
输出: <生成文件、coverage、unsupported、diff>
完成标准: <可测条件>
```

推荐首批任务：

1. `BIND-DISCOVERY-001`：锁定 rustdoc JSON toolchain，建立 fixture 和 facade re-export 解析。
2. `BIND-IR-001`：实现 schema v1、稳定 ID、serde roundtrip、重复检测。
3. `BIND-CONVERT-001`：实现 Lua54 数值/Option/Result/enum 转换和边界测试。
4. `BIND-VALUE-001`：Vector/Color/Rect 垂直切片，生成 wrapper、LuaLS、行为测试。
5. `BIND-HANDLE-001`：Node/UI token 的 epoch/generation/capability 校验。
6. `BIND-SCENE-001`：Transform 适配器和过期命令测试。
7. `BIND-UI-001`：Text/Button capability、message、Click subscription。
8. `BIND-TOOL-001`：manifest、coverage、api-diff、`--check`。

AI 不得同时领取互相改写同一 generated 文件的任务；生成文件由最后一步统一生成。每个任务结束后提交 `git diff --check`、相关 cargo 命令和未解决风险。

## 10. 失败处理和回滚

生成器失败、wrapper 编译失败、运行时行为失败时，保留 `discovered/classified/unsupported` 报告，删除失败的 generated 临时目录，不修改上一次可用产物。生成采用临时目录 + 原子替换；禁止先删除生产 generated.rs。

运行时绑定失败默认隔离当前脚本实例并记录结构化错误；开发模式允许 fail-fast。热重载先 prepare 新实例，成功后 commit；任何构造、绑定或 `on_awake` 错误都保留旧实例和旧回调。

如果底层 Fyrox API 变化导致适配器无法证明安全，标记 `unsupported` 并提交迁移说明，不能用 `unsafe`、`transmute`、静态可变引用或吞错恢复编译。

## 11. 最终发布门槛

下列 `lua-tool check` 为后续实现后的验收命令，现在执行会失败。其他命令可用于当前基线。executor 是交互进程，须按资源验收流程操作并记录结束结果，不能将成功启动视为通过。

```powershell
rtk cargo fmt --all -- --check
rtk cargo check --workspace
rtk cargo test -p lua-plugin -p lua-tool --lib --bins
rtk cargo run --bin lua-tool --manifest-path lua-tool/Cargo.toml -- check --engine ../Fyrox --policy config/lua-binding-policy.toml
rtk cargo run -p executor
rtk git diff --check
```

发布报告必须列出：引擎输入和 hash、U/E 分母、verified/generated/unsupported 数量、按 crate/领域覆盖率、未实现高风险 API、性能基准、真实 UI 证据、兼容性 diff 和回滚方案。
