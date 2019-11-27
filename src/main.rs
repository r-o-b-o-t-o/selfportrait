#[macro_use]
extern crate actix_web;

pub mod bot;
pub mod www;
pub mod tools;
pub mod error;
pub mod config;
pub mod commands;
pub mod emote_manager;

use std::{
    thread,
    path::PathBuf,
    sync::{
        Arc,
        atomic::{ AtomicBool, Ordering },
    },
};

use config::Config;
pub use emote_manager::EmoteManager;
pub use error::{ Error, ErrorKind, Result };

fn load_emotes(config: &Config) -> Result<EmoteManager> {
    log::info!("Loading emotes...");
    let emotes_path = PathBuf::from("assets");
    let mngr = EmoteManager::new(config, &emotes_path)?;
    log::info!("Loaded {} emote assets.", mngr.n_emotes());
    Ok(mngr)
}

fn setup_ctrl_c(run: Arc<AtomicBool>) -> Result<()> {
    ctrlc::set_handler(move || {
        run.store(false, Ordering::SeqCst);
    })?;
    Ok(())
}

fn setup_logging(config: &config::LoggingConfig) -> Result<()> {
    fern::Dispatch::new()
        .format(|out, message, record| {
            out.finish(format_args!(
                "{}[{}][{}] {}",
                chrono::Local::now().format("[%Y-%m-%d %H:%M:%S]"),
                record.target(),
                record.level(),
                message,
            ))
        })
        .level(config.level)
        .chain(std::io::stdout())
        .chain(fern::log_file(&config.file).map_err(|err| Error::from(ErrorKind::LogFile, err))?)
        .apply().map_err(|err| Error::from(ErrorKind::Logging, err))
}

fn setup_heroku_wakeup_thread(url: String) {
    if let Ok(val) = std::env::var("HEROKU_PREVENT_SLEEP") {
        let val = val.parse::<i32>();
        match val {
            Ok(val) if val == 1 => {
                std::thread::spawn(move || {
                    loop {
                        let interval = std::time::Duration::from_secs(20 * 60); // Every 20 minutes
                        std::thread::sleep(interval);
                        let _ = reqwest::get(&url);
                    }
                });
            },
            _ => {},
        };
    }
}

fn start_www(config: Arc<Config>, emote_mngr: Arc<EmoteManager>) {
    thread::spawn(move || {
        log::info!("Starting web server...");
        let res = www::start(&config.www, emote_mngr);
        if let Err(err) = res {
            log::error!("Web server error: {}", err);
        }
    });
}

fn wait_loop(run: Arc<AtomicBool>) {
    let sleep_duration = std::time::Duration::from_millis(100);

    while run.load(Ordering::SeqCst) {
        std::thread::sleep(sleep_duration);
    }
}

fn main() -> Result<()> {
    let config = Config::load()?;
    setup_logging(&config.logging)?;

    for arg in std::env::args() {
        if arg == "--fetch-twitch-emotes" {
            return tools::fetch_twitch_emotes::run(&config);
        } else if arg == "--print-config" {
            return tools::print_config::run(&config);
        }
    }

    if config.www.enabled {
        setup_heroku_wakeup_thread(config.www.base_url.clone());
    }

    let users = config.users()
                        .into_iter()
                        .filter(|user| user.active)
                        .collect::<Vec<_>>();
    let emote_mngr = Arc::new(load_emotes(&config)?);
    let config = Arc::new(config);

    log::info!("Starting {} bot{}...", users.len(), if users.len() > 1 { "s" } else { "" });
    for user in users {
        let config = config.clone();
        let emote_mngr = emote_mngr.clone();
        thread::spawn(move || {
            let user_id = user.discord_id;
            if let Err(err) = bot::Bot::start(user, config, emote_mngr) {
                log::error!("Error while starting bot for user {}: {}", user_id, err);
            }
        });
    }

    if config.www.enabled {
        start_www(config.clone(), emote_mngr.clone());
    }

    let run = Arc::new(AtomicBool::new(true));
    setup_ctrl_c(run.clone())?;
    wait_loop(run.clone());

    log::info!("Shutting down.");
    Ok(())
}
