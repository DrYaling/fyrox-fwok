//! 独立运行器：使用已编译的 Fyrox 引擎启动游戏插件。
#[cfg(not(feature = "dylib"))]
use fwok_game::Game;
use fyrox::{
    dpi::LogicalSize,
    engine::{executor::Executor, GraphicsContextParams},
    event_loop::EventLoop,
    window::WindowAttributes,
};
fn main() {
    let mut window = WindowAttributes::default();
    window.title = "FWOK Game".to_string();
    // Local verification can override the default size through the environment.
    let width = std::env::var("FWOK_WINDOW_WIDTH")
        .ok()
        .and_then(|value| value.parse::<f64>().ok())
        .filter(|value| value.is_finite() && *value >= 64.0)
        .unwrap_or(1280.0);
    let height = std::env::var("FWOK_WINDOW_HEIGHT")
        .ok()
        .and_then(|value| value.parse::<f64>().ok())
        .filter(|value| value.is_finite() && *value >= 64.0)
        .unwrap_or(720.0);
    window.inner_size = Some(LogicalSize::new(width, height).into());
    let mut executor = Executor::from_params(
        EventLoop::new().ok(),
        GraphicsContextParams {
            named_objects: false,
            window_attributes: window,
            vsync: true,
            msaa_sample_count: None,
            graphics_server_constructor: Default::default(),
        },
    );
    lua_plugin::register_fyrox_resources(
        &executor.resource_manager,
        &executor.serialization_context.script_constructors,
    );
    #[cfg(feature = "dylib")]
    executor
        .add_dynamic_plugin("game_dylib.dll", true, true)
        .unwrap();
    #[cfg(not(feature = "dylib"))]
    executor.add_plugin(Game::default());
    executor.run();
}
