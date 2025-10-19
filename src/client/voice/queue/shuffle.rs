use std::time::Duration;

use anyhow::Context as AnyhowContext;
use rand::seq::SliceRandom;
use serenity::all::{Context, GuildId};
use songbird::tracks::Queued;
use tokio::time::timeout;

use crate::client::voice::model::get_queue_data_lock;

use super::update_queue_message::update_queue_message;

#[tracing::instrument(skip(ctx))]
pub async fn shuffle_queue(ctx: &Context, guild_id: GuildId) -> anyhow::Result<&'static str> {
    let manager = songbird::get(ctx)
        .await
        .context("Songbird Voice client placed in at initialization.")?
        .clone();

    let Some(call_lock) = manager.get(guild_id) else {
        return Ok("You must be in a voice channel to use that command!");
    };

    let call = timeout(Duration::from_secs(30), call_lock.lock()).await?;
    let queue = call.queue();

    queue.modify_queue(|q| {
        let mut vec: Vec<Queued> = vec![];

        while q.len() > 1 {
            let Some(queued_song) = q.pop_back() else {
                break;
            };

            vec.push(queued_song);
        }

        vec.shuffle(&mut rand::thread_rng());

        while let Some(element) = vec.pop() {
            q.push_back(element);
        }
    });

    drop(call);

    let queue_data_lock = get_queue_data_lock(&ctx.data).await;
    queue_data_lock
        .write()
        .await
        .shuffle_queue
        .insert(guild_id, true);

    let cloned_ctx = ctx.clone();
    tokio::spawn(async move {
        update_queue_message(&cloned_ctx, GuildId::new(guild_id.get()), call_lock).await;
    });

    return Ok("Shuffled the queue");
}
