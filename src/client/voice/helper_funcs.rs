use serenity::{
    client::Context,
    model::{
        guild::Guild,
        id::{ChannelId, GuildId, UserId},
        interactions::application_command::ApplicationCommandInteraction,
        prelude::VoiceState,
    },
};

pub fn get_voice_channel_of_user(guild: &Guild, user_id: UserId) -> Option<ChannelId> {
    guild
        .voice_states
        .get(&user_id)
        .and_then(|voice_state| voice_state.channel_id)
}

pub fn get_voice_channel_of_bot(ctx: &Context, guild: &Guild) -> Option<ChannelId> {
    guild
        .voice_states
        .get(&ctx.http.application_id.into())
        .and_then(|voice_state| voice_state.channel_id)
}

pub fn is_bot_in_another_channel(ctx: &Context, guild: &Guild, user_id: UserId) -> bool {
    let user_voice_channel = get_voice_channel_of_user(guild, user_id);

    let user_voice_channel = match user_voice_channel {
        Some(c) => c,
        None => return true,
    };

    let bot_voice_channel = get_voice_channel_of_bot(ctx, guild);

    let bot_voice_channel = match bot_voice_channel {
        Some(c) => c,
        None => return false,
    };

    if user_voice_channel != bot_voice_channel {
        return true;
    }

    false
}

pub async fn get_call_lock(
    ctx: &Context,
    guild_id: serenity::model::id::GuildId,
    command: &ApplicationCommandInteraction,
) -> Option<std::sync::Arc<serenity::prelude::Mutex<songbird::Call>>> {
    let manager = songbird::get(ctx)
        .await
        .expect("Songbird Voice client placed in at initialisation.")
        .clone();

    let call_lock = match manager.get(guild_id.0) {
        Some(c) => c,
        None => {
            command
                .create_interaction_response(&ctx.http, |r| {
                    r.interaction_response_data(|d| {
                        d.content("Must be in a voice channel to use that command!")
                    })
                })
                .await
                .expect("Error creating interaction response");
            return None;
        }
    };

    Some(call_lock)
}

pub async fn leave_vc_if_alone(
    old: Option<VoiceState>,
    ctx: &Context,
    guild_id_option: Option<GuildId>,
) {
    // If a user joined a channel
    if old.is_none() {
        return;
    }

    let old = old.unwrap();

    let manager = songbird::get(ctx)
        .await
        .expect("Songbird Voice client placed in at initialisation.")
        .clone();

    let call_mutex = manager.get(guild_id_option.unwrap());
    if call_mutex.is_none() {
        return;
    }
    let call_mutex = call_mutex.unwrap();

    let mut call = call_mutex.lock().await;

    let bot_voice_channel = call.current_channel();
    if bot_voice_channel.is_none() {
        return;
    }
    let bot_voice_channel = bot_voice_channel.unwrap();

    let guild = guild_id_option
        .unwrap()
        .to_guild_cached(&ctx.cache)
        .await
        .unwrap();

    let changed_voice_channel = guild.channels.get(&old.channel_id.unwrap()).unwrap();

    if changed_voice_channel.id.0 != bot_voice_channel.0 {
        return;
    }

    let changed_voice_channel_members = changed_voice_channel.members(&ctx.cache).await.unwrap();

    if changed_voice_channel_members.len() == 1 {
        call.queue().stop();
        call.leave().await.expect("Couldn't leave voice channel");
    }
}
