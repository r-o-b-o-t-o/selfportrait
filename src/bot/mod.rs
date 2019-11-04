pub mod user;
pub use user::User;

use std::{
    sync::Arc,
    collections::HashMap,
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
use unicode_segmentation::UnicodeSegmentation;

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
        let prefix_str = &self.user.emote_prefix;
        let content = self.message_content(&msg, event);

        if !content.contains(prefix_str) || prefix_str.is_empty() {
            return Ok(false);
        }

        let data = ctx.data.read();
        let mngr = data.get::<EmoteManager>().ok_or(Error::new(ErrorKind::DataGet))?;

        let content = UnicodeSegmentation::graphemes(&content[..], true).collect::<Vec<&str>>();
        let content_length = content.len();
        let prefix = UnicodeSegmentation::graphemes(&prefix_str[..], true).collect::<Vec<&str>>();

        let mut prefix_pos = 0;
        let mut prefix_found = false;
        let mut delete_message = false;

        let mut emote_name = String::new();
        let mut result = String::new();

        for (g_idx, &g) in content.iter().enumerate() {
            if prefix_found {
                // After having found the prefix, we look for an emote name that matches an emote in the EmoteManager
                let is_whitespace = g.trim().is_empty();
                let last_msg_char = g_idx + 1 == content_length;

                if !is_whitespace {
                    emote_name.push_str(g);
                }

                if last_msg_char || is_whitespace {
                    // Send the emote out if the emote is at the end of the message (last_msg_char)
                    // or if we encountered a whitespace (which marks the end of the emote name)

                    prefix_found = false; // We'll start looking for the emote prefix again
                    prefix_pos = 0;       // in the next loop iteration

                    if let Some(emote) = mngr.find_emote_by_name(&emote_name) {
                        // Send the emote with the part of the message we read before the emote name
                        self.send_files(ctx, &msg, event, vec![emote.as_attachment()], |m| m.content(&result))?;
                        emote_name.clear();
                        result.clear();
                        delete_message = true;
                    } else {
                        // If the emote doesn't exist it might be just regular text,
                        // we append it to the message content
                        result.push_str(prefix_str);
                    }
                    if is_whitespace {
                        result.push_str(g);
                    }
                }
            } else { // Search for the emote prefix
                let spacing_check = if prefix_pos == 0 { // If we're at the first character of the prefix search...
                    g_idx == 0 || content[g_idx - 1].trim().is_empty() // ... we need the emote name to be separated by a whitespace from the previous word
                } else {
                    true
                };
                if g == prefix[prefix_pos] && spacing_check {
                    prefix_pos += 1;
                    if prefix_pos == prefix.len() {
                        // We've found the emote prefix in its entirety
                        prefix_found = true;
                    }
                } else {
                    // Haven't found parts of the prefix, append the current grapheme to the result content
                    result.push_str(g);
                }
            }
        }
        if !result.is_empty() && delete_message {
            // If we're about to delete the message (since we split the content in multiple new messages)
            // and if there is text after the last emote, send the last bit in a new message
            self.send_message(ctx, &msg, event, |m| m.content(&result))?;
        }
        if delete_message && self.message_has_attachments(&msg, event) {
            // Prevents deleting a message if it has attachments
            // We just edit it to an empty string instead
            self.edit_message(ctx, &mut msg, event, |m| m.content(""))?;
            delete_message = false;
        }

        Ok(delete_message)
    }

    fn handle_text_emotes(&self, ctx: &Context, mut msg: Option<&mut Message>, event: Option<&MessageUpdateEvent>) -> Result<()> {
        let prefix = &self.user.text_emote_prefix;
        if !self.message_content(&msg, event).contains(prefix) || prefix.is_empty() {
            return Ok(());
        }

        let data = ctx.data.read();
        let mngr = data.get::<EmoteManager>().ok_or(Error::new(ErrorKind::DataGet))?;
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
