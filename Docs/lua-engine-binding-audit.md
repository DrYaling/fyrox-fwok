# Lua 引擎接口绑定审计与性能评估

## 结论

当前实现是可运行的 Lua 原型，但与“绑定 Fyrox 引擎接口、对象、组件，而不是场景/UI 节点业务逻辑”的要求存在较大出入。`chat.append`、`ui.show`、`inventory.items` 实际绑定的是 `game/src/lib.rs` 中的业务桥接表和字符串命令；它们不能取得 Fyrox 对象，也不能调用组件方法。所谓 reflection 目前只是 `reflection.call(object, method, Vec<String>)` 的字符串回调，并未接入 Fyrox 的类型元数据或对象解析。

Fyrox 的 `Reflect` trait 能枚举字段、读写字段和类型信息，但不提供任意 Rust 方法调用。Fyrox `TextBoxMessage` 也没有 `Append` 变体；文本修改必须通过合法的 UI 消息（或在 UI 线程上的适配器中读取当前文本后发送 `TextMessage::Text`）。因此不能仅凭反射自动实现 `input:append()`。

## 现状证据

- `game/src/lib.rs` 的 `Api` 暴露 `game/ui/chat/inventory/events` 表，所有操作进入 `Arc<Mutex<Bridge>>`，再由 `Game::update` 按 `"hud"`、`"inventory_label"`、`"chat_log"` 等字符串映射到固定句柄。
- `Command::Chat` 最终向固定 `Handle<Text>` 发送 `TextMessage::Text`，Lua 先拼接完整聊天记录；这不是 TextBox/Input 组件绑定，并且每次发送会复制增长中的字符串。
- 编辑器的 `register_reflection` 直接调用普通注册函数，编辑器模式和完整模式没有不同的引擎绑定后端。
- `manifest.rs` 是简单词法扫描器，只能发现形如 `module.member(` 的静态调用；它不推断对象类型、Rust 签名或生成 wrapper，不能把变量 `chat` 解析为某个 UI 组件。
- `runtime.rs` 每帧按字符串查找生命周期函数，并将每个事件广播给每个脚本；事件名和 payload 都是堆分配的 `String`。

## 目标 API 与对象模型

Lua 名称可以保持稳定，但绑定对象必须携带 Fyrox 类型和句柄，而不能依赖节点名字：

```lua
function Player:on_awake()
    self.chat_input = self.ui:find_text_box(self.chat_input_handle)
end

function Player:on_event(name, payload)
    if name == "chat.send" then
        self.chat_input:append(payload) -- 适配器内部发送合法 TextMessage
    end
end
```

这里的 `TextBoxRef` 应保存 `Handle<TextBox>` 与所属 `UserInterface` 访问上下文；`ButtonRef`、`TextRef`、`WidgetRef`、场景 `NodeRef` 应分别使用各自图/容器的句柄。UI 节点和场景节点是不同图，不能设计成统一的 `scene Node:get_component()`。句柄失效必须返回 Lua 错误，而不能持有 Rust 借用或 `'static` 引用。

`append` 是 Lua 友好的适配器语义，不一定是 Fyrox 原生方法：适配器在安全处理点读取文本并发送新的 `TextMessage::Text`，同时遵守 UI 消息队列和布局更新规则。同一帧连续 append 时，不能每次读取尚未消费消息的旧文本，否则会丢失前一次追加；必须定义有序命令处理或待提交文本状态，并测试连续追加、Unicode、清空后追加和对象删除。高频数值属性可提供专用 typed setter，避免通用反射路径。聊天历史应设置容量上限，必要时按行展示，而不是无限重建全文。

## 三种机制的职责

### 绑定注册（必要，但只做一次）

注册是把 Rust wrapper、userdata metatable、模块函数装入 Lua VM。每个 VM/类型注册一次，启动成本是可接受的；不能在每帧或每个节点重复注册。完整模式应编译生成 wrapper，发布包强制 `PackageFull`。编辑器可以默认反射后端，也允许切换完整后端；若发布配置为反射，必须输出 warning 并降级为完整绑定。

自动绑定工具应根据显式导出清单和 Lua 静态依赖生成注册代码，但不能仅凭变量名猜类型。动态调用、`obj:method`、返回对象和长字符串会使当前扫描器漏报；发布构建应采用保守导出集，并把无法解析的动态依赖作为错误或 warning。

### 反射（适合编辑器，不是性能优化）

