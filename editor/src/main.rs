//! FWOK editor executable.

mod lua_binding;

use fyrox::event_loop::EventLoop;
use fyrox_mcp::McpEditorPlugin;
use fyroxed_base::{Editor, StartupData};
use std::process::{Child, Command};

// Temporary integration: start the external MCP bridge from the editor.
// This keeps the current two-process bridge while a proper launcher/configuration
// flow is developed. Set MCP_BRIDGE_BIN to override the executable path.
fn start_external_mcp() -> Option<Child> {
    let candidates = std::env::var_os("MCP_BRIDGE_BIN")
        .map(std::path::PathBuf::from)
        .into_iter()
        .chain([
            std::path::PathBuf::from("data/editor/mcp-bridge.exe"),
            std::path::PathBuf::from("target/debug/mcp-bridge.exe"),
            std::path::PathBuf::from("target/release/mcp-bridge.exe"),
        ]);
    for path in candidates {
        if !path.is_file() {
            continue;
        }
        match Command::new(&path)
            .current_dir(std::env::current_dir().ok()?)
            .spawn()
        {
            Ok(child) => {
                eprintln!("MCP bridge started: {}", path.display());
                return Some(child);
            }
            Err(error) => eprintln!("MCP bridge start failed ({}): {error}", path.display()),
        }
    }
    eprintln!("MCP bridge executable not found; build mcp-bridge or set MCP_BRIDGE_BIN");
    None
}

fn main() {
    let event_loop = EventLoop::new().expect("Unable to create event loop");
    let mut mcp_process = start_external_mcp();
    let mut editor = Editor::new(Some(StartupData {
        working_directory: Default::default(),
        scenes: vec!["data/rpg_level.rgs".into()],
        named_objects: false,
    }));
    editor.add_game_plugin(fwok_game::Game::for_editor());
    editor.add_editor_plugin(McpEditorPlugin::default());
    editor.settings.general.keep_editor_active = true;
    editor.settings.general.suspend_unfocused_editor = false;

    lua_binding::register(&mut editor);
    if std::env::var_os("FWOK_EDITOR_HEADLESS").is_some() {
        drop(event_loop);
        editor.run_headless();
    } else {
        editor.run(event_loop);
    }
    if let Some(mut child) = mcp_process.take() {
        let _ = child.kill();
        let _ = child.wait();
    }
}
