use std::time::Duration;

use crate::client::voice::model::HasAuxMetadata;

use super::super::helper_funcs::is_bot_in_another_voice_channel;
use futures::executor;
use itertools::Itertools;
use serenity::model::prelude::interaction::message_component::MessageComponentInteraction;
use serenity::prelude::Context;
use tracing::{self, Instrument};
use tracing::{info, info_span};

#[tracing::instrument(skip(ctx))]
pub async fn play_now(ctx: &Context, button: &mut MessageComponentInteraction) {
    button
        .defer(&ctx.http)
        .instrument(info_span!("deferring response"))
        .await
        .unwrap();

    let guild_id = button.guild_id.expect("Couldn't get guild ID");

    if is_bot_in_another_voice_channel(
        ctx,
        &guild_id.to_guild_cached(&ctx.cache).unwrap().clone(),
        button.user.id,
    ) {
        return;
    }

    let manager = songbird::get(ctx)
        .await
        .expect("Songbird Voice client placed in at initialization.")
        .clone();

    let Some(call_lock) = manager.get(guild_id) else {
        return;
    };
    let call = call_lock.lock().await;

    if call.queue().is_empty() {
        return;
    }

    let song_title = button
        .message
        .embeds
        .get(0)
        .unwrap()
        .title
        .as_ref()
        .unwrap()
        .clone();

    call.queue().modify_queue(|q| {
        let Some((index, _song_to_play_now)) = q.iter().find_position(|i| {
            let queued_song_title =
                executor::block_on(async { i.get_aux_metadata().await.title.unwrap() });

            queued_song_title == song_title
        }) else {
            info!("The song is no longer in the queue");
            return;
        };

        let Some(playing_song) = q.get(0) else {
            return;
        };

        _ = playing_song.pause();
        _ = playing_song.seek(Duration::from_secs(0));

        let song_to_play_now = q.remove(index).unwrap();

        q.push_front(song_to_play_now);

        let song_to_play_now = q.get(0).unwrap();

        song_to_play_now.play().unwrap();
    });
}
