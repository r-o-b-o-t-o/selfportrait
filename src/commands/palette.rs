use crate::{
    bot::Bot,
    error::Result,
};

use serenity::{
    prelude::*,
    model::channel::Message,
};

pub fn command(_bot: &Bot, _ctx: &Context, _msg: &Message) -> Result<()> {
    Ok(())
}
