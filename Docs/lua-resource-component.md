# Lua resources and LuaComponent

## What is registered

`lua_binding::LuaScript` is a native Fyrox resource for `.lua` files. The Editor and Executor
register its loader before the project resource registry is scanned, so `.lua` files are treated
like textures and models:

- they appear in the Asset Browser;
- their metadata is written to `data/resources.registry`;
- they can be selected through a `ResourceFieldPropertyEditorDefinition`.

`lua_binding::LuaComponent` is a serializable Fyrox `Script` with these Inspector fields:

- `script`: `Option<Resource<LuaScript>>`, selected with the standard asset picker;
- `source_override`: directly editable Lua source in Inspector. When non-empty it takes precedence
  over `script`, which makes binding and rapid prototyping possible without leaving the scene;
- `enabled`: whether the component is enabled for the runtime bridge;
- `editor_script.enabled`: enables the EditorScript configuration;
- `editor_script.parameters`: an editable list of `{ name, value, exported }` entries. `value` is
  stored as text so the resource remains stable and serializable.

The host applications (`executor` and `fyroxed`) register the resource loader and component constructor
before scanning the project registry or loading the game DLL. Fyrox core does not own Lua initialization;
the game plugin may call the same idempotent helper when it is statically linked.

## Workflow

1. Start the editor with `scripts/run-editor.ps1`.
2. In the Asset Browser, use **Add Resource** and select `LuaScript`, or create a `.lua` file
   under the project directory and refresh the browser.
3. Select a Node, choose **Add Script -> Lua Component**.
4. In the component Inspector, choose the `.lua` resource in the `script` field.
5. Expand **Editor Script -> Parameters**, add entries, and set each parameter name/value. Disable
   `exported` for editor-only metadata. Exported entries can be passed to `new(class, params)` by
   the runtime bridge.

New Lua resources are created from a valid template containing `new`, `on_awake`, `start`,
`update`, and `on_destroy` methods.

## Hot reload lifetime

The Lua resource loader and `LuaComponent` constructor are registered by the host applications rather
than owned by Fyrox engine code or the game DLL. This keeps their vtables valid while the game DLL is
replaced and allows `.lua` resources to be selected in the Editor.

`LuaComponent` currently provides the resource reference, an Inspector-editable `EditorScript`
parameter block. `LuaRuntime::load_scene_components` scans the actual scene graph at startup and
connects each selected resource to the shared VM; exported parameters are passed to `new(class,
params)`, followed by `on_awake`, `start`, `update`, `on_event`, and `on_destroy`.
