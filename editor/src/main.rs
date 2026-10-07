//! FWOK editor executable.

mod lua_binding;

use fyrox::event_loop::EventLoop;
use fyrox_mcp::McpEditorPlugin;
use fyroxed_base::{Editor, StartupData};
use std::path::PathBuf;

fn project_root_from_args() -> PathBuf {
    let mut args = std::env::args_os().skip(1);
    while let Some(argument) = args.next() {
        if argument == "--project-directory" {
            return args
                .next()
                .map(PathBuf::from)
                .expect("--project-directory requires a path");
        }
    }

    // Explorer may start an exe with an unrelated current directory. Prefer
    // the adjacent packaged project so double-clicking release/editor.exe works.
    if let Some(executable_directory) = std::env::current_exe()
        .ok()
        .and_then(|path| path.parent().map(PathBuf::from))
    {
        if executable_directory.join("data").is_dir() {
            return executable_directory;
        }
    }

    std::env::current_dir().expect("Unable to determine project directory")
}

fn main() {
    let project_root = project_root_from_args();
    std::env::set_current_dir(&project_root).unwrap_or_else(|error| {
        panic!(
            "Unable to use project directory {}: {error}",
            project_root.display()
        )
    });
    if let Err(error) = lua_plugin::initialize_editor_lua(&project_root) {
        eprintln!("[Lua] editor initialization failed: {error}");
    }
    let event_loop = EventLoop::new().expect("Unable to create event loop");
    let mut editor = Editor::new(Some(StartupData {
        working_directory: project_root,
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
}
