use std::time::Duration;

use rand::seq::SliceRandom;
use serenity::all::{Context, GuildId};
use songbird::tracks::Queued;
use tokio::time::timeout;

use super::update_queue_message::update_queue_message;

#[tracing::instrument(skip(ctx))]
pub async fn shuffle_queue(ctx: &Context, guild_id: GuildId) -> &'static str {
    let manager = songbird::get(ctx)
        .await
        .expect("Songbird Voice client placed in at initialization.")
        .clone();

    match manager.get(guild_id) {
        Some(call_lock) => {
            let call = timeout(Duration::from_secs(30), call_lock.lock())
                .await
                .unwrap();
            let queue = call.queue();

            if queue.is_empty() {
                return "The queue is empty!";
            }

            queue.modify_queue(|q| {
                let mut vec: Vec<Queued> = vec![];

                while q.len() > 1 {
                    let queued_song = q.pop_back().unwrap();

                    vec.push(queued_song);
                }

                vec.shuffle(&mut rand::thread_rng());

                while let Some(element) = vec.pop() {
                    q.push_back(element);
                }
            });

            drop(call);

            let cloned_ctx = ctx.clone();
            tokio::spawn(async move {
                update_queue_message(&cloned_ctx, GuildId::new(guild_id.get()), call_lock).await;
            });

            return "Shuffled the queue";
        }
        None => return "You must be in a voice channel to use that command!",
    }
}
