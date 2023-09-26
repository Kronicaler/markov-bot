use std::{cmp::min, convert::TryInto};

use itertools::Itertools;
use serenity::{
    builder::{
        CreateActionRow, CreateButton, CreateComponents, CreateEmbed, EditInteractionResponse,
        EditMessage,
    },
    client::Context,
    model::prelude::{
        component::ButtonStyle,
        interaction::{
            application_command::ApplicationCommandInteraction,
            message_component::MessageComponentInteraction,
        },
        Colour,
    },
};
use tracing::{info_span, Instrument};

use crate::client::ButtonIds;

use super::{model::get_voice_messages_lock, MyAuxMetadata};

///get the queue
#[tracing::instrument(skip(ctx), level = "info")]
pub async fn queue(ctx: &Context, command: &ApplicationCommandInteraction) {
    let manager = songbird::get(ctx)
        .await
        .expect("Songbird Voice client placed in at initialization.")
        .clone();

    command.defer(&ctx.http).await.unwrap();

    if let Some(handler_lock) = manager.get(command.guild_id.unwrap()) {
        let handler = handler_lock.lock().await;
        let queue = handler.queue();

        if queue.is_empty() {
            command
                .edit_original_interaction_response(
                    &ctx.http,
                    EditInteractionResponse::new().content("The queue is empty!"),
                )
                .instrument(info_span!("Sending message"))
                .await
                .expect("Error creating interaction response");
            return;
        }

        //embed
        let queue_message = command
            .edit_original_interaction_response(&ctx.http, create_queue_response(1, queue).await)
            .instrument(info_span!("Sending message"))
            .await
            .expect("Error creating interaction response");

        let voice_messages_lock = get_voice_messages_lock(&ctx.data).await;
        let mut voice_messages = voice_messages_lock.write().await;
        voice_messages
            .queue
            .insert(command.guild_id.unwrap(), queue_message);
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

async fn get_queue_duration(queue: &songbird::tracks::TrackQueue) -> String {
    let mut durations = vec![];

    for track in queue.current_queue() {
        durations.push(
            track
                .typemap()
                .read()
                .await
                .get::<MyAuxMetadata>()
                .unwrap()
                .read()
                .await
                .0
                .duration
                .unwrap(),
        );
    }

    let total_queue_time = durations
        .into_iter()
        .reduce(|a, f| a.checked_add(f).unwrap())
        .unwrap_or_default();
    let hours = total_queue_time.as_secs() / 60 / 60;
    let minutes = total_queue_time.as_secs() / 60 % 60;
    let seconds = total_queue_time.as_secs() % 60;
    let duration = format!("{hours:02}h:{minutes:02}m:{seconds:02}s");

    duration
}

fn create_queue_buttons() -> serenity::builder::CreateComponents {
    CreateComponents::new().set_action_row(
        CreateActionRow::new()
            .add_button(
                CreateButton::new()
                    .emoji(serenity::model::channel::ReactionType::Unicode(
                        "◀".to_string(),
                    ))
                    .style(ButtonStyle::Primary)
                    .custom_id(ButtonIds::QueuePrevious.to_string()),
            )
            .add_button(
                CreateButton::new()
                    .emoji(serenity::model::channel::ReactionType::Unicode(
                        "▶".to_string(),
                    ))
                    .style(ButtonStyle::Primary)
                    .custom_id(ButtonIds::QueueNext.to_string()),
            ),
    )
}

#[tracing::instrument(skip(ctx), level = "info")]
pub async fn change_queue_page(
    ctx: &Context,
    button: &mut MessageComponentInteraction,
    button_id: ButtonIds,
) {
    let manager = songbird::get(ctx)
        .await
        .expect("Songbird Voice client placed in at initialization.")
        .clone();

    match manager.get(button.guild_id.unwrap()) {
        Some(handler_lock) => {
            let handler = handler_lock.lock().await;
            let queue = handler.queue();

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
                get_queue_start_from_button(&button.message.content, button_id, queue);

            let queue_response = create_queue_response(queue_start, queue).await;

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

async fn create_queue_response(
    queue_start: usize,
    queue: &songbird::tracks::TrackQueue,
) -> EditInteractionResponse {
    EditInteractionResponse::new()
        .content(format!("Page {}", queue_start / 10 + 1))
        .embed(create_queue_embed(queue, queue_start - 1).await)
        .components(create_queue_buttons())
}

pub async fn create_queue_edit_message(
    queue_start: usize,
    queue: &songbird::tracks::TrackQueue,
) -> EditMessage {
    EditMessage::new()
        .content(format!("Page {}", queue_start / 10 + 1))
        .embed(create_queue_embed(queue, queue_start - 1).await)
        .components(create_queue_buttons())
}

async fn create_queue_embed(
    queue: &songbird::tracks::TrackQueue,
    queue_start_index: usize,
) -> serenity::builder::CreateEmbed {
    let duration = get_queue_duration(queue).await;
    let colour = Colour::from_rgb(149, 8, 2);

    let mut e = CreateEmbed::new()
        .title("queue")
        .title("Current Queue:")
        .description(format!(
            "Current size: {} | Total queue length: {}",
            queue.len(),
            duration
        ))
        .color(colour);

    let queue_end = min(queue.len(), queue_start_index + 10);

    for i in queue_start_index..queue_end {
        let song = queue
            .current_queue()
            .get(i)
            .unwrap()
            .typemap()
            .read()
            .await
            .get::<MyAuxMetadata>()
            .unwrap()
            .read()
            .await
            .0
            .clone();

        let channel = &song.channel.as_ref().unwrap();
        let title = &song.title.as_ref().unwrap();
        //duration
        let time = &song.duration.as_ref().unwrap();
        let minutes = time.as_secs() / 60;
        let seconds = time.as_secs() - minutes * 60;
        let duration = format!("{minutes}:{seconds:02}");
        let arg1 = format!("{}. {title} | {channel}", i + 1);
        e = e.field(arg1, duration, false);
    }
    e
}

pub fn get_queue_start(message_content: impl Into<String>) -> usize {
    let mut queue_start: i64 = message_content.into().split(' ').collect_vec()[1]
        .parse()
        .unwrap();

    if queue_start != 1 {
        queue_start = queue_start * 10 - 10;
    }

    queue_start.try_into().unwrap()
}

fn get_queue_start_from_button(
    message_content: impl Into<String>,
    button_id: ButtonIds,
    queue: &songbird::tracks::TrackQueue,
) -> usize {
    let mut queue_start = get_queue_start(message_content);

    let queue_len = queue.len();

    match button_id {
        ButtonIds::QueueNext => {
            queue_start += 10;

            while queue_start >= queue_len {
                queue_start -= 10;
            }
        }
        ButtonIds::QueuePrevious => {
            queue_start = if queue_start <= 10 {
                1
            } else {
                queue_start - 10
            };
        }
        _ => {
            panic!("Should never happen")
        }
    }

    queue_start
}
