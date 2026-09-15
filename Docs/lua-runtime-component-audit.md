# LuaComponent 运行时审计与性能结论

## 当前实现

- 基础数值、字符串和布尔值直接映射到 Lua 原生值；Vector 类型继续使用普通 Lua table，不创建 userdata。
- `LuaComponent.components.entries` 是可序列化配置源；加载时可调用 `rebuild_index` 构建运行时 `HashMap<String, usize>`。
- 构造函数收到 `params.components`。每项包含 `type`、`index`、`generation`，实际 UserData 由宿主绑定层按场景上下文解析。
- 独立的 `EventManager` 已从运行时主流程移除。脚本消息通过 `dispatch_script_event` 调用 `on_event`，UI 回调通过 `RuntimeServices` 保存 `RegistryKey` 并按 `ErasedHandle + UiEventKind` 分发。
- Fyrox 的 `UiMessage` 仍是唯一 UI 消息来源；游戏插件的 `on_ui_message` 负责将 Button Click 转换为脚本事件或回调分发。

## Vec、HashMap 与 Lua 查表

场景资源必须经过 Fyrox `Reflect/Visit`，所以配置使用 Vec。组件数量通常很小，保存和 Inspector 编辑更稳定。运行时 HashMap 只建立一次，避免 `update` 或消息回调重复线性扫描。

Lua table 的字符串查找平均为 O(1)。真正的成本通常是 Lua/Rust 边界、Handle 有效性校验、UI 消息入队和临时 table 分配，而不是 `self.components.button` 本身。建议在 `on_awake` 缓存常用引用，`update` 中直接调用缓存对象；不要每帧按名字搜索场景树。

## 事件生命周期

```text
LuaComponent load -> new(class, params) -> on_awake
Fyrox UiMessage(ButtonMessage::Click)
  -> Game::on_ui_message
  -> LuaRuntime::dispatch_ui_click / dispatch_script_event
  -> RegistryKey 回调或脚本 on_event
update(dt)
on_destroy -> 清理实例关联 RegistryKey
```

`RegistryKey` 不能进入场景序列化，也不能放进 `ScriptMessagePayload`。跨脚本消息应只传递 String、数字、布尔值和可复制的对象句柄；Lua table/function 只能在当前 VM 内通过回调注册表传递。

## 性能测量建议

不要使用未经测量的单次纳秒结论。应分别基准：Lua table 查表、HashMap 组件解析、userdata 方法调用、Handle 校验和完整 Button Click 分发，使用 1,000/10,000 次批量样本统计总耗时。优化优先级应放在跨语言调用次数、临时对象分配和 UI 树搜索。

## 尚未完成的边界

当前回调注册基础设施和运行时按句柄分发入口已完成，但场景 UI 句柄到带 `RuntimeServices` 的 `ButtonRef::add_click(callback)` UserData 注入仍需由游戏 UI 构建层接入。现有 UI 逻辑仍通过 `Game::on_ui_message` 转发脚本事件；下一步接线不应回退到独立事件队列，而应直接使用 Fyrox Button 的 `UiMessage`。
