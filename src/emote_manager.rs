use std::{
    sync::Arc, collections::HashMap,
    path::{ Path, PathBuf },
};

use crate::{
    Error, ErrorKind, Result,
    config::Config,
};

use typemap::Key;

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
    twitch_emotes: HashMap<String, String>,
}

impl EmoteManager {
    pub fn new(config: &Config, assets_directory: &Path) -> Result<Self> {
        let mut mngr = Self {
            emotes: Vec::new(),
            text_emotes: vec![
                (vec!["lf", "lennyface", "lenny"], "( ͡° ͜ʖ ͡°)"),
                (vec!["shrug", "s"], "¯\\\\\\_(ツ)\\_/¯"),
            ],
            twitch_emotes: HashMap::new(),
        };

        for dir in ["emojis", "gifs", "sounds"].iter() {
            let mut path = PathBuf::from(assets_directory);
            path.push(dir);
            mngr.load_emotes_in_dir(&path).map_err(|err| Error::from(ErrorKind::LoadEmotes, err))?;
        }

        if config.fetch_twitch_emotes_infos {
            if let Some(client_id) = &config.twitch_app_client_id {
                let twitch_emotes_json = crate::tools::fetch_twitch_emotes::get_list_from_api(client_id);
                match twitch_emotes_json {
                    Ok(json) => match serde_json::from_str::<crate::tools::fetch_twitch_emotes::TwitchResponse>(&json) {
                        Ok(twitch_emotes) => {
                            drop(json);
                            for emote in twitch_emotes.emoticons {
                                let url = emote.images.url.replace("1.0", "3.0");
                                let name = emote.regex.to_lowercase();
                                mngr.twitch_emotes.insert(name, url);
                            }
                            log::info!("Loaded {} Twitch emotes URLs", mngr.twitch_emotes.len());
                        },
                        Err(err) => {
                            let err: Error = err.into();
                            log::error!("Could not deserialize twitch emotes data: {}", err);
                        },
                    },
                    Err(err) => log::error!("{}", err),
                }
            }
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

    pub fn find_twitch_emote(&self, name: &str) -> Result<Option<Emote>> {
        // Search for the emote on disk first
        let file_name = format!("{}.png", name.to_lowercase());
        let mut path = PathBuf::new();
        path.push("assets");
        path.push("twitchemotes");
        path.push(&file_name);

        if path.exists() {
            let bytes = std::fs::read(&path)?;

            Ok(Some(Emote {
                path,
                file_name,
                name: name.to_owned(),
                bytes,
            }))
        } else {
            // If not on disk, check if we have the URL for this emote and download from CDN
            match self.twitch_emotes.get(name) {
                Some(url) => {
                    let mut res = reqwest::get(url)?;
                    let mut bytes: Vec<u8> = Vec::new();
                    res.copy_to(&mut bytes)?;
                    Ok(Some(Emote {
                        path: PathBuf::new(),
                        file_name: url.to_string(),
                        name: name.to_owned(),
                        bytes,
                    }))
                },
                None => Ok(None),
            }
        }
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
