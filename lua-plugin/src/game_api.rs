use mlua::{Lua, UserData, UserDataMethods};
use std::cell::{Ref, RefCell, RefMut};
use std::collections::HashMap;
use std::rc::Rc;

#[derive(Default, Debug)]
pub struct Bridge {
    pub ui_text: HashMap<String, String>,
    pub commands: Vec<UiCommand>,
    pub scene_commands: Vec<SceneCommand>,
}

#[derive(Debug, Clone)]
pub enum SceneCommand {
    Resolve(String),
    SetPosition(String, f32, f32, f32),
    SetRotationZ(String, f32),
    SetRotationAngles(String, f32, f32, f32),
    SetScale(String, f32, f32, f32),
    SetEnabled(String, bool),
}

#[derive(Debug, Clone, Copy)]
pub enum LuaLogLevel {
    Info,
    Warn,
    Error,
}

#[derive(Debug)]
pub enum UiCommand {
    Resolve(String),
    Create(UiElementSpec),
    SetText(String, String),
    Append(String, String),
    SetVisible(String, bool),
    SetEnabled(String, bool),
    SetWidth(String, f32),
    SetHeight(String, f32),
    SetPosition(String, f32, f32),
    Log(LuaLogLevel, String),
}

#[derive(Debug)]
pub enum UiElementKind {
    Text,
    TextBox,
    Button,
}

#[derive(Debug)]
pub struct UiElementSpec {
    pub kind: UiElementKind,
    pub id: String,
    pub text: String,
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
}

pub type BridgeRef = Rc<RefCell<Bridge>>;

#[derive(Clone)]
pub struct BridgeHandle(pub BridgeRef);

impl Default for BridgeHandle {
    fn default() -> Self {
        Self(Rc::new(RefCell::new(Bridge::default())))
    }
}

impl std::fmt::Debug for BridgeHandle {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("BridgeHandle")
    }
}
impl PartialEq for BridgeHandle {
    fn eq(&self, o: &Self) -> bool {
        Rc::ptr_eq(&self.0, &o.0)
    }
}

fn borrow_mut(b: &BridgeRef) -> mlua::Result<RefMut<'_, Bridge>> {
    b.try_borrow_mut()
        .map_err(|_| mlua::Error::runtime("Lua host context is already mutably borrowed"))
}

fn borrow(b: &BridgeRef) -> mlua::Result<Ref<'_, Bridge>> {
    b.try_borrow()
        .map_err(|_| mlua::Error::runtime("Lua host context is already mutably borrowed"))
}

#[derive(Clone)]
struct UiComponentRef {
    id: String,
    bridge: BridgeRef,
}

#[derive(Clone)]
struct SceneNodeRef {
    name: String,
    bridge: BridgeRef,
}

impl UserData for SceneNodeRef {
    fn add_methods<M: UserDataMethods<Self>>(m: &mut M) {
        m.add_method("set_position", |_, this, (x, y, z): (f32, f32, f32)| {
            borrow_mut(&this.bridge)?
                .scene_commands
                .push(SceneCommand::SetPosition(this.name.clone(), x, y, z));
            Ok(())
        });
        m.add_method("set_rotation_z", |_, this, angle: f32| {
            borrow_mut(&this.bridge)?
                .scene_commands
                .push(SceneCommand::SetRotationZ(this.name.clone(), angle));
            Ok(())
        });
        m.add_method(
            "set_rotation",
            |_, this, (roll, pitch, yaw): (f32, f32, f32)| {
                borrow_mut(&this.bridge)?
                    .scene_commands
                    .push(SceneCommand::SetRotationAngles(
                        this.name.clone(),
                        roll,
                        pitch,
                        yaw,
                    ));
                Ok(())
            },
        );
        m.add_method("set_scale", |_, this, (x, y, z): (f32, f32, f32)| {
            borrow_mut(&this.bridge)?
                .scene_commands
                .push(SceneCommand::SetScale(this.name.clone(), x, y, z));
            Ok(())
        });
        m.add_method("set_enabled", |_, this, value: bool| {
            borrow_mut(&this.bridge)?
                .scene_commands
                .push(SceneCommand::SetEnabled(this.name.clone(), value));
            Ok(())
        });
    }
}
impl UserData for UiComponentRef {
    fn add_methods<M: UserDataMethods<Self>>(m: &mut M) {
        m.add_method("set_text", |_, this, value: String| {
            borrow_mut(&this.bridge)?
                .commands
                .push(UiCommand::SetText(this.id.clone(), value));
            Ok(())
        });
        m.add_method("append", |_, this, value: String| {
            borrow_mut(&this.bridge)?
                .commands
                .push(UiCommand::Append(this.id.clone(), value));
            Ok(())
        });
        m.add_method("set_visible", |_, this, value: bool| {
            borrow_mut(&this.bridge)?
                .commands
                .push(UiCommand::SetVisible(this.id.clone(), value));
            Ok(())
        });
        m.add_method("set_enabled", |_, this, value: bool| {
            borrow_mut(&this.bridge)?
                .commands
                .push(UiCommand::SetEnabled(this.id.clone(), value));
            Ok(())
        });
        m.add_method("set_width", |_, this, value: f32| {
            borrow_mut(&this.bridge)?
                .commands
                .push(UiCommand::SetWidth(this.id.clone(), value));
            Ok(())
        });
        m.add_method("set_height", |_, this, value: f32| {
            borrow_mut(&this.bridge)?
                .commands
                .push(UiCommand::SetHeight(this.id.clone(), value));
            Ok(())
        });
        m.add_method("set_position", |_, this, (x, y): (f32, f32)| {
            borrow_mut(&this.bridge)?
                .commands
                .push(UiCommand::SetPosition(this.id.clone(), x, y));
            Ok(())
        });
        m.add_method("text", |_, this, ()| {
            Ok(borrow(&this.bridge)?
                .ui_text
                .get(&this.id)
                .cloned()
                .unwrap_or_default())
        });
        m.add_method("on_click", |lua, this, callback: mlua::Function| {
            let t: mlua::Table = match lua.globals().get("__fwok_ui_clicks") {
                Ok(table) => table,
                Err(_) => {
                    let table = lua.create_table()?;
                    lua.globals().set("__fwok_ui_clicks", table.clone())?;
                    table
                }
            };
            t.set(this.id.as_str(), callback)
        });
    }
}