反射适合属性面板、类型发现和调试。应缓存 `TypeInfo`/字段索引，调用时传递已解析的对象句柄和 typed value。方法调用需要 Fyrox 类型专用 descriptor/宏生成的 call shim；不能把任意方法压成字符串数组后“通用调用”。字段存在也不代表可直接修改：涉及布局、通知或其他引擎不变量的属性必须走专用适配器。发布模式不应依赖 Lua 动态反射派发，这不等于移除 Fyrox 自身所需的 Reflect 实现。

### 事件发射（用于解耦，不用于普通属性访问）

`EventManager` 适合按钮点击、输入提交、生命周期通知等离散事件；不应把 `set_text`、位置读取等普通调用转成事件。事件 ID 应使用静态/interned 标识，订阅者按 ID 路由，避免每个事件广播给所有脚本。事件在引擎安全更新点批量派发；只有确有跨线程需求时才使用锁/频道。

## 性能模型与验证计划

成本拆分为 Lua/Rust 边界转换、方法查找、句柄校验、分配/拷贝、同步、排队和引擎实际处理。typed userdata 可消除通用字符串解析，但缓存动态派发、队列批处理可能改变实测结果，不能给出无条件的总排序。UI 布局和消息处理本身可能高于 Lua 调用成本，不能只测函数调用。LuaJIT 也不会自动消除 Rust 回调、消息队列和布局成本，应作为独立基准变量。

应在 release、同一 Lua 后端下比较 Rust 直调、typed userdata、缓存反射、当前桥接四条路径；测试 1k/10k/100k 次调用，记录启动扫描时间、分配次数、p50/p95、每帧耗时和 UI 布局耗时。另测事件订阅者 1/10/100 个的广播开销。没有这些数据前，不应宣称“反射更快”或固定倍数收益。

## 分阶段迁移

1. 先实现 `UserInterfaceContext + TextBoxRef/TextRef/ButtonRef`，用真实句柄完成聊天和背包 UI；保留 Lua 生命周期 API。
2. 为编辑器提供字段反射和调试调用，为发布生成 typed wrapper；统一 Lua 表面 API，后端透明切换。
3. 用显式导出元数据替换当前词法扫描的唯一决策，生成模块化注册代码；动态依赖必须显式声明。
4. 将 `EventManager` 改为 typed/interned ID 和订阅路由，增加句柄失效、reload 回滚和状态迁移测试。
5. 完成上述基准后再裁剪可选绑定；以正确性、UI 消息语义和帧预算共同决定是否使用事件或反射。

## 编译边界与模块划分

新增 Lua 文件或调用已经导出的引擎方法，只需加载/热重载 Lua，不需重编译编辑器或 game。新增未导出的 Rust 方法仍须生成并编译 wrapper；反射不能让未生成调用入口的任意 Rust 方法凭空可调用。发布裁剪后新增 Lua 若引用被裁掉的接口，也必须重新生成发布包。因此开发期优先注册稳定且较宽的导出集，使用扫描主要用于诊断和可选裁剪，而不是承诺从任意动态 Lua 精确推断全部依赖。此边界与 tolua 风格的预生成调用桥接思路一致，不代表复用 tolua 的 C# 实现。

建议保留 runtime/config/events 基础模块，新增 bindings/ui、bindings/scene、bindings/values、context/handles；editor/reflection 与 package/generated 通过 feature 隔离。生成工具单独管理导出类型、签名、Lua AST 和动态依赖清单。game 只包含玩法、背包对象导出和场景装配，不承担通用 Fyrox 类型注册。lib.rs 仅组织模块与公共导出。

完整绑定应定义为“批准的导出契约全部有编译期 wrapper”，而不是“Lua 扫描命中的业务函数已注册”，也不等于暴露引擎全部内部 Rust API。自动生成工具必须校验未知成员、参数类型、返回对象依赖及 feature 可用性；未解析依赖默认阻止裁剪发布，显式保守保留后才允许继续。

验收至少包含：两套独立 UI 中同类型对象复用同一绑定；更换节点名称无需修改绑定器；失效句柄安全报错；编辑器两种模式行为一致；发布反射配置触发 warning 且实际走完整后端；构造/on_awake/start/update/on_event/on_destroy/析构顺序、订阅释放与热重载失败回滚均有测试。生命周期函数引用可缓存，但成功热重载必须更新缓存，不能复用旧 Lua 函数。

截至本审计，编辑器反射调用仍采用显式 descriptor/shim，未登记的方法不会被动态调用；TextBox typed proxy 和自动 wrapper 生成仍未实现。LuaComponent 的资源选择、Inspector 参数/源码编辑、场景扫描和生命周期已有单元测试，但 UI 反射调用仍需要独立的端到端测试和性能基准。
