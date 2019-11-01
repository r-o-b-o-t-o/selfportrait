pub mod bot;
pub mod error;
pub mod config;
pub mod commands;
pub mod emote_manager;

use std::{
    thread,
    sync::{
        Arc, Mutex,
        atomic::{ AtomicBool, Ordering },
    },
    path::PathBuf,
};

use emote_manager::EmoteManager;
use config::{ Config, LoggingConfig };
use error::{ Error, ErrorKind, Result };

fn load_emotes() -> Result<EmoteManager> {
    log::info!("Loading emotes...");
    let emotes_path = PathBuf::from("assets");
    let mngr = EmoteManager::new(&emotes_path)?;
    log::info!("Loaded {} emote assets.", mngr.n_emotes());
    Ok(mngr)
}

fn setup_ctrl_c(running: Arc<AtomicBool>) -> Result<()> {
    ctrlc::set_handler(move || {
        running.store(false, Ordering::SeqCst);
    }).map_err(|err| Error::from(ErrorKind::CtrlCHandler, err))
}

fn setup_logging(config: &LoggingConfig) -> Result<()> {
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

fn main() -> Result<()> {
    let config = Config::load()?;
    setup_logging(&config.logging)?;
    let running = Arc::new(AtomicBool::new(true));
    setup_ctrl_c(running.clone())?;

    let config_shared = Arc::new(Mutex::new(config.clone()));
    let emote_mngr = Arc::new(load_emotes()?);

    log::info!("Starting bots...");
    for user in config.users {
        if !user.active {
            continue;
        }
        let config_shared = config_shared.clone();
        let emote_mngr = emote_mngr.clone();
        thread::spawn(move || {
            let user_id = user.discord_id;
            if let Err(err) = bot::Bot::start(user, config_shared, emote_mngr) {
                log::error!("Error while starting bot for user {}: {}", user_id, err);
            }
        });
    }

    let loop_sleep = std::time::Duration::from_millis(100);
    while running.load(Ordering::SeqCst) {
        std::thread::sleep(loop_sleep);
    }
    log::info!("Shutting down.");

    Ok(())
}
