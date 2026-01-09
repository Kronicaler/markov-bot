use std::time::Duration;

use crate::client::ComponentIds;
use crate::client::global_data::GetBotState;
use crate::client::voice::queue::command_response::{
    create_queue_response, get_queue_start_from_button,
};
use serenity::all::ComponentInteraction;
use serenity::all::Context;
use serenity::builder::EditInteractionResponse;
use tokio::time::timeout;
use tracing;
use tracing::{Instrument, info_span};

#[tracing::instrument(skip(ctx))]
pub async fn change_queue_page(
    ctx: &Context,
    button: &mut ComponentInteraction,
    button_id: ComponentIds,
) {
    let manager = ctx.bot_state().read().await.songbird.clone();

    match manager.get(button.guild_id.unwrap()) {
        Some(call_lock) => {
            let call = timeout(Duration::from_secs(30), call_lock.lock())
                .await
                .unwrap();
            let queue = call.queue().clone();

            drop(call);

            button.defer(&ctx.http).await.unwrap();

            if queue.is_empty() {
                button
                    .edit_response(
                        &ctx.http,
                        EditInteractionResponse::new().content("The queue is empty!"),
                    )
                    .instrument(info_span!("Sending message"))
                    .await
                    .expect("Error creating interaction response");
                return;
            }
            let queue_start =
                get_queue_start_from_button(button.message.content.to_string(), button_id, &queue);

            let queue_response = create_queue_response(queue_start, &queue).await;

            let queue_message = button
                .edit_response(&ctx.http, queue_response)
                .instrument(info_span!("Sending message"))
                .await
                .expect("Error creating interaction response");

            let state_lock = ctx.bot_state();
            let voice_messages = &mut state_lock.write().await.voice_messages;
            voice_messages
                .queue
                .insert(button.guild_id.unwrap(), queue_message);
        }
        None => {
            button
                .edit_response(
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
