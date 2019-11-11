use crate::{ bot::Bot, error::Result };

use serenity::{
    prelude::*,
    model::channel::Message,
    model::event::MessageUpdateEvent,
};

pub mod palette;
pub use palette::Palette;

pub mod spoiler;
pub use spoiler::Spoiler;

pub trait Command {
    fn names(&self) -> &[&'static str];
    fn handle_message(&self, bot: &Bot, ctx: &Context, msg: &Option<&mut Message>, event: &Option<&MessageUpdateEvent>) -> Result<()>;
}
