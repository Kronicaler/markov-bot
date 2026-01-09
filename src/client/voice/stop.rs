use serenity::{
    all::Context,
    all::{Colour, CommandInteraction},
    builder::{CreateEmbed, EditInteractionResponse},
};
use tracing::{Instrument, info_span};

use crate::client::global_data::GetBotState;

use super::helper_funcs::{is_bot_in_another_voice_channel, voice_channel_not_same_response};

///stop playing
#[tracing::instrument(skip(ctx))]
pub async fn stop(ctx: &Context, command: &CommandInteraction) {
    let guild_id = command.guild_id.expect("Couldn't get guild ID");
    let guild = guild_id.to_guild_cached(&ctx.cache).map(|g| g.to_owned());

    command.defer(&ctx.http).await.unwrap();

    if let Some(guild) = guild
        && is_bot_in_another_voice_channel(ctx, &guild, command.user.id)
    {
        voice_channel_not_same_response(command, ctx).await;
        return;
    }

    let manager = ctx.bot_state().read().await.songbird.clone();

    if let Some(handler_lock) = manager.get(guild_id) {
        let handler = handler_lock.lock().await;
        let queue = handler.queue();
        queue.stop();
        let state_lock = ctx.bot_state();
        let mut queue = state_lock.write().await;
        queue
            .queue_data
            .filling_queue
            .entry(command.guild_id.unwrap())
            .and_modify(|f| *f = false);
    } else {
        command
            .edit_response(
                &ctx.http,
                EditInteractionResponse::new()
                    .content("Must be in a voice channel to use that command!"),
            )
            .instrument(info_span!("Sending message"))
            .await
            .expect("Error creating interaction response");
        return;
    }

    let colour = Colour::from_rgb(149, 8, 2);
    //embed
    command
        .edit_response(
            &ctx.http,
            EditInteractionResponse::new().embed(
                CreateEmbed::new()
                    .title(String::from("Stopped playing, the queue has been cleared."))
                    .colour(colour),
            ),
        )
        .instrument(info_span!("Sending message"))
        .await
        .expect("Error creating interaction response");
}
