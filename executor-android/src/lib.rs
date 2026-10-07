//! Android launcher for the FWOK game.
//!
//! Fyrox's Android backend requires an `android_main` entry point and an
//! `AndroidApp`-backed event loop.  This crate is intentionally separate from
//! the desktop `executor` binary because Android packages are native activity
//! libraries, not renamed Windows executables.
#![cfg(target_os = "android")]

use fwok_game::Game;
use fyrox::{
    core::io, engine::executor::Executor, event_loop::EventLoopBuilder,
    platform::android::EventLoopBuilderExtAndroid,
};

#[no_mangle]
fn android_main(app: fyrox::platform::android::activity::AndroidApp) {
    io::ANDROID_APP
        .set(app.clone())
        .expect("ANDROID_APP cannot be set twice.");
    #[allow(deprecated)]
    let event_loop = EventLoopBuilder::new()
        .with_android_app(app)
        .build()
        .expect("failed to create Android event loop");
    let mut executor = Executor::from_params(Some(event_loop), Default::default());
    lua_plugin::register_fyrox_resources(
        &executor.resource_manager,
        &executor.serialization_context.script_constructors,
    );
    executor.add_plugin(Game::default());
    executor.run();
}
