# Lua 组件引用、引擎事件与性能设计

## 1. 设计结论

本方案遵循 mlua 官方示例的显式注册方式，并满足以下约束：

- i32、f32、bool、String 等基础值直接使用 Lua 原生值；
- Vector2、Vector3、Vector4、Color 等简单聚合值使用 Lua table，不实现 UserData；
- Node、Button、Text、TextBox、Resource 等具有引擎身份和生命周期的对象使用 UserData 代理；
- 删除 lua-binding 自有 EventManager，UI 事件来源统一为 Fyrox UiMessage，脚本消息统一为 Fyrox ScriptMessage；
- LuaComponent 的组件配置使用可序列化 Vec，运行时构建 HashMap 索引；
- Lua 回调由 LuaRuntime 的 RegistryKey 注册表持有，组件只保存声明和引擎句柄。

## 2. 简单值类型

基础类型直接映射：

| Rust | Lua |
| --- | --- |
| bool | boolean |
| i8..i64、u8..u32 | integer |
| f32、f64 | number |
| String、&str | string |
| Option<T> | T 或 nil |
| Vec<T> | array table |

Vector 使用普通 table：

```lua
local position = { x = 1.0, y = 2.0, z = 3.0 }
node:set_position(position)
```

Rust 侧集中转换：

```rust
fn vector3_from_lua(value: Value, lua: &Lua) -> mlua::Result<Vector3<f32>>;
fn vector3_into_lua(value: Vector3<f32>, lua: &Lua) -> mlua::Result<Table>;
```

不建议在每个绑定函数中重复解析 table。转换模块必须统一检查缺失字段、数值类型和 NaN/Infinity。高频批量数据应使用数组 table 或批处理函数，避免为每个 Vector 创建一次跨语言调用。

## 3. LuaComponent 组件表

### 3.1 持久化结构

Fyrox 当前 Reflect/Visit 不能直接处理 `BTreeMap<String, Component>`。场景和 Inspector 数据应使用 Vec：

```rust
#[derive(Clone, Debug, Reflect, Visit)]
pub struct ComponentBinding {
    pub component_type: ComponentType,
    pub handle: ErasedHandle,
    pub key: String,
}

#[derive(Clone, Debug, Reflect, Visit)]
pub struct ComponentBindings {
    pub entries: Vec<ComponentBinding>,

    #[visit(skip)]
    #[reflect(hidden)]
    index: HashMap<String, usize>,
}
```

这仍然是 Map 封装：Vec 是稳定的序列化源，HashMap 是运行时索引。加载场景、Inspector 修改 key 或热重载后调用 `rebuild_index()`。

不建议把 `type` 保存为任意字符串。应使用可序列化枚举：

```rust
pub enum ComponentType {
    Node,
    Button,
    Text,
    TextBox,
    Image,
    Custom(String),
}
```

### 3.2 Vec、HashMap、BTreeMap 对比

| 结构 | 查询 | 优点 | 缺点 | 用途 |
| --- | --- | --- | --- | --- |
| Vec | O(n) | Reflect/Visit/Inspector 兼容最好、内存连续 | 按 key 高频查询变慢 | 序列化源 |
| HashMap | 平均 O(1) | 高频 key 查询最快 | 顺序不稳定、额外内存、不能直接 Reflect | 运行时索引 |
| BTreeMap | O(log n) | 顺序稳定 | 查询慢于 HashMap，当前不能直接 Reflect/Visit | 不采用 |

通常 LuaComponent 只有 3 到 20 个引用。单次 Vec 查找并不昂贵，但如果每帧、每个脚本、每次方法调用都查找，累计成本会超过一次性构建 HashMap 的成本。因此推荐：编辑数据保存在 Vec，实例创建时建立 HashMap，并把解析结果放进 Lua table。

### 3.3 注入 Lua 实例

Runtime 创建脚本实例时生成：

```lua
self.components = {
    submit = ButtonRef,
    chat_input = TextBoxRef,
    chat_log = TextRef
}
```

脚本只在 on_awake 缓存常用引用：

```lua
function Controller:on_awake(ctx)
    self.submit = ctx.components.submit
    self.chat_input = ctx.components.chat_input
end
```

禁止 Lua 通过裸 index/generation 构造对象。所有 UserData 必须由 Runtime 根据 ComponentBinding 创建并校验类型。

## 4. 删除独立 EventManager

### 4.1 UI 事件

Button、TextBox 等事件已经进入 `Plugin::on_ui_message`。Runtime 不再复制一个通用字符串事件队列，而是直接分发 Fyrox UI 消息：

```text
Fyrox ButtonMessage::Click
 -> Game::on_ui_message
 -> LuaRuntime::dispatch_ui_message(ui_handle, destination, Click)
 -> CallbackRegistry 按 (ui, node, event) 查找
 -> 调用对应 Lua Function
```

### 4.2 脚本消息

Rust 脚本之间的消息使用 Fyrox：

```rust
ctx.message_dispatcher.subscribe_to::<LuaScriptMessage>(ctx.handle);
ctx.message_sender.send_to_target(target, LuaScriptMessage { ... });
ctx.message_sender.send_global(LuaScriptMessage { ... });
```

Lua 对外 API 只做薄封装：

```lua
fyrox.messages.send(target, "inventory.changed", payload)
fyrox.messages.send_global("quest.completed", payload)
```

由于 ScriptMessagePayload 必须是静态 Rust 类型，定义一个统一载荷：

```rust
#[derive(Debug, ScriptMessagePayload)]
pub struct LuaScriptMessage {
    pub name: String,
    pub payload: LuaMessageValue,
}
```

