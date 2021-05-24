use serenity::{
    async_trait,
    client::Context,
    framework::standard::{
        macros::{command, group},
        Args, CommandResult,
    },
    futures::lock::Mutex,
    model::{
        channel::Message,
        id::{GuildId, UserId},
        misc::Mentionable,
        prelude::VoiceState,
    },
    prelude::TypeMapKey,
    Client,
};

use songbird::{CoreEvent, Event, EventContext, EventHandler};
use std::{collections::HashMap, fs::File, io::BufWriter, path::Path, sync::Arc, time::Duration};
use uuid::Uuid;

pub async fn config(client: &Client) {
    let mut data = client.data.write().await;
    data.insert::<Receivers>(Arc::new(Mutex::new(HashMap::new())));
}

pub async fn voice_state_update(
    ctx: &Context,
    guild_id: &Option<GuildId>,
    _: &Option<VoiceState>,
    new: &VoiceState,
) {
    if let Some(member) = new.member.clone() {
        if member.user.bot {
            return;
        }
    }

    let voice_manager = songbird::get(ctx).await.unwrap();

    if let Some(guild_id) = guild_id {
        if let Some(channel_id) = new.channel_id {
            if channel_id.0 == 723740962802630686 {
                let (handler_lock, _) = voice_manager.join(guild_id.0, channel_id).await;

                if let Ok(source) = songbird::ffmpeg("chew.wav").await {
                    let mut handler = handler_lock.lock().await;

                    handler.play_only_source(source);
                    return;
                }
            }
        }

        if let Some(guild) = guild_id.to_guild_cached(&ctx.cache).await {
            if guild.voice_states.len() == 1 {
                voice_manager.remove(guild_id.0).await.unwrap();
                return;
            }
        }
    }
}

#[group]
#[commands(join, leave, play, download, delete, record, stop, rename, records)]
#[required_permissions(ADMINISTRATOR)]
struct Record;

#[command]
#[only_in(guild)]
#[description = "Makes the bot join your voice channel"]
async fn join(ctx: &Context, msg: &Message) -> CommandResult {
    let guild = msg.guild(&ctx.cache).await.unwrap();

    if let Some(channel_id) = guild
        .voice_states
        .get(&msg.author.id)
        .and_then(|voice_state| voice_state.channel_id)
    {
        let guild_id = guild.id;

        let voice_manager = songbird::get(ctx).await.unwrap();

        let (handler_lock, res) = voice_manager.join(guild_id, channel_id).await;

        let receivers_lock = {
            let data = ctx.data.read().await;
            data.get::<Receivers>().unwrap().clone()
        };

        let mut receivers = receivers_lock.lock().await;

        let receiver = receivers.entry(guild_id.0).or_insert_with(Receiver::new);

        let mut handler = handler_lock.lock().await;
        handler.remove_all_global_events();
        handler.add_global_event(CoreEvent::SpeakingStateUpdate.into(), receiver.clone());
        handler.add_global_event(CoreEvent::VoicePacket.into(), receiver.clone());
        handler.add_global_event(CoreEvent::ClientConnect.into(), receiver.clone());

        let message = res.map_or(format!("Error joining {}", channel_id.mention()), |_| {
            format!("Successfully joined {}", channel_id.mention())
        });

        msg.reply(&ctx.http, message).await?;
    } else {
        msg.reply(
            &ctx.http,
            "You need to be in a voice channel to use this command",
        )
        .await?;
    }

    Ok(())
}

#[command]
#[only_in(guilds)]
#[description = "Makes the bot leave your voice channel"]
async fn leave(ctx: &Context, msg: &Message) -> CommandResult {
    let guild = msg.guild(&ctx.cache).await.unwrap();
    let guild_id = guild.id;

    let voice_manager = songbird::get(ctx).await.unwrap();

    if let Some(handler) = voice_manager.get(guild_id) {
        handler.lock().await.remove_all_global_events();

        voice_manager.remove(guild_id).await?;
    }

    Ok(())
}

#[command]
#[only_in(guild)]
#[description = "Plays the record"]
#[usage = "<record>"]
async fn play(ctx: &Context, msg: &Message, args: Args) -> CommandResult {
    let record = args.rest();

    let guild = msg.guild(&ctx.cache).await.unwrap();

    let voice_manager = songbird::get(ctx).await.unwrap();

    if let Some(handler_lock) = voice_manager.get(guild.id) {
        let mut handler = handler_lock.lock().await;

        let path = format!("records/{}.wav", record);
        if Path::new(&path).exists() {
            if let Ok(source) = songbird::ffmpeg(path).await {
                handler.play_only_source(source);

                msg.reply(&ctx.http, format!("Now playing `{}`", record))
                    .await?;
            }
        } else {
            msg.reply(&ctx.http, "This record doesn't exist").await?;
        }
    } else {
        msg.reply(&ctx.http, "Use `join` first").await?;
    }

    Ok(())
}

