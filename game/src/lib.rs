//! FWOK 搬箱子示例：Rust 负责玩法，Lua 负责 HUD 与交互界面。
#[macro_use]
extern crate fyrox;
pub mod gameplay;

mod hub;

use crate::gameplay::{GameState, InputState};
use fyrox::{
    core::{algebra::Vector3, pool::Handle, reflect::prelude::*, visitor::prelude::*, warn},
    event::{ElementState, Event, WindowEvent},
    graph::SceneGraph,
    gui::{
        button::ButtonMessage,
        message::{MessageDirection, UiMessage},
        UserInterface,
    },
    keyboard::{KeyCode, PhysicalKey},
    plugin::{
        error::{enable_backtrace_capture, GameResult},
        Plugin, PluginContext,
    },
    scene::{node::Node, Scene},
};
use lua_plugin::SceneCommand;
use lua_plugin::{
    register_fyrox_resources, Bridge, BridgeHandle, LuaConfig, LuaPluginHost, SceneRegistry,
    UiRegistry,
};
use std::{cell::RefCell, path::PathBuf, rc::Rc};

#[derive(Default, Debug, Visit, Reflect)]
#[reflect(type_uuid = "4b7e0f75-4f32-4c02-a58d-3ec89a3a7d11", non_cloneable)]
pub struct Game {
    scene: Handle<Scene>,
    player_node: Handle<Node>,
    box_nodes: Vec<Handle<Node>>,
    state: GameState,
    #[visit(skip)]
    #[reflect(hidden)]
    ui_handle: Handle<UserInterface>,
    #[visit(skip)]
    #[reflect(hidden)]
    ui_registry: Option<UiRegistry>,
    #[visit(skip)]
    #[reflect(hidden)]
    scene_registry: Option<SceneRegistry>,
    #[visit(skip)]
    #[reflect(hidden)]
    ui_font: Option<fyrox::gui::font::FontResource>,
    #[visit(skip)]
    #[reflect(hidden)]
    startup_config: Option<LuaConfig>,
    #[visit(skip)]
    #[reflect(hidden)]
    scene_ready: bool,
    left: bool,
    right: bool,
    up: bool,
    down: bool,
    #[visit(skip)]
    #[reflect(hidden)]
    lua_host: Option<LuaPluginHost>,
    #[visit(skip)]
    #[reflect(hidden)]
    bridge: BridgeHandle,
    #[visit(skip)]
    #[reflect(hidden)]
    lua_update_ticks: u64,
    #[visit(skip)]
    #[reflect(hidden)]
    ui_diagnostic_logged: bool,
}

impl PartialEq for Game {
    fn eq(&self, other: &Self) -> bool {
        self.scene == other.scene
            && self.player_node == other.player_node
            && self.box_nodes == other.box_nodes
            && self.state == other.state
            && self.ui_handle == other.ui_handle
            && self.left == other.left
            && self.right == other.right
            && self.up == other.up
            && self.down == other.down
            && self.scene_ready == other.scene_ready
    }
}

impl Plugin for Game {
    fn register(&self, context: fyrox::plugin::PluginRegistrationContext) -> GameResult {
        warn!("Registering Game plugin.");
        enable_backtrace_capture(true);
        register_fyrox_resources(
            context.resource_manager,
            &context.serialization_context.script_constructors,
        );
        context
            .serialization_context
            .script_constructors
            .add::<hub::Hub>("Hub");
        Ok(())
    }

