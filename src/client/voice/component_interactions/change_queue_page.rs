use super::super::model::get_voice_messages_lock;
use crate::client::voice::queue::{create_queue_response, get_queue_start_from_button};
use crate::client::ComponentIds;
use serenity::builder::EditInteractionResponse;
use serenity::client::Context;
use serenity::model::prelude::interaction::message_component::MessageComponentInteraction;
use tracing;
use tracing::{info_span, Instrument};

#[tracing::instrument(skip(ctx))]
pub async fn change_queue_page(
    ctx: &Context,
    button: &mut MessageComponentInteraction,
    button_id: ComponentIds,
) {
    let manager = songbird::get(ctx)
        .await
        .expect("Songbird Voice client placed in at initialization.")
        .clone();

    match manager.get(button.guild_id.unwrap()) {
        Some(call_lock) => {
            let call = call_lock.lock().await;
            let queue = call.queue().clone();

            drop(call);

            button.defer(&ctx.http).await.unwrap();

            if queue.is_empty() {
                button
                    .edit_original_interaction_response(
                        &ctx.http,
                        EditInteractionResponse::new().content("The queue is empty!"),
                    )
                    .instrument(info_span!("Sending message"))
                    .await
                    .expect("Error creating interaction response");
                return;
            }
            let queue_start =
                get_queue_start_from_button(&button.message.content, button_id, &queue);

            let queue_response = create_queue_response(queue_start, &queue).await;

            let queue_message = button
                .edit_original_interaction_response(&ctx.http, queue_response)
                .instrument(info_span!("Sending message"))
                .await
                .expect("Error creating interaction response");

            let voice_messages_lock = get_voice_messages_lock(&ctx.data).await;
            let mut voice_messages = voice_messages_lock.write().await;
            voice_messages
                .queue
                .insert(button.guild_id.unwrap(), queue_message);
        }
        None => {
            button
                .edit_original_interaction_response(
                    &ctx.http,
                    EditInteractionResponse::new()
                        .content("You must be in a voice channel to use that command!"),
                )
                .instrument(info_span!("Sending message"))
                .await
                .expect("Error creating interaction response");
        }
    }
}
