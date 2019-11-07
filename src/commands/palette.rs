use crate::{
    Error,
    Config,
    ErrorKind,
    error::Result,
    bot::Bot,
};

use serenity::{
    prelude::*,
    model::channel::Message,
    model::event::MessageUpdateEvent,
};

pub fn command(bot: &Bot, ctx: &Context, msg: Option<&mut Message>, event: Option<&MessageUpdateEvent>) -> Result<()> {
    let data = ctx.data.read();
    let config = data.get::<Config>().ok_or_else(|| Error::new(ErrorKind::DataGet))?;
    let url = format!("{}/palette", config.www.base_url);
    bot.send_message(ctx, &msg, event, |m| m.content(&url))?;
    Ok(())
}
