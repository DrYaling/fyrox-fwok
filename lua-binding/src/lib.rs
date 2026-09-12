//! Lua VM、TOML 配置与事件管理器。
use mlua::{Function, Lua, RegistryKey, Table};
use serde::{Deserialize, Serialize};
use std::{
    fs,
    path::{Path, PathBuf},
    sync::{Arc, Mutex},
};

/// Lua 暴露策略。编辑器只提供反射入口，发布包则注册稳定的完整 API。
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum BindingMode {
    EditorReflection,
    PackageFull,
}
impl Default for BindingMode {
    fn default() -> Self {
        Self::PackageFull
    }
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct LuaConfig {
    pub script_root: PathBuf,
    pub jit: bool,
    pub reload_mode: ReloadMode,
    pub enabled: bool,
    pub binding_mode: BindingMode,
}
impl Default for LuaConfig {
    fn default() -> Self {
        Self {
            script_root: "data/scripts".into(),
            jit: false,
            reload_mode: ReloadMode::Immediate,
            enabled: true,
            // 编辑器/开发环境默认使用反射；发布构建会在运行时强制完整绑定。
            binding_mode: BindingMode::EditorReflection,
        }
    }
}
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum ReloadMode {
    Immediate,
    OnStop,
}
impl Default for ReloadMode {
    fn default() -> Self {
        Self::Immediate
    }
}
impl LuaConfig {
    /// 返回当前构建目标实际使用的绑定模式。发布构建永远是完整绑定。
    pub fn effective_binding_mode(&self) -> BindingMode {
        if cfg!(debug_assertions) {
            self.binding_mode
        } else {
            BindingMode::PackageFull
        }
    }
    pub fn load(path: impl AsRef<Path>) -> Result<Self, Box<dyn std::error::Error>> {
        let p = path.as_ref();
        if !p.exists() {
            return Ok(Self::default());
        }
        Ok(toml::from_str(&fs::read_to_string(p)?)?)
    }
    pub fn save(&self, path: impl AsRef<Path>) -> Result<(), Box<dyn std::error::Error>> {
        let p = path.as_ref();
        if let Some(d) = p.parent() {
            fs::create_dir_all(d)?
        }
        fs::write(p, toml::to_string_pretty(self)?)?;
        Ok(())
    }
}
#[derive(Debug, Clone)]
pub struct GameEvent {
    pub name: String,
    pub payload: String,
}
#[derive(Clone, Default)]
pub struct EventManager {
    queue: Arc<Mutex<Vec<GameEvent>>>,
}
impl EventManager {
    pub fn emit(&self, name: impl Into<String>, payload: impl Into<String>) {
        self.queue.lock().unwrap().push(GameEvent {
            name: name.into(),
            payload: payload.into(),
        })
    }
    pub fn drain(&self) -> Vec<GameEvent> {
        std::mem::take(&mut *self.queue.lock().unwrap())
    }
}
pub trait LuaGameApi: Send + Sync + 'static {
    /// 发布构建使用的完整、稳定 API。
    fn register(&self, lua: &Lua, events: EventManager) -> mlua::Result<()>;
    /// 编辑器模式的轻量反射入口。默认不暴露完整 API，避免编辑器绑定膨胀。
    fn register_reflection(&self, _lua: &Lua, _events: EventManager) -> mlua::Result<()> {
        Ok(())
    }
}

