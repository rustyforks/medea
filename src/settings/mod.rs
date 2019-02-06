use config::{Config, Environment, File, FileFormat};
use failure::Error;
use serde_derive::{Deserialize, Serialize};
use toml::to_string;

mod duration;
mod server;

#[derive(Debug, Default, Deserialize, Serialize)]
pub struct Settings {
    server: server::Server,
}

impl Settings {
    pub fn new() -> Result<Self, Error> {
        let mut cfg = Config::new();
        let defaults = to_string(&Settings::default())?;
        cfg.merge(File::from_str(defaults.as_str(), FileFormat::Toml))?;
        cfg.merge(File::with_name("config"))?;
        cfg.merge(Environment::with_prefix("conf").separator("__"))?;

        let s: Settings = cfg.try_into()?;
        Ok(s)
    }
}
