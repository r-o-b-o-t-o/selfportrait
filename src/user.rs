#[derive(Debug, Default, Clone, PartialEq, serde::Deserialize)]
pub struct User {
    pub active: bool,
    pub palette: bool,
    pub discord_id: u64,
    pub token: String,
    pub command_prefix: String,
    pub emote_prefix: String,
    pub text_emote_prefix: String,
}
