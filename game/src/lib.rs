//! FWOK 平台跳跃示例：Rust 游戏状态 + Lua UI/交互脚本。
use fyrox::{
    core::{algebra::Vector2, pool::Handle, reflect::prelude::*, visitor::prelude::*},
    event::{ElementState, Event, WindowEvent},
    graph::SceneGraph,
    gui::{
        button::{Button, ButtonBuilder, ButtonMessage},
        message::{MessageDirection, UiMessage},
        text::{Text, TextBuilder, TextMessage},
        text_box::{TextBox, TextBoxBuilder},
        widget::{WidgetBuilder, WidgetMessage},
        UserInterface,
    },
    plugin::{error::GameResult, Plugin, PluginContext},
};
use lua_binding::{EventManager, LuaConfig, LuaGameApi, LuaRuntime};
use mlua::Lua;
use std::{
    path::PathBuf,
    sync::{Arc, Mutex},
};
use winit::keyboard::{KeyCode, PhysicalKey};
#[derive(Default, Debug, PartialEq, Visit, Reflect)]
#[reflect(type_uuid = "4b7e0f75-4f32-4c02-a58d-3ec89a3a7d11", non_cloneable)]
pub struct Game {
    #[visit(skip)]
    #[reflect(hidden)]
    position: Vector2<f32>,
    velocity: Vector2<f32>,
    grounded: bool,
    inventory: Vec<String>,
    hud: Handle<Text>,
    inventory_label: Handle<Text>,
    chat_log: Handle<Text>,
    chat_input: Handle<TextBox>,
    inventory_button: Handle<Button>,
    send_button: Handle<Button>,
    left: bool,
    right: bool,
    jump: bool,
    #[visit(skip)]
    #[reflect(hidden)]
    runtime: Option<LuaRuntime>,
    #[visit(skip)]
    #[reflect(hidden)]
    bridge: BridgeHandle,
}
#[derive(Default, Debug, PartialEq)]
struct Bridge {
    position: Vector2<f32>,
    inventory: Vec<String>,
    commands: Vec<Command>,
}
#[derive(Debug, Clone, PartialEq)]
enum Command {
    Text(String, String),
    Show(String, bool),
    Chat(String),
}
#[derive(Clone)]
struct BridgeHandle(Arc<Mutex<Bridge>>);
impl Default for BridgeHandle {
    fn default() -> Self {
        Self(Arc::new(Mutex::new(Bridge::default())))
    }
}
impl std::fmt::Debug for BridgeHandle {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("BridgeHandle")
    }
}
impl PartialEq for BridgeHandle {
    fn eq(&self, o: &Self) -> bool {
        Arc::ptr_eq(&self.0, &o.0)
    }
}
struct Api {
    bridge: Arc<Mutex<Bridge>>,
}
impl LuaGameApi for Api {
    fn register(&self, lua: &Lua, events: EventManager) -> mlua::Result<()> {
        let game = lua.create_table()?;
        let b = self.bridge.clone();
        game.set(
            "position",
            lua.create_function(move |_, ()| {
                let b = b.lock().unwrap();
                Ok((b.position.x, b.position.y))
            })?,
        )?;
        let b = self.bridge.clone();
        game.set(
            "add_item",
            lua.create_function(move |_, i: String| {
                b.lock().unwrap().inventory.push(i);
                Ok(())
            })?,
        )?;
        lua.globals().set("game", game)?;
        let ui = lua.create_table()?;
        let b = self.bridge.clone();
        ui.set(
            "set_text",
            lua.create_function(move |_, (i, t): (String, String)| {
                b.lock().unwrap().commands.push(Command::Text(i, t));
                Ok(())
            })?,
        )?;
        let b = self.bridge.clone();
        ui.set(
            "show",
            lua.create_function(move |_, (i, v): (String, bool)| {
                b.lock().unwrap().commands.push(Command::Show(i, v));
                Ok(())
            })?,
        )?;
        lua.globals().set("ui", ui)?;
        let chat = lua.create_table()?;
        let b = self.bridge.clone();
        chat.set(
            "append",
            lua.create_function(move |_, t: String| {
                b.lock().unwrap().commands.push(Command::Chat(t));
                Ok(())
            })?,
        )?;
        lua.globals().set("chat", chat)?;
        let inv = lua.create_table()?;
        let b = self.bridge.clone();
        inv.set(
            "items",
            lua.create_function(move |l, ()| {
                l.create_sequence_from(b.lock().unwrap().inventory.clone())
            })?,
        )?;
        lua.globals().set("inventory", inv)?;
        let e = events.clone();
        lua.globals().set(
            "events",
            lua.create_table_from([(
                String::from("emit"),
                lua.create_function(move |_, (n, p): (String, String)| {
                    e.emit(n, p);
                    Ok(())
                })?,
            )])?,
        )?;
        Ok(())
    }
    fn register_reflection(&self, lua: &Lua, events: EventManager) -> mlua::Result<()> {
        // 游戏只提供小型稳定反射适配器；编辑器不会生成完整 Fyrox 类型绑定。
        self.register(lua, events)
    }
}
impl Plugin for Game {
    fn init(&mut self, _: Option<&str>, c: PluginContext) -> GameResult {
        c.user_interfaces
            .add(UserInterface::new(Vector2::new(1280.0, 720.0)));
        let u = &mut c.user_interfaces.first_mut().build_ctx();
        self.hud = TextBuilder::new(
            WidgetBuilder::new()
                .with_width(600.0)
                .with_height(100.0)
                .with_desired_position(Vector2::new(20.0, 20.0)),
        )
        .with_text("FWOK PLATFORMER")
        .build(u);
        self.inventory_label = TextBuilder::new(
            WidgetBuilder::new()
                .with_width(420.0)
                .with_height(60.0)
                .with_desired_position(Vector2::new(20.0, 130.0)),
        )
        .with_text("背包")
        .build(u);
        self.inventory_button = ButtonBuilder::new(
            WidgetBuilder::new()
                .with_width(180.0)
                .with_height(36.0)
                .with_desired_position(Vector2::new(20.0, 200.0)),
        )
        .with_text("切换背包")
        .build(u);
        self.chat_log = TextBuilder::new(
            WidgetBuilder::new()
                .with_width(520.0)
                .with_height(180.0)
                .with_desired_position(Vector2::new(680.0, 80.0)),
        )
        .with_text("聊天窗口")
        .build(u);
        self.chat_input = TextBoxBuilder::new(
            WidgetBuilder::new()
                .with_width(400.0)
                .with_height(36.0)
                .with_desired_position(Vector2::new(680.0, 280.0)),
        )
        .with_text("")
        .build(u);
        self.send_button = ButtonBuilder::new(
            WidgetBuilder::new()
                .with_width(100.0)
                .with_height(36.0)
                .with_desired_position(Vector2::new(1090.0, 280.0)),
        )
        .with_text("发送")
        .build(u);
        self.inventory = vec!["红宝石".into(), "小药水".into()];
        self.bridge = BridgeHandle(Arc::new(Mutex::new(Bridge {
            position: self.position,
            inventory: self.inventory.clone(),
            commands: vec![],
        })));
        let mut cfg = LuaConfig::default();
        cfg.script_root = PathBuf::from("data/scripts");
        self.runtime = Some(
            LuaRuntime::new(
                cfg,
                Api {
                    bridge: self.bridge.0.clone(),
                },
            )
            .map_err(|e| fyrox::plugin::error::GameError::str(e.to_string()))?,
        );
        self.runtime
            .as_ref()
            .unwrap()
            .start()
            .map_err(|e| fyrox::plugin::error::GameError::str(e.to_string()))?;
        Ok(())
    }
    fn update(&mut self, c: &mut PluginContext) -> GameResult {
        let dt = c.dt as f32;
        self.velocity.x = if self.left {
            -220.0
        } else if self.right {
            220.0
        } else {
            0.0
        };
        if self.jump && self.grounded {
            self.velocity.y = 420.0;
            self.grounded = false
        }
        self.velocity.y -= 980.0 * dt;
        self.position += self.velocity * dt;
        if self.position.y <= 0.0 {
            self.position.y = 0.0;
            self.velocity.y = 0.0;
            self.grounded = true
        }
        if let Ok(mut b) = self.bridge.0.lock() {
            b.position = self.position;
            self.inventory = b.inventory.clone();
            for x in std::mem::take(&mut b.commands) {
                match x {
                    Command::Text(i, t) => {
                        let h = match i.as_str() {
                            "hud" => self.hud,
                            "inventory_label" => self.inventory_label,
                            "chat_log" => self.chat_log,
                            _ => Handle::NONE,
                        };
                        if h != Handle::<Text>::NONE {
                            c.user_interfaces.first().send(h, TextMessage::Text(t));
                        }
                    }
                    Command::Show(i, v) => {
                        if i == "inventory_label" {
                            c.user_interfaces
                                .first()
                                .send(self.inventory_label, WidgetMessage::Visibility(v));
                        }
                    }
                    Command::Chat(t) => c
                        .user_interfaces
                        .first()
                        .send(self.chat_log, TextMessage::Text(t)),
                }
            }
        }
        if let Some(r) = &self.runtime {
            r.dispatch_events()
                .map_err(|e| fyrox::plugin::error::GameError::str(e.to_string()))?;
            r.call_all("update", dt)
                .map_err(|e| fyrox::plugin::error::GameError::str(e.to_string()))?;
        }
        self.jump = false;
        Ok(())
    }
    fn on_ui_message(
        &mut self,
        c: &mut PluginContext,
        m: &UiMessage,
        _: Handle<UserInterface>,
    ) -> GameResult {
        if m.direction() != MessageDirection::FromWidget {
            return Ok(());
        }
        if let Some(ButtonMessage::Click) = m.data() {
            if m.destination() == self.inventory_button {
                if let Some(r) = &self.runtime {
                    r.events().emit("inventory.toggle", "")
                }
            } else if m.destination() == self.send_button {
                if let Ok(t) = c.user_interfaces.first().try_get(self.chat_input) {
                    if let Some(r) = &self.runtime {
                        r.events().emit("chat.send", t.text())
                    }
                    c.user_interfaces
                        .first()
                        .send(self.chat_input, TextMessage::Text(String::new()));
                }
            }
        }
        Ok(())
    }
    fn on_deinit(&mut self, _: PluginContext) -> GameResult {
        if let Some(r) = &self.runtime {
            r.destroy()
                .map_err(|e| fyrox::plugin::error::GameError::str(e.to_string()))?
        }
        Ok(())
    }
    fn on_os_event(&mut self, e: &Event<()>, _: PluginContext) -> GameResult {
        if let Event::WindowEvent {
            event: WindowEvent::KeyboardInput { event: i, .. },
            ..
        } = e
        {
            if let PhysicalKey::Code(k) = i.physical_key {
                let p = i.state == ElementState::Pressed;
                match k {
                    KeyCode::KeyA | KeyCode::ArrowLeft => self.left = p,
                    KeyCode::KeyD | KeyCode::ArrowRight => self.right = p,
                    KeyCode::Space if p => self.jump = true,
                    _ => {}
                }
            }
        }
        Ok(())
    }
}
