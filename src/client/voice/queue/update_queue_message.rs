use std::{cmp::max, sync::Arc, time::Duration};

use serenity::all::Context;
use serenity::builder::EditMessage;
use serenity::model::id::GuildId;
use songbird::Call;
use tokio::{sync::Mutex, time::timeout};
use tracing::{Instrument, info_span, instrument};

use crate::client::global_data::GetBotState;

use super::command_response::{create_queue_edit_message, get_queue_start_from_queue_message};

#[instrument(skip(ctx, call_lock))]
pub async fn update_queue_message(ctx: &Context, guild_id: GuildId, call_lock: Arc<Mutex<Call>>) {
    let state_lock = ctx.bot_state();

    let queue_message = state_lock
        .read()
        .instrument(info_span!("Waiting for voice_messages read lock"))
        .await
        .voice_messages
        .queue
        .get(&guild_id)
        .cloned();

    if let Some(mut queue_message) = queue_message {
        let call = timeout(Duration::from_secs(30), call_lock.lock())
            .await
            .unwrap();

        if call.queue().is_empty() {
            queue_message
                .edit(&ctx.http, EditMessage::new().content("The queue is empty!"))
                .instrument(info_span!("Sending message"))
                .await
                .expect("Error creating interaction response");
            return;
        }
        let mut queue_start = get_queue_start_from_queue_message(queue_message.content.to_string());

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

        state_lock
            .write()
            .instrument(info_span!("Waiting for voice_messages write lock"))
            .await
            .voice_messages
            .queue
            .insert(guild_id, queue_message);
    }
}
