use std::{
    sync::Arc,
    collections::HashMap,
};

use crate::{
    bot::User,
    error::{ Error, ErrorKind, Result },
};

use typemap::Key;

#[derive(Debug, Clone, PartialEq, serde::Deserialize)]
pub struct Config {
    pub logging: LoggingConfig,
    pub www: WwwConfig,
    pub users: Vec<User>,
    pub tools: Option<ToolsConfig>,
}

impl Config {
    pub fn load() -> Result<Self> {
        let config_str = std::fs::read_to_string("config.toml")
                                    .map_err(|err| Error::from(ErrorKind::ConfigurationFileRead, err))?;
        let mut config: Self = toml::from_str(&config_str)
                                    .map_err(|err| Error::from(ErrorKind::ConfigurationParse, err))?;

        config.www.format_base_url()?;

        Ok(config)
    }
}

#[derive(Debug, Clone, PartialEq, serde::Deserialize)]
pub struct LoggingConfig {
    pub file: std::path::PathBuf,
    pub level: log::LevelFilter,
}

#[derive(Debug, Clone, PartialEq, serde::Deserialize)]
pub struct WwwConfig {
    pub enabled: bool,
    pub bind_host: String,
    pub bind_port: u16,
    pub base_url: String,
    pub workers: usize,
}

impl WwwConfig {
    pub fn format_base_url(&mut self) -> Result<()> {
        let mut args = HashMap::new();
        args.insert("port".into(), format!("{}", self.bind_port));

        self.base_url = strfmt::strfmt(&self.base_url, &args).map_err(|err| Error::from(ErrorKind::ParseWwwBaseUrl, err))?;
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, serde::Deserialize)]
pub struct ToolsConfig {
    pub twitch_app_client_id: Option<String>,
}

impl Key for Config {
    type Value = Arc<Config>;
}
