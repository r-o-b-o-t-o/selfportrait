use std::{
    sync::Arc,
    path::{ Path, PathBuf },
};

use crate::{
    www::library,
    config::Config,
    Error, ErrorKind, Result,
};

use typemap::Key;
use serde::Deserialize;

pub struct Emote {
    pub path: PathBuf,
    pub file_name: String,
    pub name: String,
    pub bytes: Vec<u8>,
}

impl Emote {
    pub fn as_attachment(&self) -> (&[u8], &str) {
        (self.bytes.as_slice(), &self.file_name)
    }
}

pub struct EmoteManager {
    emotes: Vec<Emote>,
    text_emotes: Vec<(Vec<&'static str>, &'static str)>,
    www_config: crate::config::WwwConfig,
}

impl EmoteManager {
    pub fn new(config: &Config, assets_directory: &Path) -> Result<Self> {
        let mut mngr = Self {
            emotes: Vec::new(),
            text_emotes: vec![
                (vec!["lf", "lennyface", "lenny"], "( ͡° ͜ʖ ͡°)"),
                (vec!["shrug", "s"], r"¯\\\_(ツ)\_/¯"),
            ],
            www_config: config.www.clone(),
        };

        for dir in ["emojis", "gifs", "sounds"].iter() {
            let mut path = PathBuf::from(assets_directory);
            path.push(dir);
            mngr.load_emotes_in_dir(&path).map_err(|err| Error::from(ErrorKind::LoadEmotes, err))?;
        }

        Ok(mngr)
    }

    fn load_emotes_in_dir(&mut self, dir: &Path) -> std::io::Result<()> {
        for entry in dir.read_dir()? {
            let entry = entry?;
            let path = entry.path();

            if path.is_file() {
                let bytes = std::fs::read(&path)?;
                let file_name = path.file_name()
                                    .ok_or_else(|| std::io::Error::new(std::io::ErrorKind::Other, "no file name"))?
                                    .to_string_lossy()
                                    .into();
                let name = path.with_extension("")
                                .file_name()
                                .ok_or_else(|| std::io::Error::new(std::io::ErrorKind::Other, "no file name"))?
                                .to_string_lossy()
                                .to_lowercase();

                self.emotes.push(Emote {
                    path,
                    file_name,
                    name,
                    bytes,
                });
            }
        }
        Ok(())
    }

    pub fn find_emote_by_name(&self, name: &str) -> Option<&Emote> {
        let name = name.to_lowercase();
        self.emotes.iter().find(|emote| emote.name == name)
    }

    pub fn find_twitch_emote_urls(&self, query: &str, limit: usize, exact_match: bool) -> Result<Vec<library::Emote>> {
        let query = query.to_lowercase();
        let limit = limit.clamp(0, 50);

        #[derive(Debug, Deserialize)]
        struct ManagerSearchResult {
            pub id: u64,
            pub name: String,
            pub url: String,
        };

        let url = format!(
            "http://{}:{}/emotes/search?q={}&maxresults={}&exactmatch={}",
            self.www_config.twitch_emotes_manager_host,
            self.www_config.twitch_emotes_manager_port,
            query, limit, if exact_match { 1 } else { 0 },
        );
        let res = reqwest::blocking::get(&url)?;
        let res: Vec<ManagerSearchResult> = res.json()?;

        Ok(res.iter().map(|e| library::Emote {
            name: e.name.clone(),
            url: e.url.replace("2.0", "3.0").replace("1.0", "3.0"),
        }).collect())
    }

    pub fn find_twitch_emote(&self, name: &str) -> Result<Option<Emote>> {
        let name = name.to_lowercase();

        let res = self.find_twitch_emote_urls(&name, 1, true)?;
        Ok(match res.first() {
            Some(emote) => {
                let mut res = reqwest::blocking::get(&emote.url)?;
                let mut bytes: Vec<u8> = Vec::new();
                res.copy_to(&mut bytes)?;
                Some(Emote {
                    path: PathBuf::new(),
                    file_name: format!("{}.png", name),
                    name,
                    bytes,
                })
            },
            None => None,
        })
    }

    pub fn n_emotes(&self) -> usize {
        self.emotes.len()
    }

    pub fn emotes(&self) -> &Vec<Emote> {
        &self.emotes
    }

    pub fn text_emotes(&self) -> &Vec<(Vec<&'static str>, &'static str)> {
        &self.text_emotes
    }
}

impl Key for EmoteManager {
    type Value = Arc<EmoteManager>;
}
