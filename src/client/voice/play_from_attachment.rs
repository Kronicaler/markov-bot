use crate::client::voice::helper_funcs::{
    get_voice_channel_of_user, is_bot_in_another_voice_channel, voice_channel_not_same_response,
};
use crate::client::voice::play::{add_track_start_event, voice_channel_not_found_response};
use serenity::builder::EditInteractionResponse;
use serenity::model::prelude::interaction::application_command::ApplicationCommandInteraction;
use serenity::prelude::Context;
use tracing::info_span;
use tracing::{self, Instrument};

#[tracing::instrument(skip(ctx))]
pub async fn play_from_attachment(ctx: &Context, command: &ApplicationCommandInteraction) {
    command
        .defer(&ctx.http)
        .instrument(info_span!("deferring response"))
        .await
        .unwrap();

    let guild_id = command.guild_id.expect("Couldn't get guild ID");

    if is_bot_in_another_voice_channel(
        ctx,
        &guild_id.to_guild_cached(&ctx.cache).unwrap().clone(),
        command.user.id,
    ) {
        voice_channel_not_same_response(&command, &ctx).await;

        return;
    }

    let manager = songbird::get(ctx)
        .await
        .expect("Songbird Voice client placed in at initialization.")
        .clone();

    let message_id = command.data.target_id.unwrap();

    let video = command
        .data
        .resolved
        .messages
        .get(&message_id.into())
        .unwrap()
        .attachments
        .get(0);

    let Some(attachment) = video else {
        command
            .edit_original_interaction_response(
                &ctx.http,
                EditInteractionResponse::new().content("Message must contain an attachment"),
            )
            .await
            .unwrap();
        return;
    };
    // TODO get the attachments with yt-dlp to get the aux metadata
    // TODO get attachments that were posted as links
    if !attachment
        .content_type
        .as_ref()
        .unwrap_or(&String::new())
        .contains("video")
        && !attachment
            .content_type
            .as_ref()
            .unwrap_or(&String::new())
            .contains("audio")
    {
        command
            .edit_original_interaction_response(
                &ctx.http,
                EditInteractionResponse::new()
                    .content("The attachment in the message is not a video or audio file"),
            )
            .await
            .unwrap();
        return;
    }

    let Some(voice_channel_id) = get_voice_channel_of_user(
        &command
            .guild_id
            .unwrap()
            .to_guild_cached(&ctx.cache)
            .unwrap(),
        command.user.id,
    ) else {
        voice_channel_not_found_response(command, ctx).await;
        return;
    };

    let (call_lock, success) = manager
        .join(guild_id, voice_channel_id)
        .instrument(info_span!("Joining channel"))
        .await;

    if success.is_err() {
        voice_channel_not_found_response(command, ctx).await;
        return;
    }

    {
        add_track_start_event(&mut call_lock.lock().await, command, ctx);

        let mut call = call_lock.lock().await;

        call.enqueue_input(attachment.download().await.unwrap().into())
            .await;

        command
            .edit_original_interaction_response(
                &ctx.http,
                EditInteractionResponse::new().content("Playing the attachment now"),
            )
            .await
            .unwrap();

        if call.queue().len() == 1 {
            return;
        }

        call.queue().modify_queue(|q| {
            let Some(playing_song) = q.get(0) else {
                return;
            };

            _ = playing_song.pause();
            _ = playing_song.seek(std::time::Duration::from_secs(0));

            let song_to_play_now = q.pop_back().unwrap();

            q.push_front(song_to_play_now);

            let song_to_play_now = q.get(0).unwrap();

            song_to_play_now.play().unwrap();
        });
    }
}
