use serenity::{
    client::Context,
    model::prelude::{
        component::ButtonStyle,
        interaction::{
            application_command::ApplicationCommandInteraction,
            message_component::MessageComponentInteraction,
        },
    },
    utils::Colour,
};

use crate::client::ButtonIds;

///get the queue
pub async fn queue(ctx: &Context, command: &ApplicationCommandInteraction) {
    let manager = songbird::get(ctx)
        .await
        .expect("Songbird Voice client placed in at initialisation.")
        .clone();

    if let Some(handler_lock) = manager.get(command.guild_id.unwrap()) {
        let handler = handler_lock.lock().await;
        let queue = handler.queue();

        if queue.is_empty() {
            command
                .create_interaction_response(&ctx.http, |r| {
                    r.interaction_response_data(|d| d.content("The queue is empty!"))
                })
                .await
                .expect("Error creating interaction response");
            return;
        }
        //embed
        command
            .create_interaction_response(&ctx.http, |m| {
                //embed
                let i = if queue.len() < 10 { queue.len() } else { 10 };
                //color
                let colour = Colour::from_rgb(149, 8, 2);
                let total_queue_time = queue
                    .current_queue()
                    .iter()
                    .map(|f| f.metadata().duration.unwrap())
                    .reduce(|a, f| a.checked_add(f).unwrap())
                    .unwrap_or_default();

                let minutes = total_queue_time.as_secs() / 60;
                let seconds = total_queue_time.as_secs() - minutes * 60;
                let duration = format!("{}:{:02}", minutes, seconds);

                m.interaction_response_data(|d| {
                    d.content("0")
                        .embed(|e| {
                            e.title("queue")
                                .title("Current Queue:")
                                .description(format!(
                                    "Current size: {} | Total queue length: {}",
                                    queue.len(),
                                    duration
                                ))
                                .color(colour);
                            for i in 0..i {
                                let song =
                                    &queue.current_queue().get(i).unwrap().metadata().clone();
                                let channel = &song.channel.as_ref().unwrap();
                                let title = &song.title.as_ref().unwrap();
                                //duration
                                let time = &song.duration.as_ref().unwrap();
                                let minutes = time.as_secs() / 60;
                                let seconds = time.as_secs() - minutes * 60;
                                let duration = format!("{}:{:02}", minutes, seconds);
                                let arg1 = format!("{}. {} | {}", i + 1, title, channel);
                                e.field(arg1, duration, false);
                            }
                            e
                        })
                        .components(|c| {
                            if queue.len() > 10 {
                                c.create_action_row(|a| {
                                    a.create_button(|b| {
                                        b.emoji(serenity::model::channel::ReactionType::Unicode(
                                            "◀".to_string(),
                                        ))
                                        .style(ButtonStyle::Primary)
                                        .custom_id(ButtonIds::QueuePrevious)
                                    })
                                    .create_button(|b| {
                                        b.emoji(serenity::model::channel::ReactionType::Unicode(
                                            "▶".to_string(),
                                        ))
                                        .style(ButtonStyle::Primary)
                                        .custom_id(ButtonIds::QueueNext)
                                    })
                                })
                            } else {
                                c
                            }
                        })
                })
            })
            .await
            .expect("Error creating interaction response");
    } else {
        command
            .create_interaction_response(&ctx.http, |r| {
                r.interaction_response_data(|d| {
                    d.content("You must be in a voice channel to use that command!")
                })
            })
            .await
            .expect("Error creating interaction response");
    }
}

pub async fn change_queue_page(
    ctx: &Context,
    button: &mut MessageComponentInteraction,
    button_id: ButtonIds,
) {
    let manager = songbird::get(ctx)
        .await
        .expect("Songbird Voice client placed in at initialisation.")
        .clone();

    match manager.get(button.guild_id.unwrap()) {
        Some(handler_lock) => {
            let handler = handler_lock.lock().await;
            let queue = handler.queue();

            button.defer(&ctx.http).await.unwrap();

            if queue.is_empty() {
                button
                    .edit_original_interaction_response(&ctx.http, |r| {
                        r.content("The queue is empty!")
                    })
                    .await
                    .expect("Error creating interaction response");
                return;
            }

            change_page(button, ctx, button_id, queue).await;
        }
        None => {
            button
                .edit_original_interaction_response(&ctx.http, |d| {
                    d.content("You must be in a voice channel to use that command!")
                })
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
    button
        .edit_original_interaction_response(&ctx.http, |d| {
            let (queue_start, queue_end) = get_page_ends(button, &button_id, queue);
            //color
            let colour = Colour::from_rgb(149, 8, 2);
            let total_queue_time = queue
                .current_queue()
                .iter()
                .map(|f| f.metadata().duration.unwrap())
                .reduce(|a, f| a.checked_add(f).unwrap())
                .unwrap_or_default();

            let minutes = total_queue_time.as_secs() / 60;
            let seconds = total_queue_time.as_secs() - minutes * 60;
            let duration = format!("{}:{:02}", minutes, seconds);

            d.content(queue_start.to_string())
                .embed(|e| {
                    e.title("queue")
                        .title("Current Queue:")
                        .description(format!(
                            "Current size: {} | Total queue length: {}",
                            queue.len(),
                            duration
                        ))
                        .color(colour);

                    for i in queue_start..queue_end {
                        let song = &queue
                            .current_queue()
                            .get(usize::try_from(i).unwrap())
                            .unwrap()
                            .metadata()
                            .clone();

                        let channel = &song.channel.as_ref().unwrap();
                        let title = &song.title.as_ref().unwrap();
                        //duration
                        let time = &song.duration.as_ref().unwrap();
                        let minutes = time.as_secs() / 60;
                        let seconds = time.as_secs() - minutes * 60;
                        let duration = format!("{}:{:02}", minutes, seconds);
                        let arg1 = format!("{}. {} | {}", i + 1, title, channel);
                        e.field(arg1, duration, false);
                    }
                    e
                })
                .components(|c| {
                    if queue.len() > 10 {
                        c.create_action_row(|a| {
                            a.create_button(|b| {
                                b.emoji(serenity::model::channel::ReactionType::Unicode(
                                    "◀".to_string(),
                                ))
                                .style(ButtonStyle::Primary)
                                .custom_id(ButtonIds::QueuePrevious)
                            })
                            .create_button(|b| {
                                b.emoji(serenity::model::channel::ReactionType::Unicode(
                                    "▶".to_string(),
                                ))
                                .style(ButtonStyle::Primary)
                                .custom_id(ButtonIds::QueueNext)
                            })
                        })
                    } else {
                        c
                    }
                })
        })
        .await
        .expect("Error creating interaction response");
}

fn get_page_ends(
    button: &MessageComponentInteraction,
    button_id: &ButtonIds,
    queue: &songbird::tracks::TrackQueue,
) -> (i64, i64) {
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
    (queue_start, queue_end)
}
