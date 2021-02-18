use std::{
    sync::Arc,
    collections::HashMap,
};

use crate::{
    bot::User,
    error::{ Error, ErrorKind, Result },
};

use typemap::Key;
use serde::{ Serialize, Deserialize };

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Config {
    pub logging: LoggingConfig,
    pub www: WwwConfig,
    pub default_user: UserConfig,
    pub users: HashMap<String, UserConfig>,
}

impl Config {
    pub fn load() -> Result<Self> {
        let mut cfg = config::Config::default();
        cfg
            .merge(config::File::with_name("config").required(false))?
            .merge(config::Environment::with_prefix("SELFPORTRAIT").separator("__"))?;
        let mut cfg: Config = cfg.try_into()?;

        if let Ok(port) = std::env::var("PORT") {
            // Heroku web port
            cfg.www.bind_port = port.parse()
                                        .map_err(|err| Error::from(ErrorKind::Config, err))?;
        }

        cfg.www.format_base_url()?;

        Ok(cfg)
    }

    pub fn users(&self) -> Vec<User> {
        self.users
                .iter()
                .map(|pair| pair.1)
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

    pub fn to_json(&self, pretty: bool) -> Result<String> {
        if pretty {
            Ok(serde_json::to_string_pretty(self)?)
        } else {
            Ok(serde_json::to_string(self)?)
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LoggingConfig {
    pub file: std::path::PathBuf,
    pub level: log::LevelFilter,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WwwConfig {
    pub enabled: bool,
    pub bind_host: String,
    pub bind_port: u16,
    pub base_url: String,
    pub workers: usize,
    pub twitch_emotes_manager_host: String,
    pub twitch_emotes_manager_port: u16,
}

#[derive(Debug, Default, Clone, PartialEq, Serialize, Deserialize)]
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

impl Key for Config {
    type Value = Arc<Config>;
}
