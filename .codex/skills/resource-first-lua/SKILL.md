---
name: resource-first-lua
description: Enforce resource-first Fyrox scene and Lua workflows: create nodes/components in scenes or prefabs, then resolve and operate them from Lua.
---

# Resource-first Lua workflow

Use this skill for Fyrox scene, UI, prefab, and Lua gameplay changes.

- Create UI nodes, scene nodes, components, scripts, and exported parameters in the editor resource (`.ui`, `.rgs`, prefab, or component resource).
- Project Lua source has one canonical root: `./data/scripts`, matching `fyrox-lua.toml`. Do not add Lua files under another root or create a second `data` directory.
- Scene and prefab components must reference external Lua resources under `data/scripts`; do not embed Lua source in `.rgs`/prefab resources for debugging or tests.
- Lua code must acquire existing resources with generic lookup APIs such as `ui.find()` or `scene.find()`, cache the returned handle, and then mutate supported properties or subscribe to events.
- Rust binding code may implement generic userdata, lookup, handles, messages, and lifecycle dispatch. It must not branch on business names or create game-specific nodes/components.
- Keep `create_*` APIs only as generic capability or tests; normal gameplay and UI scripts must not call them.
- Add a Lua module test when introducing `require`/imports. The test should exercise module export, a Lua-created table/closure state, and a call from another script.
- When a resource is missing, report a diagnostic and leave the handle unresolved; do not silently construct a replacement node.
- The current HUD contract is serialized in `data/unnamed.ui`: `hud_title`, `task`, `time`, `chat_log`, `chat_input`, `send_button`, `inventory_button`, and `skill_button`. A Lua `find` call is not proof of a visible control until the resource contains the named node and the runtime reports a successful resolve.
- Verification must include the real executor or hot-reload executor, a UI diagnostic with `draw_commands > 0`, and at least one real `ButtonMessage::Click` routed into Lua. Lua lifecycle logs alone are insufficient.
- Do not add placeholder, bridge, or ghost UI objects in Rust to hide a missing resource. Fix the `.ui`/`.rgs` asset or report the unresolved resource.

Before finishing, search for dynamic scene/UI creation and verify every business object used by Lua exists in the serialized resource.
