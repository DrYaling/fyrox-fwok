use crate::{
    bindings::{register_engine_bindings, register_generated_component_aliases, BindingRegistry},
    config::{BindingMode, LuaConfig},
    resource::{EditorScript, LuaComponent},
    script_files::collect_lua_files,
    LuaGameApi,
};
use mlua::{Function, Lua, RegistryKey, Table};
use std::{
    fs,
    path::{Path, PathBuf},
    rc::Rc,
    thread::ThreadId,
};

macro_rules! lua_info {
    ($($arg:tt)*) => {
        fyrox::core::log::Log::info(format!($($arg)*));
    };
}
macro_rules! lua_warn {
    ($($arg:tt)*) => {
        fyrox::core::log::Log::warn(format!($($arg)*));
    };
}

pub struct LuaRuntime {
    pub lua: Lua,
    scripts: Vec<ScriptInstance>,
    config: LuaConfig,
    bindings: BindingRegistry,
    update_ticks: u64,
    events_dispatched: u64,
    owner_thread: ThreadId,
    // Rc is intentionally !Send + !Sync: a LuaRuntime belongs to one Fyrox thread.
    _main_thread_only: Rc<()>,
}
struct ScriptInstance {
    path: PathBuf,
    instance: RegistryKey,
    awake: Option<RegistryKey>,
    start: Option<RegistryKey>,
    update: Option<RegistryKey>,
    event: Option<RegistryKey>,
    destroy: Option<RegistryKey>,
}
impl std::fmt::Debug for LuaRuntime {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("LuaRuntime")
            .field("scripts", &self.scripts.len())
            .finish()
    }
}
impl PartialEq for LuaRuntime {
    fn eq(&self, o: &Self) -> bool {
        std::ptr::eq(self, o)
    }
}
impl LuaRuntime {
    pub fn new(config: LuaConfig, api: impl LuaGameApi) -> mlua::Result<Self> {
        Self::new_inner(config, api, true)
    }

    /// Creates a runtime for scene-owned LuaComponent instances without scanning a global
    /// script directory. Scene loading then has one authoritative source of script instances.
    pub fn new_for_scene(config: LuaConfig, api: impl LuaGameApi) -> mlua::Result<Self> {
        Self::new_inner(config, api, false)
    }

    fn new_inner(config: LuaConfig, api: impl LuaGameApi, load_root: bool) -> mlua::Result<Self> {
        let root = config.script_root.clone();
        lua_info!(
            "[Lua] runtime initializing (root={}, jit={}, reload={:?}, binding={:?})",
            root.display(),
            config.jit,
            config.reload_mode,
            config.effective_binding_mode()
        );
        let lua = Lua::new();
        // 让场景脚本可以通过标准 require 互相组合；根目录由配置决定。
        if let Ok(package) = lua.globals().get::<Table>("package") {
            let path = format!("{}/?.lua;{}/?/init.lua", root.display(), root.display());
            package.set("path", path)?;
        }
        // 引擎绑定只从手写注册入口装载；这里绝不扫描 Lua 源码。
        let bindings = register_engine_bindings(&lua)?;
        lua_info!(
            "[Lua] API registration complete (mode={:?}, catalog_bindings={})",
            config.binding_mode,
            bindings.len()
        );
        api.register(&lua)?;
        register_generated_component_aliases(&lua)?;
        let mut runtime = Self {
            lua,
            scripts: vec![],
            config,
            bindings,
            update_ticks: 0,
            events_dispatched: 0,
            owner_thread: std::thread::current().id(),
            _main_thread_only: Rc::new(()),
        };
        if load_root {
            runtime.load_root(&root)?;
        }
        lua_info!(
            "[Lua] runtime initialized with {} script(s)",
            runtime.scripts.len()
        );
        Ok(runtime)
    }
    pub fn binding_mode(&self) -> BindingMode {
        self.assert_owner_thread();
        self.config.effective_binding_mode()
    }
    pub fn config(&self) -> &LuaConfig {
        self.assert_owner_thread();
        &self.config
    }

    /// 返回本 VM 已注册的静态引擎绑定，用于编辑器检查和调试面板。
    pub fn bindings(&self) -> &BindingRegistry {
        self.assert_owner_thread();
        &self.bindings
    }
    #[inline]
    fn assert_owner_thread(&self) {
        assert_eq!(
            self.owner_thread,
            std::thread::current().id(),
            "LuaRuntime used from a non-owner thread"
        );
    }
    pub fn reload_script(&mut self, path: &Path) -> mlua::Result<()> {
        self.assert_owner_thread();
        if let Some(i) = self.scripts.iter().position(|entry| entry.path == path) {
            let old = self.scripts.remove(i);
            self.call_no_arg(&old, old.destroy.as_ref())?;
            self.remove_instance_keys(old)?;
        }
        self.load_script(path)
    }
    pub fn load_root(&mut self, root: &Path) -> mlua::Result<()> {
        self.assert_owner_thread();
        let mut files = vec![];
        collect_lua_files(root, &mut files);
        lua_info!(
            "[Lua] script root '{}': discovered {} file(s)",
            root.display(),
            files.len()
        );
        if files.is_empty() {
            lua_warn!("Configured Lua script root is empty: {}", root.display());
        }
        for p in files {
            // 目录中的可复用模块由 Lua `require` 按需加载，不作为场景组件实例化。
            if p.components().any(|c| c.as_os_str() == "modules") {
                continue;
            }
            self.load_script(&p)?;
            lua_info!("[Lua] script loaded: {}", p.display());
        }
        Ok(())
    }
    pub fn load_script(&mut self, p: &Path) -> mlua::Result<()> {
        self.assert_owner_thread();
        let source = fs::read_to_string(p).map_err(mlua::Error::external)?;
        self.load_script_source(p, &source, &EditorScript::default())
    }

