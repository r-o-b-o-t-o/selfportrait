use std::{
    sync::Arc, collections::HashMap,
    path::{ Path, PathBuf },
};

use crate::{
    www::library,
    config::Config,
    Error, ErrorKind, Result,
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
    twitch_emotes: HashMap<String, u32>,
}

impl EmoteManager {
    pub fn new(config: &Config, assets_directory: &Path) -> Result<Self> {
        let mut mngr = Self {
            emotes: Vec::new(),
            text_emotes: vec![
                (vec!["lf", "lennyface", "lenny"], "( ͡° ͜ʖ ͡°)"),
                (vec!["shrug", "s"], r"¯\\\_(ツ)\_/¯"),
            ],
            twitch_emotes: HashMap::new(),
        };

        for dir in ["emojis", "gifs", "sounds"].iter() {
            let mut path = PathBuf::from(assets_directory);
            path.push(dir);
            mngr.load_emotes_in_dir(&path).map_err(|err| Error::from(ErrorKind::LoadEmotes, err))?;
        }

        if config.fetch_twitch_emotes_infos {
            // Matches for instance https://static-cdn.jtvnw.net/emoticons/v1/{id}/3.0
            let re = regex::Regex::new(r"https://.*/emoticons/v\d+/(\d+)/\d\.\d").unwrap();

            if let Some(client_id) = &config.twitch_app_client_id {
                let twitch_emotes_json = crate::tools::fetch_twitch_emotes::get_list_from_api(client_id);
                match twitch_emotes_json {
                    Ok(json) => match serde_json::from_str::<crate::tools::fetch_twitch_emotes::TwitchResponse>(&json) {
                        Ok(twitch_emotes) => {
                            drop(json);
                            for emote in twitch_emotes.emoticons {
                                let name = emote.regex.to_lowercase();
                                if let Some(captures) = re.captures(&emote.images.url) {
                                    if captures.len() >= 1 {
                                        let id: u32 = captures[1].parse().unwrap();
                                        mngr.twitch_emotes.insert(name, id);
                                    }
                                };
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

    pub fn find_twitch_emote_urls(&self, query: &str, limit: usize) -> Result<Vec<library::Emote>> {
        let query = query.to_lowercase();
        let limit = limit.min(50); // Clamp the limit to 50 if it is higher

        if !self.twitch_emotes.is_empty() {
            Ok(self.twitch_emotes
                    .iter()
                    .filter(|emote| emote.0.contains(&query))
                    .take(limit)
                    .map(|emote| library::Emote {
                        name: emote.0.into(),
                        url: format!("https://static-cdn.jtvnw.net/emoticons/v1/{}/3.0", emote.1),
                    })
                    .collect::<Vec<_>>())
        } else {
            let mut emotes = Vec::new();
            let mut path = PathBuf::new();
            path.push("assets");
            path.push("twitchemotes");
            if path.exists() {
                fn scandir(path: &Path, query: &str, emotes: &mut Vec<library::Emote>, count: &mut usize, limit: usize) -> std::io::Result<()> {
                    for entry in std::fs::read_dir(path)? {
                        let entry = entry?;
                        let file_name = entry.file_name();
                        let file_name = file_name.to_str().ok_or_else(|| std::io::Error::new(std::io::ErrorKind::Other, "Could not turn file name to string"))?;
                        if entry.file_type()?.is_file() && file_name.contains(query) {
                            emotes.push(library::Emote::new(&entry.path()
                                                        .with_extension("")
                                                        .file_name()
                                                        .ok_or_else(|| std::io::Error::new(std::io::ErrorKind::Other, "No file name"))?
                                                        .to_string_lossy(), &entry.path()));
                            *count += 1;
                            if *count == limit {
                                return Ok(());
                            }
                        } else if entry.file_type()?.is_dir() {
                            scandir(&entry.path(), query, emotes, count, limit)?;
                        }
                    }
                    Ok(())
                }

                let mut count = 0;
                scandir(&path, &query, &mut emotes, &mut count, limit)?;
            }
            Ok(emotes)
        }
    }

    pub fn find_twitch_emote(&self, name: &str) -> Result<Option<Emote>> {
        let name = name.to_lowercase();

        // Search for the emote on disk first
        let file_name = format!("{}.png", name);
        let mut path = PathBuf::new();
        path.push("assets");
        path.push("twitchemotes");
        path.push(&file_name);

        if path.exists() {
            let bytes = std::fs::read(&path)?;

            Ok(Some(Emote {
                path,
                file_name,
                name,
                bytes,
            }))
        } else {
            // If not on disk, check if we have the URL for this emote and download from CDN
            match self.twitch_emotes.get(&name) {
                Some(id) => {
                    let url = format!("https://static-cdn.jtvnw.net/emoticons/v1/{}/3.0", id);
                    let mut res = reqwest::get(&url)?;
                    let mut bytes: Vec<u8> = Vec::new();
                    res.copy_to(&mut bytes)?;
                    Ok(Some(Emote {
                        path: PathBuf::new(),
                        file_name: format!("{}.png", name),
                        name,
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
