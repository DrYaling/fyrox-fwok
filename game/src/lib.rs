//! FWOK runtime host. Gameplay and UI behavior are owned by Lua scripts.
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
    scene::Scene,
};
use lua_plugin::{
    register_fyrox_resources, Bridge, BridgeHandle, LuaConfig, LuaPluginHost, SceneRegistry,
    UiRegistry,
};
use lua_plugin::{SceneCommand, UiCommand};
use std::{
    cell::RefCell,
    path::{Path, PathBuf},
    rc::Rc,
};

mod settings;
pub use settings::GameSettings;

const MAX_PENDING_UI_COMMANDS: usize = 20_000;

#[derive(Default, Debug, Visit, Reflect)]
#[reflect(type_uuid = "4b7e0f75-4f32-4c02-a58d-3ec89a3a7d11", non_cloneable)]
pub struct Game {
    scene: Handle<Scene>,
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
    skip_game_font: bool,
    #[visit(skip)]
    #[reflect(hidden)]
    ui_font: Option<fyrox::gui::font::FontResource>,
    #[visit(skip)]
    #[reflect(hidden)]
    startup_config: Option<LuaConfig>,
    #[visit(skip)]
    #[reflect(hidden)]
    scene_ready: bool,
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
    #[visit(skip)]
    #[reflect(hidden)]
    pending_ui_commands: Vec<UiCommand>,
    #[visit(skip)]
    #[reflect(hidden)]
    ui_visible: bool,
    #[visit(skip)]
    #[reflect(hidden)]
    ui_loading: bool,
    #[visit(skip)]
    #[reflect(hidden)]
    #[cfg(feature = "lua-benchmark")]
    benchmark_autostart_sent: bool,
}

impl PartialEq for Game {
    fn eq(&self, other: &Self) -> bool {
        self.scene == other.scene
            && self.ui_handle == other.ui_handle
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
        Ok(())
    }

