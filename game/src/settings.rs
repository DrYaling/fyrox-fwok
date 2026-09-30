use serde::Deserialize;
use std::{
    fs,
    path::{Path, PathBuf},
};

#[derive(Clone, Debug, Deserialize, Default)]
#[serde(default)]
pub struct GameSettings {
    pub editor_font: String,
    pub game_font: String,
}

impl GameSettings {
    pub fn load(path: impl AsRef<Path>) -> Result<Self, Box<dyn std::error::Error>> {
        let path = path.as_ref();
        if !path.exists() {
            return Ok(Self::default());
        }
        Ok(toml::from_str(&fs::read_to_string(path)?)?)
    }

    pub fn game_font_path(&self) -> Option<PathBuf> {
        (!self.game_font.trim().is_empty()).then(|| self.game_font.clone().into())
    }
}
