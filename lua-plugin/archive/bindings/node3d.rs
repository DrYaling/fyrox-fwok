//! Fyrox 场景 Node 的 3D 绑定声明。

crate::define_handle_binding!(
    /// 三维节点句柄。对象生命周期由 Fyrox 场景图管理。
    Node3DRef,
    handle = fyrox::core::pool::Handle<fyrox::scene::node::Node>,
    lua_name = "Node3D",
    module = "fyrox.scene",
    lua_module = "scene",
    category = super::BindingCategory::Scene3D,
    description = "Fyrox 场景 Node 的三维类型化句柄代理。"
);
