pub mod user;
pub use user::User;

use std::{
    sync::Arc,
    collections::HashMap,
};

use crate::{
    config::Config,
    error::{ Error, ErrorKind, Result },
    emote_manager::{ Emote, EmoteManager },
};

use serenity::{
    prelude::*,
    http::AttachmentType,
    framework::standard::StandardFramework,
    builder::{ EditMessage, CreateMessage },
    model::{ channel::Message, gateway::Ready, event::MessageUpdateEvent },
};

type CommandFn = fn(&Bot, &Context, Option<&mut Message>, Option<&MessageUpdateEvent>) -> Result<()>;

pub struct Bot {
    pub user: User,
    commands: HashMap<String, CommandFn>,
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

    fn ready(&self, ctx: Context, ready: Ready) {
        ctx.invisible();
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

    fn handle_emotes(&self, ctx: &Context, mut msg: Option<&mut Message>, event: Option<&MessageUpdateEvent>) -> Result<bool> {
        let prefix = &self.user.emote_prefix;
        let content = self.message_content(&msg, event);

        if !content.contains(prefix) || prefix.is_empty() {
            return Ok(false);
        }

        let re = format!(r"(^|\s+){}(?P<emote>\w*)", prefix);
        let re = regex::Regex::new(&re).unwrap();

        let splits: Vec<_> = re.split(&content).collect();
        let captures: Vec<_> = re.captures_iter(&content).collect();
        let n_splits = splits.len();
        let n_captures = captures.len();

        if n_captures == 0 || captures.iter().all(|cap| cap["emote"].trim().is_empty()) {
            return Ok(false);
        }

        let data = ctx.data.read();
        let mngr = data.get::<EmoteManager>().ok_or_else(|| Error::new(ErrorKind::DataGet))?;

        struct EmoteMessage<'a> {
            pub content: String,
            pub capture: String,
            pub whitespace: String,
            pub emote: Option<&'a Emote>,
        }

        let mut messages = splits
                            .iter().zip(captures.iter())
                            .map(|(split, capture)| EmoteMessage {
                                content: split.to_string(),
                                capture: capture[0].to_owned(),
                                whitespace: capture[1].to_owned(),
                                emote: mngr.find_emote_by_name(&capture[2]),
                            })
                            .collect::<Vec<_>>();
        if n_splits > n_captures {
            // If there is text after the last emote, add the last bit of text that was not taken by the .iter().zip()
            messages.push(EmoteMessage {
                content: splits[n_splits - 1].to_string(),
                capture: String::new(),
                whitespace: String::new(),
                emote: None,
            });
        }

        let mut content = String::new();
        let mut first = true;
        let mut delete = true;
        for emote_msg in messages.iter() {
            content.push_str(&emote_msg.content);

            if let Some(emote) = emote_msg.emote {
                content.push_str(&emote_msg.whitespace);

                if first {
                    self.edit_message(ctx, &mut msg, event, |m| m.content(&content))?;
                    if !content.trim().is_empty() {
                        delete = false;
                    }
                    first = false;
                    content.clear();
                }
                self.send_files(ctx, &msg, event, vec![emote.as_attachment()], |m| m.content(&content))?;
                content.clear();
            } else {
                content.push_str(&emote_msg.capture);
            }
        }
        if !content.trim().is_empty() {
            self.send_message(ctx, &msg, event, |m| m.content(&content))?;
        }

        Ok(delete && !self.message_has_attachments(&msg, event))
    }

    fn handle_text_emotes(&self, ctx: &Context, mut msg: Option<&mut Message>, event: Option<&MessageUpdateEvent>) -> Result<()> {
        let prefix = &self.user.text_emote_prefix;
        if !self.message_content(&msg, event).contains(prefix) || prefix.is_empty() {
            return Ok(());
        }

        let data = ctx.data.read();
        let mngr = data.get::<EmoteManager>().ok_or_else(|| Error::new(ErrorKind::DataGet))?;
        let content = self.message_content(&msg, event);
        let mut edited = content.clone();

        for (triggers, emote) in mngr.text_emotes() {
            for trigger in triggers {
                edited = edited.replace(&format!("{}{}", prefix, trigger), emote);
            }
        }
        if edited != content {
            self.edit_message(ctx, &mut msg, event, |m| m.content(edited))?;
        }

        Ok(())
    }

    fn handle_commands(&self, ctx: &Context, msg: Option<&mut Message>, event: Option<&MessageUpdateEvent>) -> Result<bool> {
        let prefix = &self.user.command_prefix;
        let content = self.message_content(&msg, event);

        if !content.starts_with(prefix) || prefix.is_empty() {
            return Ok(false);
        }
        for (command_name, func) in self.commands.iter() {
            if content.starts_with(&format!("{}{}", prefix, command_name)) {
                func(self, ctx, msg, event)?;
                return Ok(true);
            }
        }
        Ok(false)
    }

    pub fn edit_message<F>(&self, ctx: &Context, msg: &mut Option<&mut Message>, event: Option<&MessageUpdateEvent>, f: F) -> serenity::Result<()>
    where F: FnOnce(&mut EditMessage) -> &mut EditMessage {

        if let Some(msg) = msg {
            return msg.edit(ctx, f);
        } else if let Some(event) = event {
            event.channel_id.edit_message(ctx, event.id, f)?;
        }
        Ok(())
    }

    pub fn delete_message(&self, ctx: &Context, msg: &Option<Message>, event: Option<&MessageUpdateEvent>) -> serenity::Result<()> {
        if let Some(msg) = msg {
            return msg.channel_id.delete_message(&ctx, msg);
        } else if let Some(event) = event {
            return event.channel_id.delete_message(&ctx, event.id);
        }
        Ok(())
    }

    pub fn message_content(&self, msg: &Option<&mut Message>, event: Option<&MessageUpdateEvent>) -> String {
        if let Some(msg) = msg {
            return msg.content.clone();
        } else if let Some(event) = event {
            if let Some(content) = &event.content {
                return content.clone();
            }
        }
        "".into()
    }

    pub fn send_files<'a, It, T, F>(&self, ctx: &Context, msg: &Option<&mut Message>, event: Option<&MessageUpdateEvent>, files: It, f: F) -> serenity::Result<Option<Message>>
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

    pub fn send_message<'a, F>(&self, ctx: &Context, msg: &Option<&mut Message>, event: Option<&MessageUpdateEvent>, f: F) -> serenity::Result<Option<Message>>
    where for <'b> F: FnOnce(&'b mut CreateMessage<'a>) -> &'b mut CreateMessage<'a> {

        if let Some(msg) = msg {
            return Ok(Some(msg.channel_id.send_message(ctx, f)?));
        } else if let Some(event) = event {
            return Ok(Some(event.channel_id.send_message(ctx, f)?));
        }
        Ok(None)
    }

    pub fn message_has_attachments(&self, msg: &Option<&mut Message>, event: Option<&MessageUpdateEvent>) -> bool {
        if let Some(msg) = msg {
            !msg.attachments.is_empty()
        } else if let Some(event) = event {
            match &event.attachments {
                Some(attachments) => !attachments.is_empty(),
                None => false,
            }
        } else {
            false
        }
    }

    pub fn start(user: User, config: Arc<Config>, emotes_mngr: Arc<EmoteManager>) -> Result<Client> {
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
        Ok(client)
    }
}
