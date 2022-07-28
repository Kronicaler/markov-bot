use serenity::{
    client::Context,
    model::prelude::interaction::application_command::{
        ApplicationCommandInteraction, CommandDataOptionValue,
    },
};
use songbird::tracks::TrackQueue;
use thiserror::Error;

use super::helper_funcs::{get_call_lock, is_bot_in_another_channel};

pub trait Swapable {
    fn swap(&self, first_track_idx: usize, second_track_idx: usize) -> Result<(), SwapableError>;
}

#[derive(Debug, Error)]
pub enum SwapableError {
    #[error("Requested song that wasn't in the queue")]
    IndexOutOfBounds,
    #[error("Can't swap any songs if the queue is empty")]
    NothingIsPlaying,
    #[error("Can't swap the song that's currently being played")]
    CannotSwapCurrentSong,
    #[error("Can't swap a song with itself")]
    CannotSwapSameSong,
}

impl Swapable for TrackQueue {
    fn swap(&self, first_track_idx: usize, second_track_idx: usize) -> Result<(), SwapableError> {
        self.modify_queue(|q| {
            if q.len() < first_track_idx
                || q.len() < second_track_idx
                || first_track_idx < 1
                || second_track_idx < 1
            {
                return Err(SwapableError::IndexOutOfBounds);
            }

            if q.is_empty() {
                return Err(SwapableError::NothingIsPlaying);
            }

            if first_track_idx == 1 || second_track_idx == 1 {
                return Err(SwapableError::CannotSwapCurrentSong);
            }

            if first_track_idx == second_track_idx {
                return Err(SwapableError::CannotSwapSameSong);
            }

            let first_track_idx = first_track_idx - 1;
            let second_track_idx = second_track_idx - 1;

            q.swap(first_track_idx, second_track_idx);

            Ok(())
        })
    }
}

pub async fn swap(ctx: &Context, command: &ApplicationCommandInteraction) {
    let guild_id = command.guild_id.expect("Couldn't get guild ID");

    let call_lock = match get_call_lock(ctx, guild_id, command).await {
        Some(value) => value,
        None => return,
    };

    let call = call_lock.lock().await;

    if let Some(guild) = guild_id.to_guild_cached(&ctx.cache) {
        if is_bot_in_another_channel(ctx, &guild, command.user.id) {
            user_not_in_vc_response(command, ctx).await;
            return;
        }
    }

    let queue = call.queue();

    let (first_track_idx, second_track_idx) = match get_track_numbers(command) {
        Some(v) => v,
        None => {
            invalid_number_response(command, ctx).await;
            return;
        }
    };

    let (first_track_idx, second_track_idx) =
        if let Ok(value) = parse_track_numbers(first_track_idx, second_track_idx) {
            value
        } else {
            invalid_number_response(command, ctx).await;
            return;
        };

    match queue.swap(first_track_idx, second_track_idx) {
        Ok(_) => {
            command
                .create_interaction_response(&ctx.http, |r| {
                    r.interaction_response_data(|d| {
                        d.content(format!(
                            "Swapped track {} and {}.",
                            first_track_idx, second_track_idx
                        ))
                    })
                })
                .await
                .expect("Error creating interaction response");
        }
        Err(e) => swapping_error_response(e, command, ctx).await,
    }
}

async fn invalid_number_response(command: &ApplicationCommandInteraction, ctx: &Context) {
    command
        .create_interaction_response(&ctx.http, |r| {
            r.interaction_response_data(|d| d.content("Invalid number!"))
        })
        .await
        .expect("Error creating interaction response");
}

fn parse_track_numbers(
    first_track_idx: i64,
    second_track_idx: i64,
) -> anyhow::Result<(usize, usize)> {
    let first_track_idx = first_track_idx.try_into()?;
    let second_track_idx = second_track_idx.try_into()?;

    Ok((first_track_idx, second_track_idx))
}

async fn user_not_in_vc_response(command: &ApplicationCommandInteraction, ctx: &Context) {
    command
        .create_interaction_response(&ctx.http, |r| {
            r.interaction_response_data(|d| {
                d.content("Must be in the same voice channel to use that command!")
            })
        })
        .await
        .expect("Error creating interaction response");
}

async fn swapping_error_response(
    e: SwapableError,
    command: &ApplicationCommandInteraction,
    ctx: &Context,
) {
    match e {
        SwapableError::IndexOutOfBounds => {
            command
                .create_interaction_response(&ctx.http, |r| {
                    r.interaction_response_data(|d| d.content("That track isn't in the queue!"))
                })
                .await
                .expect("Error creating interaction response");
        }
        SwapableError::NothingIsPlaying => {
            command
                .create_interaction_response(&ctx.http, |r| {
                    r.interaction_response_data(|d| d.content("Nothing is playing!"))
                })
                .await
                .expect("Error creating interaction response");
        }
        SwapableError::CannotSwapCurrentSong => {
            command
                .create_interaction_response(&ctx.http, |r| {
                    r.interaction_response_data(|d| {
                        d.content("Can't swap the song that's currently playing!")
                    })
                })
                .await
                .expect("Error creating interaction response");
        }
        SwapableError::CannotSwapSameSong => {
            command
                .create_interaction_response(&ctx.http, |r| {
                    r.interaction_response_data(|d| d.content("Can't swap the same song!"))
                })
                .await
                .expect("Error creating interaction response");
        }
    }
}

fn get_track_numbers(command: &ApplicationCommandInteraction) -> Option<(i64, i64)> {
    let first_track_idx = match command.data.options.get(0)?.resolved.as_ref()? {
        CommandDataOptionValue::Integer(i) => *i,
        _ => return None,
    };

    let second_track_idx = match command.data.options.get(1)?.resolved.as_ref()? {
        CommandDataOptionValue::Integer(i) => *i,
        _ => return None,
    };

    Some((first_track_idx, second_track_idx))
}