    fn init(&mut self, scene_path: Option<&str>, mut context: PluginContext) -> GameResult {
        let path = scene_path.unwrap_or("data/scene.rgs").to_owned();
        let mut config = LuaConfig::load("fyrox-lua.toml")
            .map_err(|e| fyrox::plugin::error::GameError::str(e.to_string()))?;
        if config.script_root.as_os_str().is_empty() {
            config.script_root = PathBuf::from("data/scripts");
        }
        self.startup_config = Some(config);
        context.load_ui("data/unnamed.ui", |result, game: &mut Game, ctx| {
            let ui = result?.payload;
            game.ui_handle = ctx.user_interfaces.add(ui);
            game.ui_font = None;
            game.ui_registry = Some(UiRegistry::default());
            game.try_start_runtime(ctx)
        });
        context.load_scene(path, false, |result, game: &mut Game, ctx| {
            game.initialize_scene(result?.payload, ctx)
        });
        Ok(())
    }
    fn update(&mut self, context: &mut PluginContext) -> GameResult {
        let dt = context.dt;
        if self.state.step(
            InputState {
                left: self.left,
                right: self.right,
                up: self.up,
                down: self.down,
            },
            dt,
        ) {
            if let Some(host) = self.lua_host.as_mut() {
                host.dispatch_event("box.goal_reached", "")
                    .map_err(|e| fyrox::plugin::error::GameError::str(e.to_string()))?;
            }
        }
        self.sync_scene(context);

        if !self.ui_diagnostic_logged && self.lua_update_ticks >= 5 {
            if let Ok(ui) = context.user_interfaces.try_get(self.ui_handle) {
                let mut node_count = 0usize;
                let mut visible_nodes = 0usize;
                let mut layout_nodes = 0usize;
                for (_, node) in ui.pair_iter() {
                    node_count += 1;
                    if node.is_globally_visible() {
                        visible_nodes += 1;
                    }
                    if node.visual_valid.get() {
                        layout_nodes += 1;
                    }
                }
                warn!(
                    "[UI] diagnostic: nodes={}, visible={}, visual_valid={}, draw_commands={}, screen={:?}",
                    node_count,
                    visible_nodes,
                    layout_nodes,
                    ui.drawing_context.get_commands().len(),
                    ui.screen_size()
                );
                for (index, command) in ui.drawing_context.get_commands().iter().enumerate() {
                    warn!(
                        "[UI] draw[{}]: bounds={:?}, clip={:?}, triangles={:?}, opacity={}",
                        index,
                        command.bounds,
                        command.clip_bounds,
                        command.triangles,
                        command.opacity
                    );
                }
                self.ui_diagnostic_logged = true;
            }
        }

        if let Ok(mut bridge) = self.bridge.0.try_borrow_mut() {
            if let (Some(registry), Ok(ui)) = (
                self.ui_registry.as_mut(),
                context.user_interfaces.try_get_mut(self.ui_handle),
            ) {
                registry.sync_text_values(ui, &mut bridge.ui_text);
                let commands = std::mem::take(&mut bridge.commands);
                for command in commands {
                    registry.apply(ui, command);
                }
            }
        }

        if let Some(host) = self.lua_host.as_mut() {
            host.update(dt)
                .map_err(|e| fyrox::plugin::error::GameError::str(e.to_string()))?;
            self.lua_update_ticks += 1;
            if self.lua_update_ticks == 1 || self.lua_update_ticks % 300 == 0 {
                // warn!(
                //     "[Lua] game update heartbeat: tick={}, instances={}",
                //     self.lua_update_ticks,
                //     runtime.script_count()
                // );
            }
        }
        self.apply_scene_commands(context);
        Ok(())
    }

    fn on_ui_message(
        &mut self,
        _context: &mut PluginContext,
        message: &UiMessage,
        ui_handle: Handle<UserInterface>,
    ) -> GameResult {
        if message.direction() != MessageDirection::FromWidget {
            return Ok(());
        }
        if ui_handle != self.ui_handle {
            return Ok(());
        }
        let Some(_registry) = self.ui_registry.as_ref() else {
            return Ok(());
        };
        if let Some(ButtonMessage::Click) = message.data() {
            let id = _registry.button_id(message.destination());
            if let Some(id) = id {
                warn!("[Lua] UI click routed: id={}", id);
                if let Some(host) = self.lua_host.as_mut() {
                    host.dispatch_ui_click(id)
                        .map_err(|e| fyrox::plugin::error::GameError::str(e.to_string()))?;
                }
            }
        }
        Ok(())
    }

    fn on_deinit(&mut self, _context: PluginContext) -> GameResult {
        if let Some(host) = self.lua_host.as_mut() {
            host.destroy()
                .map_err(|e| fyrox::plugin::error::GameError::str(e.to_string()))?;
        }
        self.lua_host = None;
        Ok(())
    }

    fn on_os_event(&mut self, event: &Event<()>, _context: PluginContext) -> GameResult {
        if let Event::WindowEvent {
            event: WindowEvent::KeyboardInput { event: input, .. },
            ..
        } = event
        {
            if let PhysicalKey::Code(key) = input.physical_key {
                let pressed = input.state == ElementState::Pressed;
                match key {
                    KeyCode::KeyA | KeyCode::ArrowLeft => self.left = pressed,
                    KeyCode::KeyD | KeyCode::ArrowRight => self.right = pressed,
                    KeyCode::KeyW | KeyCode::ArrowUp => self.up = pressed,
                    KeyCode::KeyS | KeyCode::ArrowDown => self.down = pressed,
                    _ => {}
                }
            }
        }
        Ok(())
    }
}

impl Game {
    fn initialize_scene(&mut self, scene: Scene, context: &mut PluginContext) -> GameResult {
        self.scene = context.scenes.add(scene);
        let loaded_scene = context
            .scenes
            .try_get(self.scene)
            .map_err(|e| fyrox::plugin::error::GameError::str(e.to_string()))?;
        self.player_node = loaded_scene
            .graph
            .find_by_name_from_root("Player")
            .map(|(handle, _)| handle)
            .unwrap_or_default();
        self.box_nodes = ["BoxA", "BoxB"]
            .iter()
            .filter_map(|name| {
                loaded_scene
                    .graph
                    .find_by_name_from_root(name)
                    .map(|(handle, _)| handle)
            })
            .collect();
        self.scene_registry = Some(SceneRegistry::default());
        self.state = GameState::level();
        self.scene_ready = true;
        self.try_start_runtime(context)
    }

