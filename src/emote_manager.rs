use std::sync::Arc;
use std::io::Result;
use std::path::{ Path, PathBuf };

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
}

impl EmoteManager {
    pub fn new(assets_directory: &Path) -> Self {
        let mut mngr = Self {
            emotes: Vec::new(),
        };

        for dir in ["emojis", "gifs", "sounds"].iter() {
            let mut path = PathBuf::from(assets_directory);
            path.push(dir);
            if let Err(err) = mngr.load_emotes_in_dir(&path) {
                log::error!("Could not load emotes from directory {}: {}", path.display(), err);
            }
        }

        mngr
    }

    fn load_emotes_in_dir(&mut self, dir: &Path) -> Result<()> {
        for entry in dir.read_dir()? {
            let entry = entry?;
            let path = entry.path();
            if path.is_file() {
                self.emotes.push(Emote {
                    path: path.clone(),
                    file_name: path.file_name().unwrap().to_string_lossy().into(),
                    name: path.with_extension("").file_name().unwrap().to_string_lossy().into(),
                    bytes: std::fs::read(path)?,
                });
            }
        }
        Ok(())
    }

    pub fn find_emote_by_name(&self, name: &str) -> Option<&Emote> {
        self.emotes.iter().find(|emote| emote.name == name)
    }
}

impl Key for EmoteManager {
    type Value = Arc<EmoteManager>;
}