/// 编辑器反射调用桥。实现方只需按名称解析对象和方法，避免为每个 Fyrox 类型生成绑定。
pub trait LuaReflection: Send + Sync + 'static {
    fn call(&self, object: &str, method: &str, args: Vec<String>) -> Result<String, String>;
}
/// 每个 Lua 文件对应一个独立实例；实例状态保存在 table 中，重载时可迁移。
pub struct LuaRuntime {
    pub lua: Lua,
    scripts: Vec<(PathBuf, RegistryKey)>,
    events: EventManager,
    config: LuaConfig,
}
impl std::fmt::Debug for LuaRuntime {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("LuaRuntime")
            .field("scripts", &self.scripts.len())
            .finish()
    }
}
impl PartialEq for LuaRuntime {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(self, other)
    }
}
impl LuaRuntime {
    pub fn new(config: LuaConfig, api: impl LuaGameApi) -> mlua::Result<Self> {
        let script_root = config.script_root.clone();
        let lua = Lua::new();
        let events = EventManager::default();
        let binding_mode = config.effective_binding_mode();
        match binding_mode {
            BindingMode::EditorReflection => api.register_reflection(&lua, events.clone())?,
            BindingMode::PackageFull => api.register(&lua, events.clone())?,
        }
        let mut s = Self {
            lua,
            scripts: Vec::new(),
            events,
            config,
        };
        s.load_root(&script_root)?;
        Ok(s)
    }
    pub fn binding_mode(&self) -> BindingMode {
        self.config.effective_binding_mode()
    }
    pub fn config(&self) -> &LuaConfig {
        &self.config
    }
    /// 编辑器反射模式下注册 reflection.call(object, method, ...)。
    pub fn register_reflection<R: LuaReflection>(&self, reflection: R) -> mlua::Result<()> {
        let r = std::sync::Arc::new(reflection);
        let table = self.lua.create_table()?;
        let callback = self.lua.create_function(
            move |lua, (object, method, args): (String, String, mlua::Variadic<String>)| {
                let value = r
                    .call(&object, &method, args.into_iter().collect())
                    .map_err(mlua::Error::external)?;
                Ok(lua.create_string(value)?)
            },
        )?;
        table.set("call", callback)?;
        self.lua.globals().set("reflection", table)
    }
    /// Tolua 风格的单文件热更：销毁旧实例，再加载新 chunk 并执行 awake。
    pub fn reload_script(&mut self, path: &Path) -> mlua::Result<()> {
        let canonical = path.to_path_buf();
        if let Some(index) = self.scripts.iter().position(|(p, _)| *p == canonical) {
            let (_, key) = self.scripts.remove(index);
            let old_instance: Table = self.lua.registry_value(&key)?;
            if let Ok(f) = old_instance.get::<Function>("on_destroy") {
                f.call::<()>((old_instance,))?;
            }
            self.lua.remove_registry_value(key)?;
        }
        self.load_script(path)
    }
    pub fn load_root(&mut self, root: &Path) -> mlua::Result<()> {
        let mut files = Vec::new();
        collect(root, &mut files);
        for p in files {
            self.load_script(&p)?
        }
        Ok(())
    }
    /// 脚本固定返回 class table，且必须含有 new() 和 on_awake(self)。
    pub fn load_script(&mut self, p: &Path) -> mlua::Result<()> {
        let class: Table = self
            .lua
            .load(fs::read_to_string(p).map_err(mlua::Error::external)?)
            .set_name(p.to_string_lossy())
            .eval()?;
        let ctor: Function = class
            .get("new")
            .map_err(|_| mlua::Error::runtime(format!("脚本 {} 必须定义 new()", p.display())))?;
        let instance: Table = ctor.call(class.clone())?;
        let awake: Function = class.get("on_awake").map_err(|_| {
            mlua::Error::runtime(format!("脚本 {} 必须定义 on_awake()", p.display()))
        })?;
        awake.call::<()>((instance.clone(),))?;
        self.scripts
            .push((p.to_path_buf(), self.lua.create_registry_value(instance)?));
        Ok(())
    }
    pub fn call_all(&self, name: &str, dt: f32) -> mlua::Result<()> {
        for (_, k) in &self.scripts {
            let t: Table = self.lua.registry_value(k)?;
            if let Ok(f) = t.get::<Function>(name) {
                f.call::<()>((t.clone(), dt))?
            }
        }
        Ok(())
    }
    pub fn start(&self) -> mlua::Result<()> {
        for (_, k) in &self.scripts {
            let t: Table = self.lua.registry_value(k)?;
            if let Ok(f) = t.get::<Function>("start") {
                f.call::<()>((t.clone(),))?
            }
        }
        Ok(())
    }
    pub fn dispatch_events(&self) -> mlua::Result<()> {
        for e in self.events.drain() {
            for (_, k) in &self.scripts {
                let t: Table = self.lua.registry_value(k)?;
                if let Ok(f) = t.get::<Function>("on_event") {
                    f.call::<()>((t.clone(), e.name.clone(), e.payload.clone()))?
                }
            }
        }
        Ok(())
    }
    pub fn destroy(&self) -> mlua::Result<()> {
        for (_, k) in &self.scripts {
            let t: Table = self.lua.registry_value(k)?;
            if let Ok(f) = t.get::<Function>("on_destroy") {
                f.call::<()>((t.clone(),))?
            }
        }
        Ok(())
    }
    pub fn events(&self) -> &EventManager {
        &self.events
    }
}
fn collect(r: &Path, o: &mut Vec<PathBuf>) {
    if let Ok(es) = fs::read_dir(r) {
        for e in es.flatten() {
            let p = e.path();
            if p.is_dir() {
                collect(&p, o)
            } else if p.extension().is_some_and(|x| x == "lua") {
                o.push(p)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    struct Api;
    impl LuaGameApi for Api {
        fn register(&self, _: &Lua, _: EventManager) -> mlua::Result<()> {
            Ok(())
        }
    }
    #[test]
    fn class_lifecycle_and_events_work() {
        let dir = std::env::temp_dir().join(format!("fwok-lua-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let file = dir.join("sample.lua");
        std::fs::write(
            &file,
            r#"local C={}; C.__index=C
            function C.new(class) return setmetatable({count=0},class) end
            function C:on_awake() self.count=self.count+1 end
            function C:start() self.count=self.count+1 end
            function C:update(dt) self.count=self.count+dt end
            function C:on_event(name,payload) self.last=name..payload end
            function C:on_destroy() self.destroyed=true end
            return C"#,
        )
        .unwrap();
        let mut rt = LuaRuntime::new(LuaConfig::default(), Api).unwrap();
        rt.load_script(&file).unwrap();
        rt.start().unwrap();
        rt.call_all("update", 0.5).unwrap();
        rt.events().emit("x", "y");
        rt.dispatch_events().unwrap();
        rt.destroy().unwrap();
        let t: Table = rt.lua.registry_value(&rt.scripts[0].1).unwrap();
        assert_eq!(t.get::<f32>("count").unwrap(), 2.5);
        assert_eq!(t.get::<String>("last").unwrap(), "xy");
        assert!(t.get::<bool>("destroyed").unwrap());
        let _ = std::fs::remove_file(file);
        let _ = std::fs::remove_dir(dir);
    }
    #[test]
    fn awake_is_required() {
        let dir = std::env::temp_dir().join(format!("fwok-lua-awake-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let file = dir.join("bad.lua");
        std::fs::write(
            &file,
            "local C={}; function C.new(class) return class end; return C",
        )
        .unwrap();
        let mut rt = LuaRuntime::new(LuaConfig::default(), Api).unwrap();
        assert!(rt.load_script(&file).is_err());
        let _ = std::fs::remove_file(file);
        let _ = std::fs::remove_dir(dir);
    }
    #[test]
    fn editor_defaults_to_reflection_and_release_is_full() {
        let config = LuaConfig::default();
        assert_eq!(config.binding_mode, BindingMode::EditorReflection);
        assert_eq!(
            config.effective_binding_mode(),
            BindingMode::EditorReflection
        );
    }
}
