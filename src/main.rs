pub mod bot;
pub mod user;
pub mod config;
pub mod emote_manager;

use config::Config;
use emote_manager::EmoteManager;

use std::sync::{
    Arc, Mutex,
    atomic::{ AtomicBool, Ordering },
};
use std::path::PathBuf;

fn load_config() -> Config {
    let config_str = std::fs::read_to_string("config.toml").expect("Could not read the configuration file config.toml");
    toml::from_str(&config_str).expect("Could not parse the configuration")
}

fn load_emotes() -> EmoteManager {
    log::info!("Loading emotes...");
    let emotes_path = PathBuf::from("assets");
    EmoteManager::new(&emotes_path)
}

fn setup_ctrl_c(running: Arc<AtomicBool>) {
    ctrlc::set_handler(move || {
        running.store(false, Ordering::SeqCst);
    }).expect("Could not set Ctrl-C handler");
}

fn setup_logging(config: &Config) {
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
        .level(config.log_level)
        .chain(std::io::stdout())
        .chain(fern::log_file(&config.log_file).expect(&format!("Could not open log file {}", config.log_file.display())))
        .apply().expect("Could not setup logging");
}

fn main() {
    let config = load_config();
    setup_logging(&config);
    let running = Arc::new(AtomicBool::new(true));
    setup_ctrl_c(running.clone());

    let config_shared = Arc::new(Mutex::new(config.clone()));
    let emote_mngr = Arc::new(load_emotes());

    log::info!("Starting bots...");
    for user in config.users {
        let config_shared = config_shared.clone();
        let emote_mngr = emote_mngr.clone();
        std::thread::spawn(move || {
            let _ = bot::Bot::start(user, config_shared, emote_mngr);
        });
    }

    let loop_sleep = std::time::Duration::from_millis(100);
    while running.load(Ordering::SeqCst) {
        std::thread::sleep(loop_sleep);
    }
}
