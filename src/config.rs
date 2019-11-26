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
    pub default_user: UserConfig,
    pub users: Vec<UserConfig>,
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

    pub fn users(&self) -> Vec<User> {
        self.users
                .iter()
                .map(|user_config| User {
                    active: match user_config.active {
                        Some(val) => val,
                        None => match self.default_user.active {
                            Some(val) => val,
                            None => true,
                        }
                    },
                    discord_id: match user_config.discord_id {
                        Some(val) => val,
                        None => match self.default_user.discord_id {
                            Some(val) => val,
                            None => 0,
                        }
                    },
                    token: match &user_config.token {
                        Some(val) => val.clone(),
                        None => match &self.default_user.token {
                            Some(val) => val.clone(),
                            None => String::new(),
                        }
                    },
                    command_prefix: match &user_config.command_prefix {
                        Some(val) => val.clone(),
                        None => match &self.default_user.command_prefix {
                            Some(val) => val.clone(),
                            None => "s.".to_owned(),
                        }
                    },
                    emote_prefix: match &user_config.emote_prefix {
                        Some(val) => val.clone(),
                        None => match &self.default_user.emote_prefix {
                            Some(val) => val.clone(),
                            None => ">".to_owned(),
                        }
                    },
                    twitch_emote_prefix: match &user_config.twitch_emote_prefix {
                        Some(val) => val.clone(),
                        None => match &self.default_user.twitch_emote_prefix {
                            Some(val) => val.clone(),
                            None => "%".to_owned(),
                        }
                    },
                    text_emote_prefix: match &user_config.text_emote_prefix {
                        Some(val) => val.clone(),
                        None => match &self.default_user.text_emote_prefix {
                            Some(val) => val.clone(),
                            None => "$".to_owned(),
                        }
                    },
                })
                .collect()
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

#[derive(Debug, Default, Clone, PartialEq, serde::Deserialize)]
pub struct UserConfig {
    pub active: Option<bool>,
    pub discord_id: Option<u64>,
    pub token: Option<String>,
    pub command_prefix: Option<String>,
    pub emote_prefix: Option<String>,
    pub twitch_emote_prefix: Option<String>,
    pub text_emote_prefix: Option<String>,
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
