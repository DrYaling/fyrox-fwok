//! FWOK runtime host. Gameplay and UI behavior are owned by Lua scripts.
use fyrox::{
    core::{pool::Handle, reflect::prelude::*, visitor::prelude::*, warn},
    event::{ElementState, Event, WindowEvent},
    graph::SceneGraph,
    gui::{
        button::{Button, ButtonMessage},
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
use lua_plugin::{register_fyrox_resources, LuaConfig, LuaPluginHost};
use std::path::{Path, PathBuf};
#[cfg(target_arch = "wasm32")]
use std::rc::Rc;

#[cfg(target_arch = "wasm32")]
fn wasm_lua_source_loader(path: &Path) -> mlua::Result<String> {
    let key = path.to_string_lossy().replace('\\', "/");
    fyrox::core::log::Log::info(format!("[Lua] WASM source request: {key}"));
    let source = match key.as_str() {
        "data/scripts/main.lua" => include_str!("../../data/scripts/main.lua"),
        _ => {
            return Err(mlua::Error::external(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                format!("Lua source is not in the game package: {key}"),
            )))
        }
    };
    Ok(source.to_owned())
}

mod settings;
pub use settings::GameSettings;

#[derive(Default, Debug, Visit, Reflect)]
#[reflect(type_uuid = "4b7e0f75-4f32-4c02-a58d-3ec89a3a7d11", non_cloneable)]
pub struct Game {
    scene: Handle<Scene>,
    #[visit(skip)]
    #[reflect(hidden)]
    ui_handle: Handle<UserInterface>,
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
    lua_update_ticks: u64,
    #[visit(skip)]
    #[reflect(hidden)]
    ui_diagnostic_logged: bool,
    #[visit(skip)]
    #[reflect(hidden)]
    smoke_ui_click_sent: bool,
    #[visit(skip)]
    #[reflect(hidden)]
    #[cfg(feature = "lua-benchmark")]
    benchmark_autostart_sent: bool,
    #[visit(skip)]
    #[reflect(hidden)]
    #[cfg(feature = "lua-benchmark")]
    benchmark_memory_samples: u32,
    #[visit(skip)]
    #[reflect(hidden)]
    #[cfg(feature = "lua-benchmark")]
    stress_started: bool,
    #[visit(skip)]
    #[reflect(hidden)]
    #[cfg(feature = "lua-benchmark")]
    stress_ticks: u64,
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
        let mut config = match load_lua_config() {
            Ok(config) => config,
            Err(error) => {
                warn!("[Lua] configuration error; runtime disabled: {}", error);
                LuaConfig {
                    enabled: false,
                    ..LuaConfig::default()
                }
            }
        };
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

        let ui_load_requests = if let Some(host) = self.lua_host.as_mut() {
            let requests = match host.update_with_context(context, dt) {
                Ok(requests) => requests,
                Err(error) => {
                    warn!("[Lua] update host error recovered: {}", error);
                    Vec::new()
                }
            };
            self.lua_update_ticks += 1;
            if self.lua_update_ticks == 1 || self.lua_update_ticks % 300 == 0 {
                // warn!(
                //     "[Lua] game update heartbeat: tick={}, instances={}",
                //     self.lua_update_ticks,
                //     runtime.script_count()
                // );
            }
            requests
        } else {
            Vec::new()
        };
        self.ui_handle = self
            .lua_host
            .as_ref()
            .map(LuaPluginHost::ui_handle)
            .unwrap_or(Handle::NONE);
        self.schedule_ui_loads(context, ui_load_requests);
        if !self.smoke_ui_click_sent {
            if let Ok(target) = std::env::var("FWOK_TEST_UI_CLICK") {
                if let Ok(ui) = context.user_interfaces.try_get_mut(self.ui_handle) {
                    if let Some((button, node)) = ui.find_by_name_from_root(&target) {
                        if node.cast::<Button>().is_some() {
                            ui.send_message(UiMessage::from_widget(button, ButtonMessage::Click));
                            self.smoke_ui_click_sent = true;
                            warn!("[UI] smoke click injected for serialized button: {target}");
                        }
                    }
                }
            }
        }
        #[cfg(feature = "lua-benchmark")]
        {
            if std::env::var_os("FWOK_MEMORY_AUDIT").is_some() && self.lua_update_ticks % 30 == 0 {
                if let Some(host) = self.lua_host.as_ref() {
                    if let Some(runtime) = host.runtime() {
                        let before = runtime.lua.used_memory();
                        let gc_before = runtime.lua.used_memory();
                        let gc_started = std::time::Instant::now();
                        let gc_result = runtime.lua.gc_collect();
                        let gc_ms = gc_started.elapsed().as_secs_f64() * 1000.0;
                        let after = runtime.lua.used_memory();
                        let stats = host.bridge_stats();
                        self.benchmark_memory_samples += 1;
                        warn!(
                            "[LuaMemory] sample={} tick={} used_before={} gc_before={} used_after={} reclaimed={} gc_ms={:.3} gc_ok={} loaded_scripts={} component_scripts={} bridge_queued={} bridge_applied={} bridge_dropped={}",
                            self.benchmark_memory_samples,
                            self.lua_update_ticks,
                            before,
                            gc_before,
                            after,
                            gc_before.saturating_sub(after),
                            gc_ms,
                            gc_result.is_ok(),
                            runtime.loaded_script_count(),
                            runtime.script_count(),
                            stats.queued_commands,
                            stats.applied_commands,
                            stats.dropped_commands,
                        );
                    }
                }
            }
            if self.ui_diagnostic_logged
                && !self.benchmark_autostart_sent
                && self.lua_update_ticks >= 10
                && std::env::var("FWOK_BENCHMARK_AUTOSTART").ok().as_deref() == Some("1")
            {
                if let Ok(ui) = context.user_interfaces.try_get_mut(self.ui_handle) {
                    if let Some((button, _)) = ui.find_by_name_from_root("lua_benchmark_button") {
                        ui.send_message(UiMessage::from_widget(button, ButtonMessage::Click));
                        self.benchmark_autostart_sent = true;
                        warn!("[Benchmark] injected real UI message for serialized button (automated event, not physical pointer)");
                    } else {
                        if let Some(host) = self.lua_host.as_mut() {
                            if let Some(runtime) = host.runtime_mut() {
                                match runtime
                                    .lua
                                    .load(include_str!("../../data/scripts/lua_benchmark.lua"))
                                    .into_function()
                                    .and_then(|function| function.call::<()>(()))
                                {
                                    Ok(()) => {}
                                    Err(error) => {
                                        warn!(
                                            "[Benchmark] Lua benchmark error recovered: {}",
                                            error
                                        );
                                    }
                                }
                                self.benchmark_autostart_sent = true;
                                warn!("[Benchmark] executed live Lua benchmark script");
                            }
                        }
                    }
                }
            }
            if self.benchmark_autostart_sent
                && self.lua_update_ticks >= 120
                && std::env::var_os("FWOK_BENCHMARK_NO_EXIT").is_none()
            {
                context.loop_controller.exit();
            }
            if std::env::var("FWOK_STRESS_AUTOSTART").ok().as_deref() == Some("1") {
                if !self.stress_started {
                    if let Some(host) = self.lua_host.as_mut() {
                        if let Some(runtime) = host.runtime_mut() {
                            let batch =
                                std::env::var("FWOK_STRESS_BATCH").ok().as_deref() == Some("1");
                            let _ = runtime.lua.globals().set("__fwok_stress_batch", batch);
                            let result = runtime
                                .lua
                                .load(include_str!("../../data/scripts/lua_stress.lua"))
                                .set_name("@lua-plugin/stress")
                                .exec();
                            match result {
                                Ok(()) => {
                                    self.stress_started = true;
                                    warn!("[LuaStress] started");
                                }
                                Err(error) => warn!("[LuaStress] setup error recovered: {}", error),
                            }
                        }
                    }
                }
                if self.stress_started {
                    self.stress_ticks += 1;
                    let stress_tick = self.stress_ticks;
                    if let Some(host) = self.lua_host.as_mut() {
                        let stats_snapshot = if stress_tick % 300 == 0 {
                            Some(host.bridge_stats())
                        } else {
                            None
                        };
                        if let Some(runtime) = host.runtime_mut() {
                            let tick: Result<mlua::Function, _> =
                                runtime.lua.globals().get("__fwok_stress_tick");
                            if let Ok(tick) = tick {
                                if let Err(error) = tick.call::<()>(200u32) {
                                    warn!("[LuaStress] tick error recovered: {}", error);
                                }
                            }
                            if let Ok(event) = runtime
                                .lua
                                .globals()
                                .get::<mlua::Function>("__fwok_stress_event")
                            {
                                let _ = event.call::<()>(("frame", self.stress_ticks.to_string()));
                            }
                            if let Some(stats) = stats_snapshot {
                                let used = runtime.lua.used_memory();
                                warn!("[LuaStress] tick={} lua_heap={} queued={} applied={} dropped={} apply_ns={}", stress_tick, used, stats.queued_commands, stats.applied_commands, stats.dropped_commands, stats.apply_duration_ns);
                            }
                        }
                    }
                }
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
        if let Some(ButtonMessage::Click) = message.data() {
            let id = self
                .lua_host
                .as_ref()
                .and_then(LuaPluginHost::ui_registry)
                .and_then(|registry| registry.button_id(message.destination()))
                .map(str::to_owned);
            if let Some(id) = id {
                warn!("[Lua] UI click routed: id={}", id);
                if let Some(host) = self.lua_host.as_mut() {
                    if let Err(error) = host.dispatch_ui_click(&id) {
                        warn!("[Lua] UI click error recovered: {}", error);
                    }
                }
            }
        }
        Ok(())
    }

    fn on_deinit(&mut self, _context: PluginContext) -> GameResult {
        if let Some(host) = self.lua_host.as_mut() {
            if let Err(error) = host.destroy() {
                warn!("[Lua] destroy error recovered: {}", error);
            }
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
            if let Err(error) = host.dispatch_event("input.key", &format!("{key_name}:{state}")) {
                warn!("[Lua] input event error recovered: {}", error);
            }
        }
        Ok(())
    }
}

fn load_lua_config() -> Result<LuaConfig, Box<dyn std::error::Error>> {
    #[cfg(target_arch = "wasm32")]
    {
        return Ok(toml::from_str(include_str!("../../data/fyrox-lua.toml"))?);
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        LuaConfig::load("data/fyrox-lua.toml")
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
        self.scene_ready = true;
        self.try_start_runtime(context)
    }

    fn try_start_runtime(&mut self, context: &mut PluginContext) -> GameResult {
        if !self.scene_ready || self.lua_host.is_some() || self.startup_config.is_none() {
            return Ok(());
        }
        let Some(config) = self.startup_config.take() else {
            return Ok(());
        };
        if config.enabled {
            warn!("[Lua] creating project script runtime");
            let mut host = LuaPluginHost::new(config);
            host.attach_scene(self.scene);
            // The host owns the bridge consumed by the command host. Pass that
            // exact bridge into Lua so queued UI/Scene mutations are applied.
            #[cfg(target_arch = "wasm32")]
            host.set_source_loader(Rc::new(wasm_lua_source_loader));
            let scene = context
                .scenes
                .try_get(self.scene)
                .map_err(|e| fyrox::plugin::error::GameError::str(e.to_string()))?;
            let loaded = host
                .start_scene(scene, Path::new("scene"))
                .unwrap_or_else(|error| {
                    warn!("[Lua] scene runtime startup error recovered: {}", error);
                    0
                });
            warn!("[Lua] scene Lua scripts instantiated: {}", loaded);
            if host.runtime().is_some() {
                self.lua_host = Some(host);
            } else {
                warn!("[Lua] main.lua is missing; runtime remains disabled");
                self.lua_host = None;
            }
        } else {
            self.lua_host = None;
        }
        Ok(())
    }

    fn schedule_ui_loads(&mut self, context: &mut PluginContext, requests: Vec<String>) {
        for path in requests {
            let path_for_log = path.clone();
            context.load_ui(path, move |result, game: &mut Game, ctx| {
                let Some(host) = game.lua_host.as_mut() else {
                    return Ok(());
                };
                match result {
                    Ok(result) => {
                        host.complete_ui_load(result.payload, ctx, game.ui_font.as_ref());
                        game.ui_handle = host.ui_handle();
                        if !host.ui_visible() {
                            if let Ok(ui) = ctx.user_interfaces.try_get_mut(game.ui_handle) {
                                ui.send(
                                    ui.root(),
                                    fyrox::gui::widget::WidgetMessage::Visibility(false),
                                );
                            }
                        }
                        warn!("[Lua] UI resource loaded by ui.load: {}", path_for_log);
                    }
                    Err(error) => {
                        host.fail_ui_load();
                        warn!(
                            "[Lua] UI resource load failed for {}: {}",
                            path_for_log, error
                        );
                    }
                }
                Ok(())
            });
        }
    }
}
