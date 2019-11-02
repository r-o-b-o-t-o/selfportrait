mod data;
mod index;
mod library;
mod palette;

use std::sync::Arc;

use crate::{
    Result,
    EmoteManager,
    config::WwwConfig,
};
use data::Data;

use actix_web::{ middleware, App, HttpServer };

pub fn start(config: &WwwConfig, emote_mngr: Arc<EmoteManager>) -> Result<()> {
    HttpServer::new(move || {
        App::new()
            .data(Data {
                emote_mngr: emote_mngr.clone(),
            })
            .wrap(middleware::Logger::default())
            .service(actix_files::Files::new("/assets", "assets"))
            .service(index::handler)
            .service(library::handler)
            .service(palette::handler)
    })
    .workers(config.workers)
    .bind(format!("{}:{}", config.bind_host, config.bind_port))?
    .run()?;

    Ok(())
}
