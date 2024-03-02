use crate::client::{
    voice::{
        create_bring_to_front_select_menu, create_play_now_select_menu,
        model::{get_voice_messages_lock, MyAuxMetadata},
    },
    ComponentIds,
};
use itertools::Itertools;
use serenity::{
    builder::{
        CreateActionRow, CreateButton, CreateComponents, CreateEmbed, EditInteractionResponse,
        EditMessage,
    },
    client::Context,
    model::prelude::{
        component::ButtonStyle, interaction::application_command::ApplicationCommandInteraction,
        Colour,
    },
};
use songbird::tracks::TrackQueue;
use std::{cmp::{min, max}, convert::TryInto, time::Duration};
use tokio::time::timeout;
use tracing::{info_span, Instrument};

#[tracing::instrument(skip(ctx))]
pub async fn queue(ctx: &Context, command: &ApplicationCommandInteraction) {
    let manager = songbird::get(ctx)
        .await
        .expect("Songbird Voice client placed in at initialization.")
        .clone();

    command.defer(&ctx.http).await.unwrap();

    if let Some(call_lock) = manager.get(command.guild_id.unwrap()) {
        let call = timeout(Duration::from_secs(30), call_lock.lock())
            .await
            .unwrap();
        let queue = call.queue().clone();

        drop(call);

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
            .edit_original_interaction_response(&ctx.http, create_queue_response(1, &queue).await)
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
                .unwrap_or_default(),
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

async fn create_queue_components(
    queue: &TrackQueue,
    queue_start: usize,
) -> serenity::builder::CreateComponents {
    CreateComponents::new()
        .set_action_row(
            CreateActionRow::new()
                .add_button(
                    CreateButton::new()
                        .emoji(serenity::model::channel::ReactionType::Unicode(
                            "⏪".to_string(),
                        ))
                        .style(ButtonStyle::Primary)
                        .custom_id(ComponentIds::QueueStart.to_string()),
                )
                .add_button(
                    CreateButton::new()
                        .emoji(serenity::model::channel::ReactionType::Unicode(
                            "◀".to_string(),
                        ))
                        .style(ButtonStyle::Primary)
                        .custom_id(ComponentIds::QueuePrevious.to_string()),
                )
                .add_button(
                    CreateButton::new()
                        .emoji(serenity::model::channel::ReactionType::Unicode(
                            "▶".to_string(),
                        ))
                        .style(ButtonStyle::Primary)
                        .custom_id(ComponentIds::QueueNext.to_string()),
                )
                .add_button(
                    CreateButton::new()
                        .emoji(serenity::model::channel::ReactionType::Unicode(
                            "⏩".to_string(),
                        ))
                        .style(ButtonStyle::Primary)
                        .custom_id(ComponentIds::QueueEnd.to_string()),
                ),
        )
        .add_action_row(
            CreateActionRow::new()
                .add_select_menu(create_bring_to_front_select_menu(queue, queue_start).await),
        )
        .add_action_row(
            CreateActionRow::new()
                .add_select_menu(create_play_now_select_menu(queue, queue_start).await),
        )
}

pub async fn create_queue_response(
    queue_start: usize,
    queue: &songbird::tracks::TrackQueue,
) -> EditInteractionResponse {
    EditInteractionResponse::new()
        .content(format!("Page {}", queue_start / 10 + 1))
        .embed(create_queue_embed(queue, queue_start - 1).await)
        .components(create_queue_components(queue, queue_start).await)
}

pub async fn create_queue_edit_message(
    queue_start: usize,
    queue: &songbird::tracks::TrackQueue,
) -> EditMessage {
    EditMessage::new()
        .content(format!("Page {}", queue_start / 10 + 1))
        .embed(create_queue_embed(queue, queue_start - 1).await)
        .components(create_queue_components(queue, queue_start).await)
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
        let (song_name_and_channel, duration) = get_song_metadata_from_queue(queue, i).await;
        e = e.field(song_name_and_channel, duration, false);
    }
    e
}

pub async fn get_song_metadata_from_queue(
    queue: &TrackQueue,
    index_in_queue: usize,
) -> (String, String) {
    let song = queue
        .current_queue()
        .get(index_in_queue)
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

    let channel = &song.channel.unwrap_or_else(|| "Unknown".to_string());
    let title = &song.title.unwrap_or_else(|| "Unknown".to_string());
    //duration
    let time = &song.duration.unwrap_or_default();
    let minutes = time.as_secs() / 60;
    let seconds = time.as_secs() - minutes * 60;
    let duration = format!("{minutes}:{seconds:02}");
    let song_name_and_channel = format!("{}. {title} | {channel}", index_in_queue + 1);

    (song_name_and_channel, duration)
}

pub fn get_queue_start_from_queue_message(message_content: impl Into<String>) -> usize {
    let mut queue_start: i64 = message_content.into().split(' ').collect_vec()[1]
        .parse()
        .unwrap();

    if queue_start != 1 {
        queue_start = queue_start * 10 - 10;
    }

    queue_start.try_into().unwrap()
}

pub fn get_queue_start_from_button(
    message_content: impl Into<String>,
    button_id: ComponentIds,
    queue: &songbird::tracks::TrackQueue,
) -> usize {
    let mut queue_start = get_queue_start_from_queue_message(message_content);

    let queue_len = queue.len();

    match button_id {
        ComponentIds::QueueNext => {
            if queue_start + 10 <= queue_len {
                queue_start += 10;
            }
        }
        ComponentIds::QueuePrevious => {
            queue_start = if queue_start <= 10 {
                1
            } else {
                queue_start.saturating_sub(10)
            };
        }
        ComponentIds::QueueStart => queue_start = 1,
        ComponentIds::QueueEnd => queue_start = max(queue_len - queue_len % 10, 1),
        _ => {
            panic!("Should never happen")
        }
    }

    queue_start
}
