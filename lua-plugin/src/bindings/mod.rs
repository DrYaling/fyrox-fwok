//! Fyrox 引擎基础绑定。
//!
//! 每个类型都必须在这里显式加入注册入口。这个模块不读取 Lua 源码，
//! 也不根据脚本调用推断 API；新增导出接口必须修改对应模块并重新编译。
mod catalog;
mod generated;
mod manual;
mod registry;

pub use registry::{BindingCategory, BindingMethod, BindingRegistry, BindingStatus, BindingType};

use mlua::Lua;

/// 在 Lua VM 中注册当前已批准的基础引擎绑定。
///
/// 注册顺序是稳定的，适合编辑器和发布版共用；发布版是否裁剪由
/// 编译 feature 或后续显式配置决定，而不是由 Lua 源码决定。
pub fn register_engine_bindings(_lua: &Lua) -> mlua::Result<BindingRegistry> {
    let mut registry = BindingRegistry::default();
    // 目录项只描述 Fyrox 公开 API，未实现 wrapper 的条目不会出现在 Lua 全局表中。
    catalog::register(&mut registry)?;
    manual::register(&mut registry)?;
    generated::register_generated_bindings(_lua, &mut registry)?;
    Ok(registry)
}

/// Registers aliases for the generated common UI and 3D component profile.
/// The aliases only resolve existing resources through `find`; they never build nodes.
pub fn register_generated_component_aliases(lua: &Lua) -> mlua::Result<()> {
    generated::register_generated_component_aliases(lua)
}