pub struct Api {
    pub bridge: BridgeRef,
}
impl crate::LuaGameApi for Api {
    fn register(&self, lua: &Lua) -> mlua::Result<()> {
        let log = lua.create_table()?;
        for (name, level) in [
            ("info", LuaLogLevel::Info),
            ("warn", LuaLogLevel::Warn),
            ("error", LuaLogLevel::Error),
        ] {
            let bridge = self.bridge.clone();
            log.set(
                name,
                lua.create_function(move |_, msg: String| {
                    borrow_mut(&bridge)?
                        .commands
                        .push(UiCommand::Log(level, msg));
                    Ok(())
                })?,
            )?;
        }
        lua.globals().set("log", log)?;
        let ui = lua.create_table()?;
        let bridge = self.bridge.clone();
        let component = lua.create_function(move |lua, id: String| {
            let cache: mlua::Table = match lua.globals().get("__fwok_ui_components") {
                Ok(table) => table,
                Err(_) => {
                    let table = lua.create_table()?;
                    lua.globals().set("__fwok_ui_components", table.clone())?;
                    table
                }
            };
            if let Ok(existing) = cache.get::<mlua::AnyUserData>(id.as_str()) {
                return Ok(existing);
            }
            let proxy = lua.create_userdata(UiComponentRef {
                id: id.clone(),
                bridge: bridge.clone(),
            })?;
            borrow_mut(&bridge)?
                .commands
                .push(UiCommand::Resolve(id.clone()));
            cache.set(id, proxy.clone())?;
            Ok(proxy)
        })?;
        ui.set("component", component.clone())?;
        ui.set("find", component.clone())?;
        ui.set("register", component)?;
        for (name, kind) in [
            ("create_text", UiElementKind::Text),
            ("create_text_box", UiElementKind::TextBox),
            ("create_button", UiElementKind::Button),
        ] {
            let bridge = self.bridge.clone();
            ui.set(name, lua.create_function(move |_, (id, text, x, y, width, height): (String, String, f32, f32, f32, f32)| {
                let kind = match kind { UiElementKind::Text => UiElementKind::Text, UiElementKind::TextBox => UiElementKind::TextBox, UiElementKind::Button => UiElementKind::Button };
                borrow_mut(&bridge)?.commands.push(UiCommand::Create(UiElementSpec { kind, id, text, x, y, width, height }));
                Ok(())
            })?)?;
        }
        lua.globals().set("ui", ui)?;
        let scene = lua.create_table()?;
        let bridge = self.bridge.clone();
        scene.set(
            "find",
            lua.create_function(move |lua, name: String| {
                let cache: mlua::Table = match lua.globals().get("__fwok_scene_nodes") {
                    Ok(table) => table,
                    Err(_) => {
                        let table = lua.create_table()?;
                        lua.globals().set("__fwok_scene_nodes", table.clone())?;
                        table
                    }
                };
                if let Ok(existing) = cache.get::<mlua::AnyUserData>(name.as_str()) {
                    return Ok(existing);
                }
                let proxy = lua.create_userdata(SceneNodeRef {
                    name: name.clone(),
                    bridge: bridge.clone(),
                })?;
                borrow_mut(&bridge)?
                    .scene_commands
                    .push(SceneCommand::Resolve(name.clone()));
                cache.set(name, proxy.clone())?;
                Ok(proxy)
            })?,
        )?;
        lua.globals().set("scene", scene)?;
        Ok(())
    }
}
