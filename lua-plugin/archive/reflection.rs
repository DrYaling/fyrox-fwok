use mlua::{Lua, MultiValue, Value};
use std::sync::Arc;
pub trait LuaReflection: Send + Sync + 'static {
    fn call(&self, object: &str, method: &str, args: Vec<String>) -> Result<String, String>;

    /// Typed 调用钩子。实现者可将 Lua 值转换为 Fyrox 类型并执行白名单方法。
    /// 默认回退到旧的字符串协议，便于逐步迁移既有编辑器后端。
    fn invoke(
        &self,
        lua: &Lua,
        object: &str,
        method: &str,
        args: MultiValue,
    ) -> mlua::Result<Value> {
        let values = args
            .into_iter()
            .map(|value| match value {
                Value::String(value) => value
                    .to_str()
                    .map(|s| s.to_owned())
                    .map_err(mlua::Error::external),
                Value::Boolean(value) => Ok(value.to_string()),
                Value::Integer(value) => Ok(value.to_string()),
                Value::Number(value) => Ok(value.to_string()),
                Value::Nil => Ok(String::new()),
                _ => Err(mlua::Error::runtime(
                    "反射参数必须是基础 Lua 值，或由后端实现 invoke",
                )),
            })
            .collect::<mlua::Result<Vec<_>>>()?;
        let result = self
            .call(object, method, values)
            .map_err(mlua::Error::external)?;
        Ok(Value::String(lua.create_string(result)?))
    }

    /// 返回可反射查找的类型名称。默认不开放类型创建。
    fn find_type(&self, _name: &str) -> Option<String> {
        None
    }

    /// 动态字段读取。字段访问必须由后端使用 Fyrox Reflect 执行。
    fn get_field(&self, _object: &str, _field: &str) -> Result<Value, String> {
        Err("当前反射后端未实现字段读取".into())
    }

    /// 动态字段写入。后端应校验字段类型和可写属性。
    fn set_field(&self, _object: &str, _field: &str, _value: Value) -> Result<(), String> {
        Err("当前反射后端未实现字段写入".into())
    }
}
pub fn register<R: LuaReflection>(lua: &Lua, reflection: R) -> mlua::Result<()> {
    let r = Arc::new(reflection);
    let table = lua.create_table()?;
    // 兼容入口：直接调用对象方法。
    let call_backend = r.clone();
    table.set(
        "call",
        lua.create_function(
            move |lua, (object, method, args): (String, String, MultiValue)| {
                call_backend.invoke(lua, &object, &method, args)
            },
        )?,
    )?;

    // tolua 风格：先取得方法，再重复调用，避免每次查找方法描述符。
    let method_backend = r.clone();
    let get_method = lua.create_function(move |lua, (object, method): (String, String)| {
        let backend = method_backend.clone();
        lua.create_function(move |lua, args: MultiValue| {
            backend.invoke(lua, &object, &method, args)
        })
    })?;
    table.set("get_method", get_method.clone())?;
    // tolua 兼容拼写：旧项目通常使用无下划线的方法名。
    table.set("getmethod", get_method)?;

    let type_backend = r.clone();
    let find_type =
        lua.create_function(move |_, name: String| Ok(type_backend.find_type(&name)))?;
    table.set("find_type", find_type.clone())?;
    table.set("findtype", find_type)?;

    let field_backend = r.clone();
    let get_field = lua.create_function(move |_, (object, field): (String, String)| {
        field_backend
            .get_field(&object, &field)
            .map_err(mlua::Error::external)
    })?;
    table.set("get_field", get_field.clone())?;
    table.set("getfield", get_field)?;

    let set_backend = r.clone();
    let set_field =
        lua.create_function(move |_, (object, field, value): (String, String, Value)| {
            set_backend
                .set_field(&object, &field, value)
                .map_err(mlua::Error::external)
        })?;
    table.set("set_field", set_field.clone())?;
    table.set("setfield", set_field)?;

    // 属性是字段的兼容别名；Fyrox Reflect 的属性写入仍由宿主后端负责。
    let property_backend = r.clone();
    let get_property = lua.create_function(move |_, (object, property): (String, String)| {
        property_backend
            .get_field(&object, &property)
            .map_err(mlua::Error::external)
    })?;
    table.set("get_property", get_property.clone())?;
    table.set("getproperty", get_property)?;
    lua.globals().set("reflection", table)
}
