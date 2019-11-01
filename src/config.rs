use std::sync::{ Arc, Mutex };

use crate::{
    bot::User,
    error::{ Error, ErrorKind, Result },
};

use typemap::Key;

#[derive(Debug, Clone, PartialEq, serde::Deserialize)]
pub struct Config {
    pub logging: LoggingConfig,
    pub users: Vec<User>,
}

impl Config {
    pub fn load() -> Result<Self> {
        let config_str = std::fs::read_to_string("config.toml")
                                 .map_err(|err| Error::from(ErrorKind::ConfigurationFileRead, err))?;
        toml::from_str(&config_str)
             .map_err(|err| Error::from(ErrorKind::ConfigurationParse, err))
    }
}

#[derive(Debug, Clone, PartialEq, serde::Deserialize)]
pub struct LoggingConfig {
    pub file: std::path::PathBuf,
    pub level: log::LevelFilter,
}

impl Key for Config {
    type Value = Arc<Mutex<Config>>;
}
