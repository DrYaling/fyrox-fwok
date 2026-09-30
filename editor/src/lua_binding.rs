//! Lua integration owned by the FWOK editor executable.

use fyrox::gui::inspector::editors::{
    collection::VecCollectionPropertyEditorDefinition,
    inspectable::InspectablePropertyEditorDefinition,
};
use fyroxed_base::{
    plugins::inspector::editors::resource::ResourceFieldPropertyEditorDefinition, Editor,
};

pub fn register(editor: &mut Editor) {
    let sender = editor.message_sender.clone();
    editor
        .property_editors
        .insert(ResourceFieldPropertyEditorDefinition::<lua_plugin::LuaScript>::new(sender));
    editor
        .property_editors
        .insert(InspectablePropertyEditorDefinition::<
            lua_plugin::EditorScript,
        >::new());
    editor
        .property_editors
        .insert(InspectablePropertyEditorDefinition::<
            lua_plugin::LuaScriptParameter,
        >::new());
    editor
        .property_editors
        .register_inspectable::<lua_plugin::LuaComponent>();
    editor
        .property_editors
        .insert(VecCollectionPropertyEditorDefinition::<
            lua_plugin::LuaScriptParameter,
        >::new());
    lua_plugin::register_fyrox_resources(
        &editor.engine.resource_manager,
        &editor.engine.serialization_context.script_constructors,
    );
}
