use std::{
    sync::Arc,
    path::{ Path, PathBuf },
};

use crate::{ Error, ErrorKind, Result };

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
}

impl EmoteManager {
    pub fn new(assets_directory: &Path) -> Result<Self> {
        let mut mngr = Self {
            emotes: Vec::new(),
            text_emotes: vec![
                (vec!["lf", "lennyface", "lenny"], "( ͡° ͜ʖ ͡°)"),
                (vec!["shrug", "s"], "¯\\\\\\_(ツ)\\_/¯"),
            ],
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
                                    .ok_or(std::io::Error::new(std::io::ErrorKind::Other, "no file name"))?
                                    .to_string_lossy()
                                    .into();
                let name = path.with_extension("")
                                .file_name()
                                .ok_or(std::io::Error::new(std::io::ErrorKind::Other, "no file name"))?
                                .to_string_lossy()
                                .into();

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
        self.emotes.iter().find(|emote| emote.name == name)
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
