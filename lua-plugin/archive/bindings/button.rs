//! Fyrox UI Button 绑定声明。

crate::define_handle_binding!(
    /// Button 类型化句柄。实际 UI 操作必须经由 UI 上下文执行。
    ButtonRef,
    handle = fyrox::core::pool::Handle<fyrox::gui::button::Button>,
    lua_name = "Button",
    module = "fyrox.ui",
    lua_module = "ui",
    category = super::BindingCategory::Ui,
    description = "Fyrox UI Button 的类型化句柄代理。"
);
