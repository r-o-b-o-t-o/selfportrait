use super::*;

use crate::{
    bot::UserSettingsKey,
    error::{ Error, ErrorKind },
};

pub struct Spoiler {
    names: Vec<&'static str>,
}

impl Default for Spoiler {
    fn default() -> Self {
        Self {
            names: vec![ "spoiler", "spoil", "spoilermode", "spoilmode", "sm" ],
        }
    }
}

impl Spoiler {
    pub fn boxed() -> Box<Self> {
        Box::new(Self::default())
    }

    pub fn spoilerize(s: &mut String) -> bool {
        let starts = s.starts_with("||");
        let ends = s.ends_with("||");

        if !starts || !ends {
            if !starts {
                *s = format!("|| {}", s.trim());
            }
            if !ends {
                *s = format!("{} ||", s.trim());
            }
            true
        } else {
            false
        }
    }
}

impl Command for Spoiler {
    fn names(&self) -> &[&'static str] {
        &self.names
    }

    fn handle_message(&self, bot: &Bot, ctx: &Context, msg: &Option<&mut Message>, event: &Option<&MessageUpdateEvent>) -> Result<()> {
        let mut data = ctx.data.write();
        let settings = data.get_mut::<UserSettingsKey>().ok_or_else(|| Error::new(ErrorKind::DataGet))?;
        let channel_id = bot.channel_id(msg, event);
        if settings.spoiler_mode.contains(&channel_id) {
            settings.spoiler_mode.remove(&channel_id);
        } else {
            settings.spoiler_mode.insert(channel_id);
        }

        Ok(())
    }
}