#[command]
#[only_in(guild)]
#[description = "Downloads the record"]
#[usage = "<record>"]
async fn download(ctx: &Context, msg: &Message, args: Args) -> CommandResult {
    let record = args.rest();

    let path = format!("records/{}.wav", record);

    if Path::new(&path).exists() {
        msg.channel_id
            .send_files(&ctx.http, vec![path.as_str()], |m| m)
            .await?;
    } else {
        msg.reply(&ctx.http, "This record doesn't exist").await?;
    }

    Ok(())
}

#[command]
#[only_in(guild)]
#[description = "Deletes the record"]
#[usage = "<record>"]
async fn delete(ctx: &Context, msg: &Message, args: Args) -> CommandResult {
    let record = args.rest();

    let path = format!("records/{}.wav", record);

    if Path::new(&path).exists() {
        std::fs::remove_file(path)?;
        msg.reply(&ctx.http, format!("Succesfully deleted `{}`", record))
            .await?;
    } else {
        msg.reply(&ctx.http, "This record doesn't exist").await?;
    }

    Ok(())
}

#[command]
#[only_in(guilds)]
#[description = "Record yourself or somebody else"]
#[usage = "(@user) (duration)"]
async fn record(ctx: &Context, msg: &Message, mut args: Args) -> CommandResult {
    let duration = args.find::<u64>().unwrap_or(0);
    let user_id = args.find::<UserId>().unwrap_or(msg.author.id);
    let guild = msg.guild(&ctx.cache).await.unwrap();

    let voice_manager = songbird::get(ctx).await.unwrap();

    if voice_manager.get(guild.id).is_some() {
        let receivers_lock = {
            let data = ctx.data.read().await;
            data.get::<Receivers>().unwrap().clone()
        };

        let receivers = receivers_lock.lock().await;

        if let Some(receiver) = receivers.get(&guild.id.0) {
            receiver.set_user_id(user_id.0).await;

            if duration != 0 {
                tokio::spawn(stop_delay(
                    ctx.clone(),
                    msg.clone(),
                    duration,
                    receiver.clone(),
                ));
                msg.reply(
                    &ctx.http,
                    format!(
                        "Now recording {} for {} seconds or use `stop`",
                        user_id.mention(),
                        duration
                    ),
                )
                .await?;
            } else {
                msg.reply(
                    &ctx.http,
                    format!(
                        "Now recording {}. Use `stop` to stop recording",
                        user_id.mention()
                    ),
                )
                .await?;
            }
        }
    } else {
        msg.reply(&ctx.http, "Use `join` first").await?;
    }

    Ok(())
}

async fn stop_delay(ctx: Context, msg: Message, duration: u64, receiver: Receiver) {
    tokio::time::sleep(Duration::from_secs(duration)).await;

    let uuid = Uuid::new_v4().to_simple().to_string();
    if receiver.flush(&uuid).await {
        msg.reply(
            &ctx.http,
            format!(
                "Your recording `{0}` is ready. Rename it with `rename {0} new_name`.",
                uuid
            ),
        )
        .await
        .unwrap();
    }
}

#[command]
#[only_in(guilds)]
#[description = "Stop the recording"]
async fn stop(ctx: &Context, msg: &Message) -> CommandResult {
    let guild = msg.guild(&ctx.cache).await.unwrap();

    let voice_manager = songbird::get(ctx).await.unwrap();

    if let Some(handler_lock) = voice_manager.get(guild.id) {
        {
            let mut handler = handler_lock.lock().await;
            handler.stop();
        }

        let receivers_lock = {
            let data = ctx.data.read().await;
            data.get::<Receivers>().unwrap().clone()
        };

        let receivers = receivers_lock.lock().await;

        if let Some(receiver) = receivers.get(&guild.id.0) {
            let uuid = Uuid::new_v4().to_simple().to_string();
            if receiver.flush(&uuid).await {
                msg.reply(
                    &ctx.http,
                    format!(
                        "Your recording `{0}` is ready. Rename it with `rename {0} new_name`.",
                        uuid
                    ),
                )
                .await?;
            }
        }
    } else {
        msg.reply(&ctx.http, "Not recording").await?;
    }

    Ok(())
}