`LuaMessageValue` 应是可 Send 的值枚举，不能保存 Lua Table、Function 或 RegistryKey。建议支持 Nil、Bool、Integer、Number、String、Array 和 ObjectId。收到消息后再转换为 Lua table。

## 5. Unity UGUI 风格回调

目标 API：

```lua
function Controller:on_awake(ctx)
    self.submit = ctx.components.submit
    self.submit:add_click(function(button)
        self.chat_log:append(self.chat_input:text())
    end)
end
```

ButtonRef 是 UserData，保存：

```rust
struct ButtonRef {
    ui: Handle<UserInterface>,
    handle: Handle<Button>,
    owner_instance: ScriptInstanceId,
    callbacks: Rc<RefCell<CallbackRegistry>>,
}
```

`add_click` 接收 `mlua::Function`，将函数转换成 RegistryKey：

```rust
methods.add_method("add_click", |lua, this, callback: Function| {
    let key = lua.create_registry_value(callback)?;
    this.callbacks.borrow_mut().insert(
        CallbackTarget::button_click(this.ui, this.handle),
        this.owner_instance,
        key,
    );
    Ok(CallbackToken::new(...))
});
```

建议返回 CallbackToken，并支持：

```lua
local token = button:add_click(callback)
token:disconnect()
button:remove_click(callback) -- 不推荐，Lua Function 身份比较不稳定
button:clear_clicks()
```

Runtime 在脚本 `on_destroy`、热重载或场景卸载时按 owner_instance 清理全部 RegistryKey，防止回调持有旧 Lua table。

Lua 原表用于脚本侧对象关系和回调函数保存；真正的监听索引必须由 Runtime 持有，因为 Fyrox 消息到达时需要从 UI handle 反向找到 Lua Function。

## 6. 生命周期与事件顺序

```text
加载 LuaComponent
 -> 构建 ComponentBindings.index
 -> 验证句柄和组件类型
 -> 创建 UserData 并生成 ctx.components Lua table
 -> new(class, params)
 -> on_awake(ctx)，允许 add_click
 -> start(ctx)
 -> Fyrox UI/ScriptMessage 到达
 -> Runtime 调用已注册回调
 -> update(dt)
 -> on_destroy()
 -> 清理该实例的全部 RegistryKey
```

事件回调中再次注册或移除回调时，先复制待调用 key 列表，再调用 Lua，避免持有 CallbackRegistry 的 RefCell 可变借用导致递归 panic。

## 7. Lua 查表性能评估

Lua table 是哈希表和数组混合结构，按字符串 key 查询平均为 O(1)。组件表通常很小，单纯 `self.components.submit` 的开销不是主要瓶颈。更大的成本来自：

- Rust -> Lua 或 Lua -> Rust 的跨语言调用；
- 创建临时 Lua table，尤其是每帧创建 Vector table；
- 字符串分配和 UTF-8 转换；
- 每次调用都从 Fyrox Pool 校验和解析 Handle；
- Reflect 字段路径解析；
- 锁竞争或跨线程 channel。

推荐脚本写法：

```lua
function Controller:on_awake(ctx)
    self.button = ctx.components.submit
end

function Controller:update(dt)
    -- 使用缓存的 UserData，不重复按字符串查找组件。
end
```

避免：

```lua
function Controller:update(dt)
    fyrox.ui.find("submit"):set_visible(true)
end
```

即使每帧查一次 Lua table 通常也可接受，但组件查找后再跨 Rust 边界、扫描 UI 树才是真正的热点。

### 性能策略

- on_awake 解析组件，update 使用缓存；
- CallbackRegistry 使用 HashMap；
- 组件序列化 Vec 只在加载/编辑后重建索引；
- Reflect 路径只在编辑器低频操作使用；
- 高频 Vector 运算尽量留在 Lua table 内，跨边界时批量提交；
- UI 修改使用 Fyrox UiMessage 批量排队；
- 不在每次事件分发中复制完整 Lua table。

不能脱离硬件和脚本规模给出可靠纳秒数。应添加 Criterion 基准，分别测量 Lua 原生查表、HashMap 组件解析、UserData 方法调用、Handle 校验和完整 Button Click 分发。验收目标应以一帧 1,000 次和 10,000 次调用的总耗时为准，而不是单次微基准。

## 8. 模块调整建议

```text
lua-binding/src/
  component/
    binding.rs       # ComponentBinding、ComponentBindings
    resolver.rs      # Handle 和类型验证
  callback/
    registry.rs      # RegistryKey 生命周期
    ui.rs            # UiMessage 分发
  message/
    payload.rs       # LuaScriptMessage、LuaMessageValue
    bridge.rs        # ScriptMessage 转换
  bindings/
    button.rs
    text.rs
    text_box.rs
    node.rs
  value/
    vector.rs        # Lua table 与 Vector 转换
  runtime.rs
```

移除 `events.rs` 后，`LuaGameApi::register` 不再接收 EventManager，而是接收 RuntimeServices。RuntimeServices 提供 CallbackRegistry、组件解析器和引擎消息发送适配器。引擎核心不需要修改。

## 9. 实施顺序

1. 新增 ComponentBinding/ComponentBindings，先完成 Vec 序列化和 HashMap 重建测试。
2. 在 LuaComponent 初始化时生成 `ctx.components`。
3. 实现 CallbackRegistry 和 ButtonRef::add_click。
4. 将 Game::on_ui_message 直接接入 CallbackRegistry。
5. 定义 LuaScriptMessage，接入 Fyrox ScriptMessageSender/Dispatcher。
6. 移除 EventManager 和字符串事件队列。
7. 修改 ui_controller.lua 使用 `ctx.components` 和 `button:add_click`。
8. 添加生命周期清理、热重载清理和性能基准。

