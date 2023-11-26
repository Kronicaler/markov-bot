use std::time::Duration;

use rand::{seq::SliceRandom, thread_rng};
use serenity::{
    builder::EditInteractionResponse,
    model::prelude::interaction::application_command::ApplicationCommandInteraction, prelude::*,
};
use songbird::tracks::Queued;
use tokio::time::timeout;
use tracing::{info_span, Instrument};

use super::update_queue_message::update_queue_message;

#[tracing::instrument(skip(ctx))]
pub async fn shuffle_queue(ctx: &Context, command: &ApplicationCommandInteraction) {
    let manager = songbird::get(ctx)
        .await
        .expect("Songbird Voice client placed in at initialization.")
        .clone();

    command.defer(&ctx.http).await.unwrap();

    if let Some(call_lock) = manager.get(command.guild_id.unwrap()) {
        let call = timeout(Duration::from_secs(30), call_lock.lock())
            .await
            .unwrap();
        let queue = call.queue();

        if queue.is_empty() {
            empty_queue_response(command, ctx).await;
            return;
        }

        queue.modify_queue(|q| {
            let mut vec: Vec<Queued> = vec![];

            let current_song = q.pop_front().unwrap();
            while q.len() != 0 {
                let queued_song = q.pop_front().unwrap();

                vec.push(queued_song);
            }

            vec.shuffle(&mut thread_rng());

            q.clear();

            q.push_front(current_song);

            while vec.len() != 0 {
                q.push_back(vec.pop().unwrap());
            }
        });

        queue.resume().unwrap();
        drop(call);

        let cloned_ctx = ctx.clone();
        let guild_id = command.guild_id;
        tokio::spawn(async move {
            update_queue_message(&cloned_ctx, guild_id.unwrap(), call_lock).await
        });

        command
            .edit_original_interaction_response(
                &ctx.http,
                EditInteractionResponse::new().content("Shuffled the queue"),
            )
            .instrument(info_span!("Sending message"))
            .await
            .expect("Error creating interaction response");
    } else {
        command
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

async fn empty_queue_response(command: &ApplicationCommandInteraction, ctx: &Context) {
    command
        .edit_original_interaction_response(
            &ctx.http,
            EditInteractionResponse::new().content("The queue is empty!"),
        )
        .instrument(info_span!("Sending message"))
        .await
        .expect("Error creating interaction response");
}