#[command]
#[only_in(guild)]
#[description = "Rename a record"]
#[usage = "<old_name> <new_name>"]
async fn rename(ctx: &Context, msg: &Message, mut args: Args) -> CommandResult {
    let old_name = args.single::<String>()?;
    let new_name = args.single::<String>()?;

    let old_path = format!("records/{}.wav", old_name);
    let new_path = format!("records/{}.wav", new_name);

    if Path::new(&new_path).exists() {
        msg.reply(
            &ctx.http,
            format!("The record named `{}` already exists", new_name),
        )
        .await?;
        return Ok(());
    }

    if std::fs::rename(old_path, new_path).is_ok() {
        msg.reply(
            &ctx.http,
            format!("Successfully renamed `{}` to `{}`", old_name, new_name),
        )
        .await?;
    } else {
        msg.reply(
            &ctx.http,
            format!("The record `{}` doesn't exist", old_name),
        )
        .await?;
    }

    Ok(())
}

#[command]
#[only_in(guild)]
#[description = "Lists all records"]
async fn records(ctx: &Context, msg: &Message) -> CommandResult {
    let message = std::fs::read_dir("records/")?
        .flatten()
        .flat_map(|e| e.file_name().into_string())
        .filter_map(|s| s.strip_suffix(".wav").map(|s| s.to_owned()))
        .collect::<Vec<String>>()
        .join("\n");

    msg.reply(&ctx, message).await?;

    Ok(())
}

#[derive(Clone)]
struct Receiver {
    user_id: Arc<Mutex<Option<u64>>>,
    ssrcs: Arc<Mutex<HashMap<u64, u32>>>,
    all_bytes: Arc<Mutex<Vec<i16>>>,
}

impl Receiver {
    fn new() -> Self {
        Self {
            user_id: Arc::new(Mutex::new(None)),
            ssrcs: Arc::new(Mutex::new(HashMap::new())),
            all_bytes: Arc::new(Mutex::new(Vec::new())),
        }
    }

    async fn get_user_id(&self) -> Option<u64> {
        let guard = self.user_id.lock().await;
        *guard
    }

    async fn set_user_id(&self, user_id: u64) {
        let mut guard = self.user_id.lock().await;
        let op = &mut *guard;
        op.insert(user_id);
    }

    async fn flush(&self, name: &str) -> bool {
        {
            let mut guard = self.user_id.lock().await;
            let op = &mut *guard;
            op.take();
        }

        let mut guard = self.all_bytes.lock().await;
        let all_bytes = &mut *guard;

        if all_bytes.is_empty() {
            return false;
        }

        let file = File::create(format!("records/{}.wav", name)).unwrap();
        let writer = BufWriter::new(file);
        let mut wav_writer = riff_wave::WaveWriter::new(1, 48000, 32, writer).unwrap();

        for byte in all_bytes.clone() {
            wav_writer.write_sample_i16(byte).unwrap();
        }

        all_bytes.clear();

        true
    }

    async fn add(&self, bytes: &[i16]) {
        let mut guard = self.all_bytes.lock().await;
        let all_bytes = &mut *guard;

        for byte in bytes {
            all_bytes.push(*byte);
        }
    }

    async fn get_ssrc(&self, user_id: u64) -> Option<u32> {
        let guard = self.ssrcs.lock().await;
        let ssrcs = guard;
        ssrcs.get(&user_id).copied()
    }

    async fn set_ssrc(&self, user_id: u64, ssrc: u32) {
        let mut guard = self.ssrcs.lock().await;
        let ssrcs = &mut *guard;
        ssrcs.insert(user_id, ssrc);
    }
}

struct Receivers;

impl TypeMapKey for Receivers {
    type Value = Arc<Mutex<HashMap<u64, Receiver>>>;
}

#[async_trait]
impl EventHandler for Receiver {
    #[allow(clippy::collapsible_match)]
    async fn act(&self, ctx: &EventContext<'_>) -> Option<Event> {
        match ctx {
            EventContext::SpeakingStateUpdate(speaking) => {
                if let Some(user_id) = speaking.user_id {
                    self.set_ssrc(user_id.0, speaking.ssrc).await;
                }
            }
            EventContext::VoicePacket { audio, packet, .. } => {
                if let Some(audio) = audio {
                    if let Some(user_id) = self.get_user_id().await {
                        if let Some(ssrc) = self.get_ssrc(user_id).await {
                            if packet.ssrc == ssrc {
                                self.add(audio).await;
                            }
                        }
                    }
                }
            }
            EventContext::ClientConnect(connected) => {
                self.set_ssrc(connected.user_id.0, connected.audio_ssrc)
                    .await;
            }
            _ => {}
        }

        None
    }
}
