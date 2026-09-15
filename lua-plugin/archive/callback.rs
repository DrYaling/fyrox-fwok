//! Lua 回调注册表。回调属于 Lua VM，只保存 RegistryKey，不进入场景序列化。

use fyrox::core::pool::ErasedHandle;
use mlua::{Function, Lua, RegistryKey};
use std::{cell::RefCell, collections::HashMap, rc::Rc};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum UiEventKind {
    Click,
}

#[derive(Default)]
struct CallbackRegistry {
    entries: HashMap<(ErasedHandle, UiEventKind), Vec<RegistryKey>>,
}

#[derive(Clone, Default)]
pub struct RuntimeServices {
    callbacks: Rc<RefCell<CallbackRegistry>>,
}

impl RuntimeServices {
    pub fn add_ui_callback(
        &self,
        lua: &Lua,
        target: ErasedHandle,
        kind: UiEventKind,
        callback: Function,
    ) -> mlua::Result<()> {
        let key = lua.create_registry_value(callback)?;
        self.callbacks
            .borrow_mut()
            .entries
            .entry((target, kind))
            .or_default()
            .push(key);
        Ok(())
    }

    pub fn clear_ui_callbacks(
        &self,
        lua: &Lua,
        target: ErasedHandle,
        kind: UiEventKind,
    ) -> mlua::Result<()> {
        if let Some(keys) = self.callbacks.borrow_mut().entries.remove(&(target, kind)) {
            for key in keys {
                lua.remove_registry_value(key)?;
            }
        }
        Ok(())
    }

    pub fn dispatch_ui_event(
        &self,
        lua: &Lua,
        target: ErasedHandle,
        kind: UiEventKind,
    ) -> mlua::Result<usize> {
        let callbacks = {
            let registry = self.callbacks.borrow();
            registry
                .entries
                .get(&(target, kind))
                .map(|keys| {
                    keys.iter()
                        .map(|key| lua.registry_value::<Function>(key))
                        .collect::<mlua::Result<Vec<_>>>()
                })
                .transpose()?
                .unwrap_or_default()
        };
        for callback in &callbacks {
            callback.call::<()>(())?;
        }
        Ok(callbacks.len())
    }
}
