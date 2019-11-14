use std::{
    thread,
    io::BufWriter,
    path::PathBuf,
    fs::{ self, File },
    sync::{ Arc, RwLock },
    time::{ Instant, Duration },
};

use crate::Result;

use reqwest::Client;
use indicatif::{ ProgressBar, ProgressStyle };

#[derive(serde::Deserialize)]
struct TwitchResponse {
    pub _links: TwitchLinks,
    pub emoticons: Vec<TwitchEmote>,
}

#[derive(serde::Deserialize)]
struct TwitchLinks {
    #[serde(rename(deserialize = "self"))]
    pub link_self: String,
}

#[derive(serde::Deserialize)]
struct TwitchEmote {
    pub id: u64,
    pub regex: String,
    pub images: TwitchImage,
}

#[derive(serde::Deserialize)]
struct TwitchImage {
    pub width: u32,
    pub height: u32,
    pub url: String,
    pub emoticon_set: u64,
}

#[derive(serde::Deserialize)]
struct TwitchErrorResponse {
    pub error: String,
    pub status: u16,
    pub message: String,
}

fn get_list_from_api(client: &Client) -> Result<String> {
    let mut res = client.get("https://api.twitch.tv/kraken/chat/emoticons")
                        .send()?;

    let text = res.text()?;
    if !res.status().is_success() {
        let res = serde_json::from_str::<TwitchErrorResponse>(&text);
        match res {
            Ok(res) => log::error!("API {} error (status {}): {}", res.error, res.status, res.message),
            Err(_err) => log::error!("API error: {}", text),
        };
        std::process::exit(0);
    }
    Ok(text)
}

fn _get_list_from_file() -> Result<String> {
    let text = fs::read_to_string("twitch_emotes.json")?;
    Ok(text)
}

fn save_emote(client: &Client, path: &str, url: &str) -> Result<()> {
    let f = File::create(path)?;
    let mut buf = BufWriter::new(f);
    let mut res = client.get(url).send()?;
    res.copy_to(&mut buf)?;

    Ok(())
}

pub fn run() -> Result<()> {
    let client = Client::new();

    log::info!("Loading URLs...");
    let text = get_list_from_api(&client)?;
    let twitch_emotes: TwitchResponse = serde_json::from_str(&text)?;
    let n_emotes = twitch_emotes.emoticons.len();
    log::info!("{} twitch emotes available.", n_emotes);

    let dir = "assets/twitchemotes";
    if PathBuf::from(dir).exists() {
        fs::remove_dir_all(dir)?;
    }
    fs::create_dir_all(dir)?;

    log::info!("Starting download...");
    let pb = ProgressBar::new(n_emotes as u64);
    pb.set_style(ProgressStyle::default_bar()
                    .template("[{elapsed_precise}] [{bar:40.cyan/blue}] {pos}/{len} ({percent}%, {eta} remaining)"));
    let n_threads = 8;
    let client = Arc::new(client);
    let pb = Arc::new(RwLock::new(pb));
    let twitch_emotes = Arc::new(twitch_emotes);

    let threads = (0..n_threads).map(|thread_id| {
        let pb = pb.clone();
        let client = client.clone();
        let twitch_emotes = twitch_emotes.clone();

        thread::spawn(move || {
            let mut i = 0;
            let mut last_progress_refresh = Instant::now();
            let progress_step = Duration::from_millis(1_000);

            for (emote_idx, emote) in twitch_emotes.emoticons.iter().enumerate() {
                if emote_idx % n_threads == thread_id {
                    let url = &emote.images.url;
                    let url = url.replace("1.0", "3.0");
                    let path = format!("{}/{}.png", dir, emote.regex).to_lowercase();
                    let _ = save_emote(&client, &path, &url);

                    i += 1;
                    let now = Instant::now();
                    if now - last_progress_refresh >= progress_step {
                        pb.write().unwrap().inc(i);
                        i = 0;
                        last_progress_refresh = now;
                    }
                }
            }
        })
    }).collect::<Vec<_>>();
    for thread in threads {
        thread.join().unwrap();
    }
    pb.write().unwrap().finish_with_message("done");

    Ok(())
}
