use std::{
    ffi::OsString,
    path::PathBuf,
    collections::HashMap,
};

use super::Data;

use serde::Serialize;
use actix_web::{ web, HttpRequest, HttpResponse };

#[derive(Serialize, Default)]
struct Library(pub Vec<List>);

impl Library {
    pub fn get_list_for_type(&mut self, t: &str) -> Option<&mut List> {
        self.0.iter_mut().find(|list| list.type_name == t)
    }
}

#[derive(Serialize)]
struct List {
    pub type_name: String,
    pub emotes: Vec<Emote>,
}

#[derive(Serialize)]
struct Emote {
    pub name: String,
    pub url: String,
}

impl Emote {
    pub fn new(name: &str, path: &PathBuf) -> Self {
        let mut url = OsString::new();
        for component in path.components() {
            url.push("/");
            url.push(component);
        }
        let url = url.to_string_lossy();

        Self {
            name: name.to_string(),
            url: url.to_string(),
        }
    }
}

#[get("/library")]
pub fn handler(_req: HttpRequest, data: web::Data<Data>) -> HttpResponse {
    let mut dir_to_type_bind = HashMap::new();
    dir_to_type_bind.insert("emojis".to_string(), "Emoji".to_string());
    dir_to_type_bind.insert("gifs".to_string(), "GIF".to_string());
    dir_to_type_bind.insert("sounds".to_string(), "Sound".to_string());

    let mut library = Library::default();
    for emote in data.emote_mngr.emotes() {
        let dir = match emote.path.parent() {
            Some(dir) => dir,
            None => continue,
        };
        let dir = match dir.file_name() {
            Some(dir) => dir,
            None => continue,
        };
        let dir = match dir.to_str() {
            Some(dir) => dir,
            None => continue,
        };
        let default_type = String::from("");
        let emote_type = dir_to_type_bind.get(dir).unwrap_or(&default_type);

        let serializable_emote = Emote::new(&emote.name, &emote.path);
        match library.get_list_for_type(emote_type) {
            Some(list) => list.emotes.push(serializable_emote),
            None => library.0.push(List {
                type_name: emote_type.clone(),
                emotes: vec![serializable_emote],
            }),
        };
    }

    HttpResponse::Ok().json(library)
}