    fn try_start_runtime(&mut self, context: &mut PluginContext) -> GameResult {
        if !self.scene_ready
            || !self.ui_handle.is_some()
            || self.lua_host.is_some()
            || self.startup_config.is_none()
        {
            return Ok(());
        }
        let config = self.startup_config.take().unwrap();
        let ui = context
            .user_interfaces
            .try_get_mut(self.ui_handle)
            .map_err(|e| fyrox::plugin::error::GameError::str(e.to_string()))?;
        let font = context
            .resource_manager
            .request::<fyrox::gui::font::Font>(&config.font_path);
        warn!("Loading project UI font: {}", config.font_path.display());
        ui.default_font = font.clone();
        self.ui_font = Some(font);
        let bridge = Rc::new(RefCell::new(Bridge {
            ui_text: Default::default(),
            commands: Vec::new(),
            scene_commands: Vec::new(),
        }));
        self.bridge = BridgeHandle(bridge.clone());
        if config.enabled {
            warn!("[Lua] creating scene runtime");
            let loaded_scene = context
                .scenes
                .try_get(self.scene)
                .map_err(|e| fyrox::plugin::error::GameError::str(e.to_string()))?;
            let mut host = LuaPluginHost::new(config);
            let loaded = host
                .start_scene(loaded_scene, std::path::Path::new("scene"))
                .map_err(|e| fyrox::plugin::error::GameError::str(e.to_string()))?;
            warn!("[Lua] scene components instantiated: {}", loaded);
            warn!(
                "[Lua] lifecycle start complete: instances={}",
                host.runtime().map(|runtime| runtime.script_count()).unwrap_or(0)
            );
            self.bridge = host.bridge().clone();
            self.lua_host = Some(host);
        } else {
            self.lua_host = None;
        }
        self.apply_scene_commands(context);
        Ok(())
    }

    fn sync_scene(&self, context: &mut PluginContext) {
        let Ok(scene) = context.scenes.try_get_mut(self.scene) else {
            return;
        };
        if let Ok(player) = scene.graph.try_get_mut(self.player_node) {
            player.local_transform_mut().set_position(Vector3::new(
                self.state.player.x,
                self.state.player.y,
                0.0,
            ));
        }
        for (handle, x) in self.box_nodes.iter().zip(&self.state.boxes) {
            if let Ok(box_node) = scene.graph.try_get_mut(*handle) {
                box_node
                    .local_transform_mut()
                    .set_position(Vector3::new(x.x, x.y, 0.0));
            }
        }
    }

    fn apply_scene_commands(&mut self, context: &mut PluginContext) {
        let commands = self
            .bridge
            .0
            .try_borrow_mut()
            .ok()
            .map(|mut bridge| std::mem::take(&mut bridge.scene_commands));
        let Some(commands) = commands else { return };
        let Ok(scene) = context.scenes.try_get_mut(self.scene) else {
            return;
        };
        let Some(registry) = self.scene_registry.as_mut() else {
            return;
        };
        for command in commands {
            match command {
                SceneCommand::Resolve(name) => {
                    if registry.resolve(scene, &name).is_some() {
                        warn!("[Lua] scene.find resolved existing node: {}", name);
                    } else {
                        warn!("[Lua] scene.find could not resolve existing node: {}", name);
                    }
                }
                SceneCommand::SetPosition(name, x, y, z) => {
                    if let Some(handle) = registry.resolve(scene, &name) {
                        if let Ok(node) = scene.graph.try_get_mut(handle) {
                            node.local_transform_mut()
                                .set_position(Vector3::new(x, y, z));
                        }
                    }
                }
                SceneCommand::SetRotationZ(name, angle) => {
                    if let Some(handle) = registry.resolve(scene, &name) {
                        if let Ok(node) = scene.graph.try_get_mut(handle) {
                            node.set_rotation_z(angle);
                        }
                    }
                }
                SceneCommand::SetRotationAngles(name, roll, pitch, yaw) => {
                    if let Some(handle) = registry.resolve(scene, &name) {
                        if let Ok(node) = scene.graph.try_get_mut(handle) {
                            node.set_rotation_angles(roll, pitch, yaw);
                        }
                    }
                }
                SceneCommand::SetScale(name, x, y, z) => {
                    if let Some(handle) = registry.resolve(scene, &name) {
                        if let Ok(node) = scene.graph.try_get_mut(handle) {
                            node.set_scale_xyz(x, y, z);
                        }
                    }
                }
                SceneCommand::SetEnabled(name, enabled) => {
                    if let Some(handle) = registry.resolve(scene, &name) {
                        if let Ok(node) = scene.graph.try_get_mut(handle) {
                            node.set_enabled(enabled);
                        }
                    }
                }
            }
        }
    }
}
