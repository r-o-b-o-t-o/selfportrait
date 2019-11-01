pub mod user;
pub use user::User;

use std::{
    collections::HashMap,
    sync::{ Arc, Mutex },
};

use crate::{
    config::Config,
    emote_manager::EmoteManager,
    error::{ Error, ErrorKind, Result },
};

use serenity::{
    prelude::*,
    framework::standard::StandardFramework,
    model::{ channel::Message, gateway::Ready },
};

pub struct Bot {
    pub user: User,
    commands: HashMap<String, fn(&Bot, &Context, &Message) -> Result<()>>,
}

impl Bot {
    pub fn new(user: User) -> Self {
        let mut bot = Self {
            user,
            commands: HashMap::new(),
        };
        bot.commands.insert("palette".into(), crate::commands::palette);

        bot
    }
}

impl EventHandler for Bot {
    fn message(&self, ctx: Context, msg: Message) {
        if self.user.discord_id != msg.author.id.0 {
            // Respond to messages sent only by the user themselves
            return;
        }

        if let Err(err) = self.handle_message(ctx, msg) {
            log::error!("Error while handling message: {}", err);
        }
    }

    fn ready(&self, _ctx: Context, ready: Ready) {
        log::info!("{} is connected!", ready.user.name);
    }
}

impl Bot {
    fn handle_message(&self, ctx: Context, mut msg: Message) -> Result<()> {
        let mut delete_message = false;

        self.handle_text_emotes(&ctx, &mut msg)?;
        if self.handle_emotes(&ctx, &mut msg)? {
            delete_message = true;
        }
        if !delete_message && self.handle_commands(&ctx, &msg)? {
            delete_message = true;
        }

        if delete_message {
            msg.channel_id.delete_message(&ctx.http, msg)?;
        }
        Ok(())
    }

    fn handle_emotes(&self, ctx: &Context, msg: &mut Message) -> Result<bool> {
        let prefix = &self.user.emote_prefix;

        if !msg.content.contains(prefix) {
            return Ok(false);
        }

        let mut content_text = String::new();
        let mut delete_message = false;
        let data = ctx.data.read();
        let mngr = data.get::<EmoteManager>().ok_or(Error::new(ErrorKind::DataGet))?;

        for word in msg.content.split_whitespace() {
            if word.starts_with(prefix) {
                let emote_name = word.trim_start_matches(prefix);
                if let Some(emote) = mngr.find_emote_by_name(emote_name) {
                    msg.channel_id.send_files(&ctx.http, vec![emote.as_attachment()], |m| m.content(&content_text))?;
                    content_text = "".into();
                    delete_message = true;
                }
            } else {
                content_text.push_str(word);
                content_text.push_str(" ");
            }
        }

        Ok(delete_message)
    }

    fn handle_text_emotes(&self, ctx: &Context, msg: &mut Message) -> Result<()> {
        if !msg.content.contains(&self.user.text_emote_prefix) {
            return Ok(());
        }

        self.replace_text_emote(ctx, msg, &["lf", "lennyface", "lenny"], "( ͡° ͜ʖ ͡°)")?;
        self.replace_text_emote(ctx, msg, &["shrug", "s"], "¯\\\\\\_(ツ)\\_/¯")?;
        Ok(())
    }

    fn replace_text_emote(&self, ctx: &Context, msg: &mut Message, triggers: &[&str], emote: &str) -> Result<()> {
        let prefix = &self.user.text_emote_prefix;

        if triggers.iter().any(|trigger| msg.content.contains(&format!("{}{}", prefix, trigger))) {
            let mut edited = msg.content.clone();
            for trigger in triggers {
                edited = edited.replace(&format!("{}{}", prefix, trigger), emote);
            }

            msg.edit(ctx, |m| m.content(edited))?;
        }
        Ok(())
    }

    fn handle_commands(&self, ctx: &Context, msg: &Message) -> Result<bool> {
        let prefix = &self.user.command_prefix;
        if !msg.content.starts_with(prefix) {
            return Ok(false);
        }
        for (command_name, func) in self.commands.iter() {
            if msg.content.starts_with(&format!("{}{}", prefix, command_name)) {
                func(self, ctx, msg)?;
                return Ok(true);
            }
        }
        Ok(false)
    }

    pub fn start(user: User, config: Arc<Mutex<Config>>, emotes_mngr: Arc<EmoteManager>) -> Result<()> {
        let bot = Bot::new(user.clone());
        let mut client = Client::new(&user.token, bot)?;
        client.with_framework(StandardFramework::new()
            .configure(|c| c
                .allow_dm(false)
                .with_whitespace(true)
                .prefix(&user.command_prefix)
            )
        );
        {
            let mut data = client.data.write();
            data.insert::<Config>(config);
            data.insert::<EmoteManager>(emotes_mngr);
        }

        client.start()?;
        Ok(())
    }
}