    /// 从场景 LuaComponent 加载脚本。资源脚本和目录脚本共用同一生命周期实现。
    pub fn load_script_source(
        &mut self,
        id: &Path,
        source: &str,
        editor_script: &EditorScript,
    ) -> mlua::Result<()> {
        self.assert_owner_thread();
        self.load_script_source_with_components(id, source, editor_script, &Default::default())
    }

    fn load_script_source_with_components(
        &mut self,
        id: &Path,
        source: &str,
        editor_script: &EditorScript,
        component_bindings: &crate::component::ComponentBindings,
    ) -> mlua::Result<()> {
        if !editor_script.enabled {
            return Ok(());
        }
        let class: Table = self
            .lua
            .load(source)
            .set_name(id.to_string_lossy())
            .eval()?;
        let ctor: Function = class
            .get("new")
            .map_err(|_| mlua::Error::runtime(format!("脚本 {} 必须定义 new()", id.display())))?;
        let params = self.lua.create_table()?;
        let components = self.lua.create_table()?;
        for binding in &component_bindings.entries {
            let value = self.lua.create_table()?;
            value.set("type", format!("{:?}", binding.component_type))?;
            value.set("index", binding.handle.index())?;
            value.set("generation", binding.handle.generation())?;
            components.set(binding.key.as_str(), value)?;
        }
        params.set("components", components)?;
        for (name, value) in editor_script.exported_parameters() {
            params.set(name, value)?;
        }
        let instance: Table = ctor.call((class.clone(), params))?;
        let entry = ScriptInstance {
            path: id.to_path_buf(),
            awake: Self::method_key(&self.lua, &instance, "on_awake")?,
            start: Self::method_key(&self.lua, &instance, "start")?,
            update: Self::method_key(&self.lua, &instance, "update")?,
            event: Self::method_key(&self.lua, &instance, "on_event")?,
            destroy: Self::method_key(&self.lua, &instance, "on_destroy")?,
            instance: self.lua.create_registry_value(instance)?,
        };
        self.call_no_arg(&entry, entry.awake.as_ref())?;
        lua_info!("[Lua] script instance ready: {}", id.display());
        self.scripts.push(entry);
        Ok(())
    }

    fn method_key(lua: &Lua, instance: &Table, name: &str) -> mlua::Result<Option<RegistryKey>> {
        let method = instance.get::<Function>(name).ok();
        method.map(|f| lua.create_registry_value(f)).transpose()
    }

    fn function(
        &self,
        entry: &ScriptInstance,
        key: &RegistryKey,
    ) -> mlua::Result<(Table, Function)> {
        Ok((
            self.lua.registry_value(&entry.instance)?,
            self.lua.registry_value(key)?,
        ))
    }

    fn call_no_arg(&self, entry: &ScriptInstance, key: Option<&RegistryKey>) -> mlua::Result<()> {
        let Some(key) = key else { return Ok(()) };
        let (instance, function) = self.function(entry, key)?;
        function.call::<()>((instance,))
    }

    fn call_dt(
        &self,
        entry: &ScriptInstance,
        key: Option<&RegistryKey>,
        dt: f32,
    ) -> mlua::Result<()> {
        let Some(key) = key else { return Ok(()) };
        let (instance, function) = self.function(entry, key)?;
        function.call::<()>((instance, dt))
    }

    fn call_event(
        &self,
        entry: &ScriptInstance,
        key: Option<&RegistryKey>,
        name: &str,
        payload: &str,
    ) -> mlua::Result<()> {
        let Some(key) = key else { return Ok(()) };
        let (instance, function) = self.function(entry, key)?;
        function.call::<()>((instance, name, payload))
    }

    fn remove_instance_keys(&self, entry: ScriptInstance) -> mlua::Result<()> {
        self.lua.remove_registry_value(entry.instance)?;
        for key in [
            entry.awake,
            entry.start,
            entry.update,
            entry.event,
            entry.destroy,
        ]
        .into_iter()
        .flatten()
        {
            self.lua.remove_registry_value(key)?;
        }
        Ok(())
    }

