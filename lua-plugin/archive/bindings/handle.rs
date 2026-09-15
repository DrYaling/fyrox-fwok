//! tolua 风格的句柄 userdata 模板。
//!
//! Fyrox 对象由 generational handle 标识。不同领域只需声明 Rust 句柄类型、Lua 类型名和
//! 命名空间，通用模板会生成 `from_handle`、`index`、`generation`、`is_none` 及注册元数据。

/// 为一个 Fyrox handle 类型生成轻量 Lua userdata 代理和静态注册函数。
#[macro_export]
macro_rules! define_handle_binding {
    (
        $(#[$meta:meta])*
        $name:ident,
        handle = $handle_ty:ty,
        lua_name = $lua_name:literal,
        module = $module:literal,
        lua_module = $lua_module:literal,
        category = $category:expr,
        description = $description:literal
    ) => {
        $(#[$meta])*
        #[derive(Clone, Copy, Debug, PartialEq, Eq)]
        pub struct $name {
            index: u32,
            generation: u32,
        }

        impl $name {
            pub fn from_handle(handle: $handle_ty) -> Self {
                Self { index: handle.index(), generation: handle.generation() }
            }

            pub fn handle(self) -> $handle_ty {
                fyrox::core::pool::Handle::new(self.index, self.generation)
            }
        }

        impl mlua::UserData for $name {
            fn add_methods<M: mlua::UserDataMethods<Self>>(methods: &mut M) {
                methods.add_method("index", |_, this, ()| Ok(this.index));
                methods.add_method("generation", |_, this, ()| Ok(this.generation));
                methods.add_method("is_none", |_, this, ()| Ok(this.index == 0 && this.generation == 0));
            }
        }

        pub(super) fn register(
            lua: &mlua::Lua,
            registry: &mut super::BindingRegistry,
        ) -> mlua::Result<()> {
            let fyrox = super::namespace::ensure_fyrox_table(lua)?;
            let parent = super::namespace::ensure_path(lua, &fyrox, $lua_module)?;
            let class = lua.create_table()?;
            class.set("from_handle", lua.create_function(|_, (index, generation): (u32, u32)| {
                Ok($name { index, generation })
            })?)?;
            parent.set($lua_name, class)?;
            registry.register_type(super::BindingType {
                name: $lua_name,
                module: $module,
                category: $category,
                description: $description,
                status: super::BindingStatus::Implemented,
                methods: super::registry::standard_handle_methods(),
            })
        }
    };
}
