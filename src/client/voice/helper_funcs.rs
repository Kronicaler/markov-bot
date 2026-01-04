use serenity::{
    all::{CommandInteraction, VoiceState},
    builder::EditInteractionResponse,
    client::Context,
    model::{
        guild::Guild,
        id::{ChannelId, UserId},
    },
};
use tracing::{Instrument, info_span};

use crate::client::helper_funcs::get_guild_channel;

pub fn get_voice_channel_of_user(guild: &Guild, user_id: UserId) -> Option<ChannelId> {
    guild
        .voice_states
        .get(&user_id)
        .and_then(|voice_state| voice_state.channel_id)
}

pub fn get_voice_channel_of_bot(ctx: &Context, guild: &Guild) -> Option<ChannelId> {
    guild
        .voice_states
        .get(&UserId::new(ctx.http.application_id().unwrap().get()))
        .and_then(|voice_state| voice_state.channel_id)
}

pub fn is_bot_in_another_voice_channel(ctx: &Context, guild: &Guild, user_id: UserId) -> bool {
    let user_voice_channel = get_voice_channel_of_user(guild, user_id);

    let Some(user_voice_channel) = user_voice_channel else {
        return true;
    };

    let bot_voice_channel = get_voice_channel_of_bot(ctx, guild);

    let Some(bot_voice_channel) = bot_voice_channel else {
        return false;
    };

    if user_voice_channel != bot_voice_channel {
        return true;
    }

    false
}

pub async fn voice_channel_not_same_response(command: &CommandInteraction, ctx: &Context) {
    command
        .edit_response(
            &ctx.http,
            EditInteractionResponse::new()
                .content("You must be in the same voice channel to use this command!"),
        )
        .instrument(info_span!("Sending message"))
        .await
        .expect("Error creating interaction response");
}

pub async fn get_call_lock(
    ctx: &Context,
    guild_id: serenity::model::id::GuildId,
    command: &CommandInteraction,
) -> Option<std::sync::Arc<serenity::prelude::Mutex<songbird::Call>>> {
    let manager = songbird::get(ctx)
        .await
        .expect("Songbird Voice client placed in at initialization.")
        .clone();

    let Some(call_lock) = manager.get(guild_id) else {
        command
            .edit_response(
                &ctx.http,
                EditInteractionResponse::new()
                    .content("Must be in a voice channel to use that command!"),
            )
            .instrument(info_span!("Sending message"))
            .await
            .expect("Error creating interaction response");
        return None;
    };

    Some(call_lock)
}

pub async fn leave_vc_if_alone(old: Option<VoiceState>, ctx: &Context) {
    let Some(old) = old else { return };

    let Some(guild_id) = old.guild_id else { return };

    let Some(channel_id) = old.channel_id else {
        return;
    };

    let manager = songbird::get(ctx)
        .await
        .expect("Songbird Voice client placed in at initialization.")
        .clone();

    let call_mutex = manager.get(guild_id);
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

    let changed_voice_channel = get_guild_channel(guild_id, ctx, channel_id).await.unwrap();

    if changed_voice_channel.id.get() != bot_voice_channel.0.get() {
        return;
    }

    let changed_voice_channel_members = changed_voice_channel.members(&ctx.cache).unwrap();

    if changed_voice_channel_members.len() == 1 {
        call.queue().stop();
        call.remove_all_global_events();
        call.leave().await.expect("Couldn't leave voice channel");
    }
}
