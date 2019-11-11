use std::collections::HashSet;

#[derive(Debug, Default, Clone, PartialEq, serde::Deserialize)]
pub struct User {
    pub active: bool,
    pub discord_id: u64,
    pub token: String,
    pub command_prefix: String,
    pub emote_prefix: String,
    pub text_emote_prefix: String,
}

pub struct UserSettingsKey;

impl typemap::Key for UserSettingsKey {
    type Value = UserSettings;
}

#[derive(Debug, Default, Clone)]
pub struct UserSettings {
    pub spoiler_mode: HashSet<u64>, // Ids of the channels for which spoiler mode is enabled
}

impl UserSettings {
    pub fn spoiler_mode(&self, channel: u64) -> bool {
        self.spoiler_mode.contains(&channel)
    }
}
