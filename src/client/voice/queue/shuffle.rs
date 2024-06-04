use std::time::Duration;

use rand::{seq::SliceRandom, thread_rng};
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

                let current_song = q.pop_front().unwrap();
                while !q.is_empty() {
                    let queued_song = q.pop_front().unwrap();

                    vec.push(queued_song);
                }

                vec.shuffle(&mut thread_rng());

                q.clear();

                q.push_front(current_song);

                while let Some(element) = vec.pop() {
                    q.push_back(element);
                }
            });

            queue.resume().unwrap();
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
