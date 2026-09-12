use fwok_game::Game;
use fyrox::plugin::Plugin;

#[no_mangle]
pub fn fyrox_plugin() -> Box<dyn Plugin> {
    Box::new(Game::default())
}
