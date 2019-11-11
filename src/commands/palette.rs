use super::*;

use crate::{
    config::Config,
    error::{ Error, ErrorKind },
};

pub struct Palette {
    names: Vec<&'static str>,
}

impl Default for Palette {
    fn default() -> Self {
        Self {
            names: vec![ "palette", "emotes", "list", "ls" ],
        }
    }
}

impl Palette {
    pub fn boxed() -> Box<Self> {
        Box::new(Self::default())
    }
}

impl Command for Palette {
    fn names(&self) -> &[&'static str] {
        &self.names
    }

    fn handle_message(&self, bot: &Bot, ctx: &Context, msg: &Option<&mut Message>, event: &Option<&MessageUpdateEvent>) -> Result<()> {
        let data = ctx.data.read();
        let config = data.get::<Config>().ok_or_else(|| Error::new(ErrorKind::DataGet))?;
        let url = format!("{}/palette", config.www.base_url);
        bot.send_message(ctx, &msg, event, |m| m.content(&url))?;
        Ok(())
    }
}
