use serenity::builder::EditMessage;
use serenity::client::Context;
use serenity::model::id::GuildId;
use tracing::{info_span, instrument, Instrument};

use crate::client::voice::model::get_voice_messages_lock;

use super::queue_command_response::{get_queue_start_from_queue_message, create_queue_edit_message};

#[instrument(skip(ctx))]
pub async fn update_queue_message(ctx: &Context, guild_id: GuildId) {
    let songbird = songbird::get(&ctx).await.unwrap();

    let call_lock = songbird.get(guild_id).unwrap();

    let voice_messages_lock = get_voice_messages_lock(&ctx.data).await;

    let queue_message = voice_messages_lock
        .read()
        .instrument(info_span!("Waiting for voice_messages read lock"))
        .await
        .queue
        .get(&guild_id)
        .cloned();

    if let Some(mut queue_message) = queue_message {
        if call_lock
            .lock()
            .instrument(info_span!("Waiting for call lock"))
            .await
            .queue()
            .is_empty()
        {
            queue_message
                .edit(&ctx.http, EditMessage::new().content("The queue is empty!"))
                .instrument(info_span!("Sending message"))
                .await
                .expect("Error creating interaction response");
            return;
        }
        let queue_start = get_queue_start_from_queue_message(&queue_message.content);

        let queue = call_lock
            .lock()
            .instrument(info_span!("Waiting for call lock"))
            .await
            .queue()
            .clone();
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
