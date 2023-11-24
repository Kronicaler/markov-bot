use std::cmp::max;

use serenity::builder::EditMessage;
use serenity::client::Context;
use serenity::model::id::GuildId;
use songbird::Call;
use tokio::sync::MutexGuard;
use tracing::{info_span, instrument, Instrument};

use crate::client::voice::model::get_voice_messages_lock;

use super::command_response::{create_queue_edit_message, get_queue_start_from_queue_message};

#[instrument(skip(ctx))]
pub async fn update_queue_message(ctx: &Context, guild_id: GuildId, call: MutexGuard<'_, Call>) {
    let voice_messages_lock = get_voice_messages_lock(&ctx.data).await;

    let queue_message = voice_messages_lock
        .read()
        .instrument(info_span!("Waiting for voice_messages read lock"))
        .await
        .queue
        .get(&guild_id)
        .cloned();

    if let Some(mut queue_message) = queue_message {
        if call.queue().is_empty() {
            queue_message
                .edit(&ctx.http, EditMessage::new().content("The queue is empty!"))
                .instrument(info_span!("Sending message"))
                .await
                .expect("Error creating interaction response");
            return;
        }
        let mut queue_start = get_queue_start_from_queue_message(&queue_message.content);

        let queue = call.queue().clone();
        drop(call);

        let queue_len = queue.len();
        if queue_start > queue_len {
            queue_start = queue_len.saturating_sub(10).saturating_sub(queue_len % 10);

            queue_start = max(queue_start, 1);
        }

        let queue_response = create_queue_edit_message(queue_start, &queue).await;

        queue_message
            .edit(&ctx.http, queue_response)
            .instrument(info_span!("Sending message"))
            .await
            .expect("Error creating interaction response");

        voice_messages_lock
            .write()
            .instrument(info_span!("Waiting for voice_messages write lock"))
            .await
            .queue
            .insert(guild_id, queue_message);
    }
}
