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
    http::AttachmentType,
    framework::standard::StandardFramework,
    builder::{ EditMessage, CreateMessage },
    model::{ channel::Message, gateway::Ready, event::MessageUpdateEvent },
};

pub struct Bot {
    pub user: User,
    commands: HashMap<String, fn(&Bot, &Context) -> Result<()>>,
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
            // Respond only to messages sent by the user themselves
            return;
        }

        if let Err(err) = self.handle_message(ctx, Some(msg), None) {
            log::error!("Error while handling message: {}", err);
        }
    }

    fn message_update(&self, ctx: Context, _old: Option<Message>, _new: Option<Message>, event: MessageUpdateEvent) {
        if let Some(author) = &event.author {
            if author.id != self.user.discord_id {
                // Update only messages sent by the user themselves
                return;
            }

            if let Err(err) = self.handle_message(ctx, None, Some(&event)) {
                log::error!("Error while handling message update: {}", err);
            }
        }
    }

    fn ready(&self, _ctx: Context, ready: Ready) {
        log::info!("{} is connected!", ready.user.name);
    }
}

impl Bot {
    fn handle_message(&self, ctx: Context, mut msg: Option<Message>, event: Option<&MessageUpdateEvent>) -> Result<()> {
        let mut delete_message = false;

        self.handle_text_emotes(&ctx, msg.as_mut(), event)?;
        if self.handle_emotes(&ctx, msg.as_mut(), event)? {
            delete_message = true;
        }
        if !delete_message && self.handle_commands(&ctx, msg.as_mut(), event)? {
            delete_message = true;
        }

        if delete_message {
            self.delete_message(&ctx, &msg, event)?;
        }
        Ok(())
    }

    fn handle_emotes(&self, ctx: &Context, msg: Option<&mut Message>, event: Option<&MessageUpdateEvent>) -> Result<bool> {
        let prefix = &self.user.emote_prefix;
        let content = self.message_content(&msg, event);

        if !content.contains(prefix) {
            return Ok(false);
        }

        let mut content_text = String::new();
        let mut delete_message = false;
        let data = ctx.data.read();
        let mngr = data.get::<EmoteManager>().ok_or(Error::new(ErrorKind::DataGet))?;

        for word in content.split_whitespace() {
            if word.starts_with(prefix) {
                let emote_name = word.trim_start_matches(prefix);
                if let Some(emote) = mngr.find_emote_by_name(emote_name) {
                    self.send_files(ctx, &msg, event, vec![emote.as_attachment()], |m| m.content(&content_text))?;
                    content_text = "".into();
                    delete_message = true;
                }
            } else {
                content_text.push_str(word);
                content_text.push_str(" ");
            }
        }
        if !content_text.is_empty() {
            self.send_message(ctx, &msg, event, |m| m.content(&content_text))?;
        }

        Ok(delete_message)
    }

    fn handle_text_emotes(&self, ctx: &Context, mut msg: Option<&mut Message>, event: Option<&MessageUpdateEvent>) -> Result<()> {
        let prefix = &self.user.text_emote_prefix;
        if !self.message_content(&msg, event).contains(prefix) {
            return Ok(());
        }

        let data = ctx.data.read();
        let mngr = data.get::<EmoteManager>().ok_or(Error::new(ErrorKind::DataGet))?;
        let mut content = self.message_content(&msg, event);

        for (triggers, emote) in mngr.text_emotes() {
            for trigger in triggers {
                content = content.replace(&format!("{}{}", prefix, trigger), emote);
            }
        }
        self.edit_message(ctx, &mut msg, event, |m| m.content(content))?;

        Ok(())
    }

    fn handle_commands(&self, ctx: &Context, msg: Option<&mut Message>, event: Option<&MessageUpdateEvent>) -> Result<bool> {
        let prefix = &self.user.command_prefix;
        let content = self.message_content(&msg, event);

        if !content.starts_with(prefix) {
            return Ok(false);
        }
        for (command_name, func) in self.commands.iter() {
            if content.starts_with(&format!("{}{}", prefix, command_name)) {
                func(self, ctx)?;
                return Ok(true);
            }
        }
        Ok(false)
    }

    fn edit_message<F>(&self, ctx: &Context, msg: &mut Option<&mut Message>, event: Option<&MessageUpdateEvent>, f: F) -> serenity::Result<()>
    where F: FnOnce(&mut EditMessage) -> &mut EditMessage {

        if let Some(msg) = msg {
            return msg.edit(ctx, f);
        } else if let Some(event) = event {
            event.channel_id.edit_message(ctx, event.id, f)?;
        }
        Ok(())
    }

    fn delete_message(&self, ctx: &Context, msg: &Option<Message>, event: Option<&MessageUpdateEvent>) -> serenity::Result<()> {
        if let Some(msg) = msg {
            return msg.channel_id.delete_message(&ctx, msg);
        } else if let Some(event) = event {
            return event.channel_id.delete_message(&ctx, event.id);
        }
        Ok(())
    }

    fn message_content(&self, msg: &Option<&mut Message>, event: Option<&MessageUpdateEvent>) -> String {
        if let Some(msg) = msg {
            return msg.content.clone();
        } else if let Some(event) = event {
            if let Some(content) = &event.content {
                return content.clone();
            }
        }
        "".into()
    }

    fn send_files<'a, It, T, F>(&self, ctx: &Context, msg: &Option<&mut Message>, event: Option<&MessageUpdateEvent>, files: It, f: F) -> serenity::Result<Option<Message>>
    where T: Into<AttachmentType<'a>>,
            It: IntoIterator<Item = T>,
            for <'b> F: FnOnce(&'b mut CreateMessage<'a>) -> &'b mut CreateMessage<'a> {

        if let Some(msg) = msg {
            return Ok(Some(msg.channel_id.send_files(ctx, files, f)?));
        } else if let Some(event) = event {
            return Ok(Some(event.channel_id.send_files(ctx, files, f)?));
        }
        Ok(None)
    }

    fn send_message<'a, F>(&self, ctx: &Context, msg: &Option<&mut Message>, event: Option<&MessageUpdateEvent>, f: F) -> serenity::Result<Option<Message>>
    where for <'b> F: FnOnce(&'b mut CreateMessage<'a>) -> &'b mut CreateMessage<'a> {

        if let Some(msg) = msg {
            return Ok(Some(msg.channel_id.send_message(ctx, f)?));
        } else if let Some(event) = event {
            return Ok(Some(event.channel_id.send_message(ctx, f)?));
        }
        Ok(None)
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
