//! Lua 命名空间创建辅助，集中处理 tolua 风格的模块树。

pub(super) fn ensure_fyrox_table(lua: &mlua::Lua) -> mlua::Result<mlua::Table> {
    match lua.globals().get::<mlua::Table>("fyrox") {
        Ok(table) => Ok(table),
        Err(_) => {
            let table = lua.create_table()?;
            lua.globals().set("fyrox", table.clone())?;
            Ok(table)
        }
    }
}

/// 创建点号路径，例如 `scene` 或 `core.math`。
pub(super) fn ensure_path(
    lua: &mlua::Lua,
    root: &mlua::Table,
    path: &str,
) -> mlua::Result<mlua::Table> {
    let mut current = root.clone();
    for part in path.split('.') {
        let next = match current.get::<mlua::Table>(part) {
            Ok(table) => table,
            Err(_) => {
                let table = lua.create_table()?;
                current.set(part, table.clone())?;
                table
            }
        };
        current = next;
    }
    Ok(current)
}
