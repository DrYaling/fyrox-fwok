//! Fyrox 场景 Node 的 2D 语义绑定声明。

crate::define_handle_binding!(
    /// 二维节点句柄。Fyrox 场景图仍为 3D，调用层约定二维语义。
    Node2DRef,
    handle = fyrox::core::pool::Handle<fyrox::scene::node::Node>,
    lua_name = "Node2D",
    module = "fyrox.scene",
    lua_module = "scene",
    category = super::BindingCategory::Scene2D,
    description = "Fyrox 场景 Node 的二维语义句柄代理。"
);