    /// 加载场景中的 LuaComponent。组件只保存资源引用，VM 状态仍集中在 Runtime。
    pub fn load_component(&mut self, component: &LuaComponent, id: &Path) -> mlua::Result<()> {
        self.assert_owner_thread();
        if !component.enabled {
            return Ok(());
        }
        if !component.source_override.trim().is_empty() {
            return self.load_script_source_with_components(
                id,
                &component.source_override,
                &component.editor_script,
                &component.components,
            );
        }
        let resource = component.script.as_ref().ok_or_else(|| {
            mlua::Error::runtime("LuaComponent 未选择 LuaScript 资源，且 source_override 为空")
        })?;
        let data = resource.data_ref();
        let script = data
            .as_loaded_ref()
            .ok_or_else(|| mlua::Error::runtime("LuaComponent 脚本资源尚未加载"))?;
        self.load_script_source_with_components(
            id,
            &script.source,
            &component.editor_script,
            &component.components,
        )
    }

    /// 扫描一个真实场景图并加载其中的 LuaComponent。
    ///
    /// LuaComponent 只负责序列化配置，所有实例状态仍由此 Runtime 集中管理；
    /// 因此编辑器在节点上新增或修改组件后，运行时不会使用另一份布局副本。
    pub fn load_scene_components(
        &mut self,
        scene: &fyrox::scene::Scene,
        id_prefix: &Path,
    ) -> mlua::Result<usize> {
        self.assert_owner_thread();
        use fyrox::graph::SceneGraph;

        let mut loaded = 0;
        for (node_handle, node) in scene.graph.pair_iter() {
            for (script_index, component) in node.try_get_scripts::<LuaComponent>().enumerate() {
                if !component.enabled || !component.editor_script.enabled {
                    continue;
                }
                if component.script.is_none() && component.source_override.trim().is_empty() {
                    lua_warn!(
                        "跳过未选择 LuaScript 的 LuaComponent: 节点 {:?}, 脚本索引 {}",
                        node_handle,
                        script_index
                    );
                    continue;
                }
                let id = id_prefix.join(format!(
                    "node_{}_script_{}",
                    node_handle.index(),
                    script_index
                ));
                self.load_component(component, &id)?;
                loaded += 1;
            }
        }
        Ok(loaded)
    }

    /// 返回当前 VM 中已实例化的脚本数量，供宿主状态面板和自检使用。
    pub fn script_count(&self) -> usize {
        self.assert_owner_thread();
        self.scripts.len()
    }
    pub fn call_all(&mut self, name: &str, dt: f32) -> mlua::Result<()> {
        self.assert_owner_thread();
        for entry in &self.scripts {
            let key = match name {
                "update" => entry.update.as_ref(),
                _ => None,
            };
            self.call_dt(entry, key, dt)?;
        }
        if name == "update" {
            self.update_ticks += 1;
            if self.update_ticks == 1 || self.update_ticks % 300 == 0 {
                // lua_info!(
                //     "[Lua] update heartbeat: tick={}, scripts={}, events={}",
                //     self.update_ticks,
                //     self.scripts.len(),
                //     self.events_dispatched
                // );
            }
        }
        Ok(())
    }
    pub fn start(&mut self) -> mlua::Result<()> {
        self.assert_owner_thread();
        self.call_no_args("start")?;
        lua_info!(
            "[Lua] lifecycle start complete: scripts={}",
            self.scripts.len()
        );
        Ok(())
    }
    pub fn destroy(&mut self) -> mlua::Result<()> {
        self.assert_owner_thread();
        self.call_no_args("on_destroy")
    }
    fn call_no_args(&self, name: &str) -> mlua::Result<()> {
        for entry in &self.scripts {
            let key = match name {
                "start" => entry.start.as_ref(),
                "on_destroy" => entry.destroy.as_ref(),
                _ => None,
            };
            self.call_no_arg(entry, key)?;
        }
        Ok(())
    }
    pub fn dispatch_script_event(&mut self, name: &str, payload: &str) -> mlua::Result<()> {
        self.assert_owner_thread();
        lua_info!(
            "[Lua] event dispatch: name={}, payload_len={}, scripts={}",
            name,
            payload.len(),
            self.scripts.len()
        );
        for entry in &self.scripts {
            self.call_event(entry, entry.event.as_ref(), name, payload)?;
        }
        self.events_dispatched += 1;
        Ok(())
    }

    /// Dispatches a Fyrox UI click to the callback registered by Lua.
    pub fn dispatch_ui_click(&mut self, id: &str) -> mlua::Result<bool> {
        self.assert_owner_thread();
        let callbacks: Table = match self.lua.globals().get("__fwok_ui_clicks") {
            Ok(table) => table,
            Err(_) => return Ok(false),
        };
        let callback: Function = match callbacks.get(id) {
            Ok(callback) => callback,
            Err(_) => return Ok(false),
        };
        callback.call::<()>(())?;
        Ok(true)
    }
}
