use std::convert::TryInto;

use serenity::{
    builder::{
        CreateActionRow, CreateButton, CreateComponents, CreateEmbed, EditInteractionResponse,
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

use crate::client::ButtonIds;

use super::MyAuxMetadata;

///get the queue
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
                .await
                .expect("Error creating interaction response");
            return;
        }
        let i = if queue.len() < 10 { queue.len() } else { 10 };

        let colour = Colour::from_rgb(149, 8, 2);
        let duration = get_queue_duration(queue).await;
        //embed
        command
            .edit_original_interaction_response(
                &ctx.http,
                EditInteractionResponse::new()
                    .content("0")
                    .embed(create_queue_embed(queue, &duration, colour, 0usize, i).await)
                    .components(create_queue_buttons(queue)),
            )
            .await
            .expect("Error creating interaction response");
    } else {
        command
            .edit_original_interaction_response(
                &ctx.http,
                EditInteractionResponse::new()
                    .content("You must be in a voice channel to use that command!"),
            )
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
                .unwrap()
                .0
                .duration
                .unwrap(),
        )
    }

    let total_queue_time = durations
        .into_iter()
        .reduce(|a, f| a.checked_add(f).unwrap())
        .unwrap_or_default();
    let minutes = total_queue_time.as_secs() / 60;
    let seconds = total_queue_time.as_secs() - minutes * 60;
    let duration = format!("{}:{:02}", minutes, seconds);

    duration
}

fn create_queue_buttons<'a>(
    queue: &songbird::tracks::TrackQueue,
) -> serenity::builder::CreateComponents {
    if queue.len() > 10 {
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
    } else {
        CreateComponents::new()
    }
}

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
                    .await
                    .expect("Error creating interaction response");
                return;
            }

            change_page(button, ctx, button_id, queue).await;
        }
        None => {
            button
                .edit_original_interaction_response(
                    &ctx.http,
                    EditInteractionResponse::new()
                        .content("You must be in a voice channel to use that command!"),
                )
                .await
                .expect("Error creating interaction response");
        }
    }
}

async fn change_page(
    button: &mut MessageComponentInteraction,
    ctx: &Context,
    button_id: ButtonIds,
    queue: &songbird::tracks::TrackQueue,
) {
    let (queue_start, queue_end) = get_page_ends(button, &button_id, queue);

    let duration = get_queue_duration(queue).await;
    let colour = Colour::from_rgb(149, 8, 2);

    button
        .edit_original_interaction_response(
            &ctx.http,
            EditInteractionResponse::new()
                .content(queue_start.to_string())
                .embed(create_queue_embed(queue, &duration, colour, queue_start, queue_end).await)
                .components(create_queue_buttons(queue)),
        )
        .await
        .expect("Error creating interaction response");
}

async fn create_queue_embed(
    queue: &songbird::tracks::TrackQueue,
    duration: &str,
    colour: Colour,
    queue_start: usize,
    queue_end: usize,
) -> serenity::builder::CreateEmbed {
    let mut e = CreateEmbed::new()
        .title("queue")
        .title("Current Queue:")
        .description(format!(
            "Current size: {} | Total queue length: {}",
            queue.len(),
            duration
        ))
        .color(colour);
    for i in queue_start..queue_end {
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
            .unwrap()
            .0
            .clone();

        let channel = &song.channel.as_ref().unwrap();
        let title = &song.title.as_ref().unwrap();
        //duration
        let time = &song.duration.as_ref().unwrap();
        let minutes = time.as_secs() / 60;
        let seconds = time.as_secs() - minutes * 60;
        let duration = format!("{}:{:02}", minutes, seconds);
        let arg1 = format!("{}. {} | {}", i + 1, title, channel);
        e = e.field(arg1, duration, false);
    }
    e
}

fn get_page_ends(
    button: &MessageComponentInteraction,
    button_id: &ButtonIds,
    queue: &songbird::tracks::TrackQueue,
) -> (usize, usize) {
    let queue_end: i64;
    let mut queue_start: i64 = button.message.content.parse().unwrap();
    if button_id == &ButtonIds::QueueNext {
        queue_start += 10;
        queue_end = if queue.len() as i64 - queue_start < 10 {
            queue.len() as i64
        } else {
            queue_start + 10
        };

        while queue_start >= queue.len() as i64 {
            queue_start -= 10;
        }
    } else {
        queue_start = if queue_start < 10 {
            0
        } else {
            queue_start - 10
        };

        queue_end = if queue.len() as i64 - queue_start < 10 {
            queue.len() as i64
        } else {
            queue_start + 10
        };

        while queue_start < 0 {
            queue_start += 10;
        }
    }
    (
        queue_start.try_into().unwrap(),
        queue_end.try_into().unwrap(),
    )
}
