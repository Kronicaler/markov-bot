use serenity::{
    client::Context,
    model::{
        guild::Guild,
        id::{ChannelId, UserId},
        interactions::application_command::ApplicationCommandInteraction,
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

pub async fn is_bot_in_another_channel(ctx: &Context, guild: &Guild, user_id: UserId) -> bool {
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

    return false;
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
