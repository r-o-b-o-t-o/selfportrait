use crate::user::User;

use typemap::Key;
use std::sync::{ Arc, Mutex };

#[derive(Debug, Clone, PartialEq, serde::Deserialize)]
pub struct Config {
    pub log_file: std::path::PathBuf,
    pub log_level: log::LevelFilter,
    pub users: Vec<User>,
}

impl Key for Config {
    type Value = Arc<Mutex<Config>>;
}
