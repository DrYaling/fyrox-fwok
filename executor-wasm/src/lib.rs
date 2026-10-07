//! Browser launcher for FWOK.
#![cfg(target_arch = "wasm32")]

use fwok_game::Game;
use fyrox::{
    core::wasm_bindgen::{self, prelude::*},
    engine::executor::Executor,
    event_loop::EventLoop,
};

/// Starts the Fyrox game loop after the browser has created the WebGL canvas.
#[wasm_bindgen]
pub fn main() {
    let mut executor = Executor::new(Some(EventLoop::new().expect("event loop")));
    lua_plugin::register_fyrox_resources(
        &executor.resource_manager,
        &executor.serialization_context.script_constructors,
    );
    executor.add_plugin(Game::default());
    executor.run();
}
