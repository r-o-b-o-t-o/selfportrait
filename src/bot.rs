use std::sync::{ Arc, Mutex };

use crate::{
    user::User,
    config::Config,
    emote_manager::EmoteManager,
};

use serenity::{
    prelude::*,
    model::{ channel::Message, gateway::Ready },
};

pub struct Bot {
    user: User,
}

impl Bot {
    pub fn new(user: User) -> Self {
        Self {
            user,
        }
    }
}

impl EventHandler for Bot {
    fn message(&self, ctx: Context, mut msg: Message) {
        if self.user.discord_id != msg.author.id.0 {
            // Respond to messages sent only by the user themselves
            return;
        }

        self.handle_text_emotes(&ctx, &mut msg);
        if self.handle_emotes(&ctx, &mut msg) {
            let _ = msg.channel_id.delete_message(&ctx.http, msg);
        }
    }

    fn ready(&self, _ctx: Context, ready: Ready) {
        log::info!("{} is connected!", ready.user.name);
    }
}

impl Bot {
    fn handle_emotes(&self, ctx: &Context, msg: &mut Message) -> bool {
        let prefix = &self.user.emote_prefix;

        if !msg.content.contains(prefix) {
            return false;
        }

        let mut content_text = String::new();
        let mut delete_message = false;
        let data = ctx.data.read();
        let mngr = match data.get::<EmoteManager>() {
            Some(mngr) => mngr,
            None => {
                log::error!("Could not get emote manager");
                return false;
            }
        };

        for word in msg.content.split_whitespace() {
            if word.starts_with(prefix) {
                let emote_name = word.trim_start_matches(prefix);
                if let Some(emote) = mngr.find_emote_by_name(emote_name) {
                    match msg.channel_id.send_files(&ctx.http, vec![emote.as_attachment()], |m| m.content(&content_text)) {
                        Ok(_) => {
                            content_text = "".into();
                            delete_message = true;
                        },
                        Err(err) => log::error!("Could not send emote: {}", err),
                    };
                }
            } else {
                content_text.push_str(word);
                content_text.push_str(" ");
            }
        }

        delete_message
    }

    fn handle_text_emotes(&self, ctx: &Context, msg: &mut Message) {
        if !msg.content.contains(&self.user.text_emote_prefix) {
            return;
        }

        self.replace_text_emote(ctx, msg, &["lf", "lenny", "lennyface"], "( ͡° ͜ʖ ͡°)");
        self.replace_text_emote(ctx, msg, &["s", "shrug"], "¯\\\\\\_(ツ)\\_/¯");
    }

    fn replace_text_emote(&self, ctx: &Context, msg: &mut Message, triggers: &[&str], emote: &str) {
        let prefix = &self.user.text_emote_prefix;

        if triggers.iter().any(|trigger| msg.content.contains(&format!("{}{}", prefix, trigger))) {
            let mut edited = msg.content.clone();
            for trigger in triggers {
                edited = edited.replace(&format!("{}{}", prefix, trigger), emote);
            }

            let _ = msg.edit(ctx, |m| m.content(edited));
        }
    }

    pub fn start(user: User, config: Arc<Mutex<Config>>, emotes_mngr: Arc<EmoteManager>) -> serenity::Result<()> {
        let bot = Bot::new(user);
        let mut client = Client::new(bot.user.token.clone(), bot)?;
        {
            let mut data = client.data.write();
            data.insert::<Config>(config);
            data.insert::<EmoteManager>(emotes_mngr);
        }

        client.start()?;
        Ok(())
    }
}