    fn init(&mut self, scene_path: Option<&str>, mut context: PluginContext) -> GameResult {
        let path = scene_path.unwrap_or("data/scene.rgs").to_owned();
        if !self.skip_game_font {
            let settings = GameSettings::load("data/game_setting.toml")
                .map_err(|e| fyrox::plugin::error::GameError::str(e.to_string()))?;
            if let Some(path) = settings.game_font_path() {
                warn!("Loading game default font: {}", path.display());
                self.ui_font = Some(
                    context
                        .resource_manager
                        .request::<fyrox::gui::font::Font>(&path),
                );
            }
        }
        let mut config = LuaConfig::load("data/fyrox-lua.toml")
            .map_err(|e| fyrox::plugin::error::GameError::str(e.to_string()))?;
        if config.script_root.as_os_str().is_empty() {
            config.script_root = PathBuf::from("data/scripts");
        }
        self.startup_config = Some(config);
        context.load_scene(path, false, |result, game: &mut Game, ctx| {
            game.initialize_scene(result?.payload, ctx)
        });
        Ok(())
    }
    fn update(&mut self, context: &mut PluginContext) -> GameResult {
        let dt = context.dt;
        if !self.ui_diagnostic_logged && self.lua_update_ticks >= 5 {
            if let Ok(ui) = context.user_interfaces.try_get(self.ui_handle) {
                if !ui.drawing_context.get_commands().is_empty() {
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
        }

        self.apply_ui_commands(context);

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
        #[cfg(feature = "lua-benchmark")]
        {
            if self.ui_diagnostic_logged
                && !self.benchmark_autostart_sent
                && self.lua_update_ticks >= 10
                && std::env::var_os("FWOK_BENCHMARK_AUTOSTART").is_some()
            {
                let ui = context
                    .user_interfaces
                    .try_get_mut(self.ui_handle)
                    .map_err(|e| fyrox::plugin::error::GameError::str(e.to_string()))?;
                let (button, _) = ui
                    .find_by_name_from_root("lua_benchmark_button")
                    .ok_or_else(|| {
                        fyrox::plugin::error::GameError::str("benchmark resource button missing")
                    })?;
                ui.send_message(UiMessage::from_widget(button, ButtonMessage::Click));
                self.benchmark_autostart_sent = true;
                warn!("[Benchmark] injected real UI message for serialized button (automated event, not physical pointer)");
            }
            if self.benchmark_autostart_sent && self.lua_update_ticks >= 120 {
                context.loop_controller.exit();
            }
        }
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
        let Event::WindowEvent {
            event: WindowEvent::KeyboardInput { event: input, .. },
            ..
        } = event
        else {
            return Ok(());
        };
        let PhysicalKey::Code(key) = input.physical_key else {
            return Ok(());
        };
        let key_name = match key {
            KeyCode::KeyW => "W",
            KeyCode::KeyA => "A",
            KeyCode::KeyS => "S",
            KeyCode::KeyD => "D",
            KeyCode::ArrowUp => "Up",
            KeyCode::ArrowLeft => "Left",
            KeyCode::ArrowDown => "Down",
            KeyCode::ArrowRight => "Right",
            _ => return Ok(()),
        };
        if let Some(host) = self.lua_host.as_mut() {
            let state = if input.state == ElementState::Pressed {
                "pressed"
            } else {
                "released"
            };
            host.dispatch_event("input.key", &format!("{key_name}:{state}"))
                .map_err(|e| fyrox::plugin::error::GameError::str(e.to_string()))?;
        }
        Ok(())
    }
}

impl Game {
    pub fn for_editor() -> Self {
        Self {
            skip_game_font: true,
            ..Default::default()
        }
    }

    fn initialize_scene(&mut self, scene: Scene, context: &mut PluginContext) -> GameResult {
        self.scene = context.scenes.add(scene);
        self.scene_registry = Some(SceneRegistry::default());
        self.scene_ready = true;
        self.try_start_runtime(context)
    }

    fn try_start_runtime(&mut self, context: &mut PluginContext) -> GameResult {
        if !self.scene_ready || self.lua_host.is_some() || self.startup_config.is_none() {
            return Ok(());
        }
        let config = self.startup_config.take().unwrap();
        let bridge = Rc::new(RefCell::new(Bridge {
            ui_text: Default::default(),
            commands: Vec::new(),
            scene_commands: Vec::new(),
        }));
        self.bridge = BridgeHandle(bridge.clone());
        if config.enabled {
            warn!("[Lua] creating project script runtime");
            let mut host = LuaPluginHost::new(config);
            let scene = context
                .scenes
                .try_get(self.scene)
                .map_err(|e| fyrox::plugin::error::GameError::str(e.to_string()))?;
            let loaded = host
                .start_scene(scene, Path::new("scene"))
                .map_err(|e| fyrox::plugin::error::GameError::str(e.to_string()))?;
            warn!("[Lua] scene Lua scripts instantiated: {}", loaded);
            if host.runtime().is_some() {
                self.bridge = host.bridge().clone();
                self.lua_host = Some(host);
            } else {
                warn!("[Lua] main.lua is missing; runtime remains disabled");
                self.lua_host = None;
            }
        } else {
            self.lua_host = None;
        }
        self.apply_scene_commands(context);
        Ok(())
    }

    fn apply_ui_commands(&mut self, context: &mut PluginContext) {
        let commands = self
            .bridge
            .0
            .try_borrow_mut()
            .ok()
            .map(|mut bridge| std::mem::take(&mut bridge.commands))
            .unwrap_or_default();
        let mut ordered = std::mem::take(&mut self.pending_ui_commands);
        ordered.extend(commands);

        let mut deferred = Vec::new();
        for command in ordered {
            match command {
                UiCommand::Load(path) => {
                    if self.ui_handle.is_some() || self.ui_loading {
                        warn!(
                            "[Lua] ui.load ignored because a UI is already loaded or loading: {}",
                            path
                        );
                        continue;
                    }
                    self.ui_loading = true;
                    let path_for_log = path.clone();
                    context.load_ui(path, move |result, game: &mut Game, ctx| {
                        game.ui_loading = false;
                        let mut ui = result?.payload;
                        if let Some(font) = game.ui_font.as_ref() {
                            ui.default_font = font.clone();
                        }
                        game.ui_handle = ctx.user_interfaces.add(ui);
                        game.ui_registry = Some(UiRegistry::default());
                        if !game.ui_visible {
                            if let Ok(ui) = ctx.user_interfaces.try_get_mut(game.ui_handle) {
                                ui.send(
                                    ui.root(),
                                    fyrox::gui::widget::WidgetMessage::Visibility(false),
                                );
                            }
                        }
                        warn!("[Lua] UI resource loaded by ui.load: {}", path_for_log);
                        Ok(())
                    });
                }
                UiCommand::Show(visible) => {
                    self.ui_visible = visible;
                    if let Ok(ui) = context.user_interfaces.try_get_mut(self.ui_handle) {
                        ui.send(
                            ui.root(),
                            fyrox::gui::widget::WidgetMessage::Visibility(visible),
                        );
                    }
                }
                command => deferred.push(command),
            }
        }

        if let (Some(registry), Ok(ui)) = (
            self.ui_registry.as_mut(),
            context.user_interfaces.try_get_mut(self.ui_handle),
        ) {
            if let Ok(mut bridge) = self.bridge.0.try_borrow_mut() {
                registry.sync_text_values(ui, &mut bridge.ui_text);
            }
            for command in deferred {
                registry.apply(ui, command);
            }
        } else {
            let available = MAX_PENDING_UI_COMMANDS.saturating_sub(self.pending_ui_commands.len());
            let dropped = deferred.len().saturating_sub(available);
            self.pending_ui_commands
                .extend(deferred.into_iter().take(available));
            if dropped > 0 && (self.lua_update_ticks == 0 || self.lua_update_ticks % 300 == 0) {
                warn!("[Lua] dropped {dropped} UI command(s) while waiting for UI to load");
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
                SceneCommand::Resolve(target) => {
                    if registry
                        .resolve_scoped(
                            scene,
                            target.scope.map(|token| token.to_handle()),
                            &target.name,
                        )
                        .is_some()
                    {
                        warn!(
                            "[Lua] scene.find resolved existing node: scope={:?}, name={}",
                            target.scope, target.name
                        );
                    } else {
                        warn!(
                            "[Lua] scene.find could not resolve existing node: scope={:?}, name={}",
                            target.scope, target.name
                        );
                    }
                }
                SceneCommand::SetPosition(target, x, y, z) => {
                    if let Some(handle) = registry.resolve_scoped(
                        scene,
                        target.scope.map(|token| token.to_handle()),
                        &target.name,
                    ) {
                        if let Ok(node) = scene.graph.try_get_mut(handle) {
                            node.local_transform_mut()
                                .set_position(Vector3::new(x, y, z));
                        }
                    }
                }
                SceneCommand::SetRotationZ(target, angle) => {
                    if let Some(handle) = registry.resolve_scoped(
                        scene,
                        target.scope.map(|token| token.to_handle()),
                        &target.name,
                    ) {
                        if let Ok(node) = scene.graph.try_get_mut(handle) {
                            node.set_rotation_z(angle);
                        }
                    }
                }
                SceneCommand::SetRotationAngles(target, roll, pitch, yaw) => {
                    if let Some(handle) = registry.resolve_scoped(
                        scene,
                        target.scope.map(|token| token.to_handle()),
                        &target.name,
                    ) {
                        if let Ok(node) = scene.graph.try_get_mut(handle) {
                            node.set_rotation_angles(roll, pitch, yaw);
                        }
                    }
                }
                SceneCommand::SetScale(target, x, y, z) => {
                    if let Some(handle) = registry.resolve_scoped(
                        scene,
                        target.scope.map(|token| token.to_handle()),
                        &target.name,
                    ) {
                        if let Ok(node) = scene.graph.try_get_mut(handle) {
                            node.set_scale_xyz(x, y, z);
                        }
                    }
                }
                SceneCommand::SetEnabled(target, enabled) => {
                    if let Some(handle) = registry.resolve_scoped(
                        scene,
                        target.scope.map(|token| token.to_handle()),
                        &target.name,
                    ) {
                        if let Ok(node) = scene.graph.try_get_mut(handle) {
                            node.set_enabled(enabled);
                        }
                    }
                }
            }
        }
    }
}
