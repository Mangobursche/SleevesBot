use serenity::{
    async_trait,
    client::{Context, EventHandler},
    framework::StandardFramework,
    model::{guild::Member, id::GuildId, prelude::VoiceState},
    Client,
};
use songbird::{
    driver::{Config, DecodeMode},
    SerenityInit, Songbird,
};

mod audio;
mod general;
mod penis;

pub struct Handler;

#[async_trait]
impl EventHandler for Handler {
    async fn voice_state_update(
        &self,
        ctx: Context,
        guild_id: Option<GuildId>,
        old: Option<VoiceState>,
        new: VoiceState,
    ) {
        audio::voice_state_update(&ctx, &guild_id, &old, &new).await;
    }

    async fn guild_member_addition(&self, ctx: Context, _: GuildId, new_member: Member) {
        penis::guild_member_addition(&ctx, &new_member).await;
    }
}

#[tokio::main]
async fn main() -> Result<(), serenity::Error> {
    let token = "<your-token>";

    let framework = StandardFramework::new()
        .configure(|c| c.prefix("!"))
        .group(&general::GENERAL_GROUP)
        .group(&audio::RECORD_GROUP)
        .group(&penis::PENIS_GROUP)
        .help(&general::HELP);

    let songbird = Songbird::serenity();
    songbird.set_config(Config::default().decode_mode(DecodeMode::Decode));

    let mut client = Client::builder(&token)
        .event_handler(Handler)
        .framework(framework)
        .register_songbird_with(songbird)
        .await?;

    audio::config(&client).await;

    client.start().await
}
