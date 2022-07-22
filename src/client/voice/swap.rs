use serenity::{
    client::Context,
    model::interactions::application_command::{
        ApplicationCommandInteraction, ApplicationCommandInteractionDataOptionValue,
    },
};
use songbird::tracks::TrackQueue;
use thiserror::Error;

use super::helper_funcs::{get_call_lock, is_bot_in_another_channel};

pub trait Swapable {
    fn swap(&self, first_track_idx: usize, second_track_idx: usize) -> Result<(), SwapError>;
}

#[derive(Debug, Error)]
pub enum SwapError {
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
    fn swap(&self, first_track_idx: usize, second_track_idx: usize) -> Result<(), SwapError> {
        self.modify_queue(|q| {
            if q.len() < first_track_idx
                || q.len() < second_track_idx
                || first_track_idx < 1
                || second_track_idx < 1
            {
                return Err(SwapError::IndexOutOfBounds);
            }

            if q.is_empty() {
                return Err(SwapError::NothingIsPlaying);
            }

            if first_track_idx == 1 || second_track_idx == 1 {
                return Err(SwapError::CannotSwapCurrentSong);
            }

            if first_track_idx == second_track_idx {
                return Err(SwapError::CannotSwapSameSong);
            }

            let first_track_idx = first_track_idx - 1;
            let second_track_idx = second_track_idx - 1;

            q.swap(first_track_idx, second_track_idx);

            Ok(())
        })
    }
}

pub async fn swap_songs(ctx: &Context, command: &ApplicationCommandInteraction) {
    let guild_id = command.guild_id.expect("Couldn't get guild ID");

    let call_lock = match get_call_lock(ctx, guild_id, command).await {
        Some(value) => value,
        None => return,
    };
    let call = call_lock.lock().await;

    if let Some(guild) = guild_id.to_guild_cached(&ctx.cache).await {
        if is_bot_in_another_channel(ctx, &guild, command.user.id) {
            command
                .create_interaction_response(&ctx.http, |r| {
                    r.interaction_response_data(|d| {
                        d.content("Must be in the same voice channel to use that command!")
                    })
                })
                .await
                .expect("Error creating interaction response");
            return;
        }
    }

    let queue = call.queue();

    let (first_track_idx, second_track_idx) = match get_track_numbers(command) {
        Some(v) => v,
        None => {
            command
                .create_interaction_response(&ctx.http, |r| {
                    r.interaction_response_data(|d| {
                        d.content("Must input which tracks you want to switch!")
                    })
                })
                .await
                .expect("Error creating interaction response");
            return;
        }
    };

    let first_track_idx = if let Ok(v) = first_track_idx.try_into() {
        v
    } else {
        command
            .create_interaction_response(&ctx.http, |r| {
                r.interaction_response_data(|d| d.content("Invalid number!"))
            })
            .await
            .expect("Error creating interaction response");
        return;
    };

    let second_track_idx = if let Ok(v) = second_track_idx.try_into() {
        v
    } else {
        command
            .create_interaction_response(&ctx.http, |r| {
                r.interaction_response_data(|d| d.content("Invalid number!"))
            })
            .await
            .expect("Error creating interaction response");
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
        Err(e) => match e {
            SwapError::IndexOutOfBounds => {
                command
                    .create_interaction_response(&ctx.http, |r| {
                        r.interaction_response_data(|d| d.content("That track isn't in the queue!"))
                    })
                    .await
                    .expect("Error creating interaction response");
            }
            SwapError::NothingIsPlaying => {
                command
                    .create_interaction_response(&ctx.http, |r| {
                        r.interaction_response_data(|d| d.content("Nothing is playing!"))
                    })
                    .await
                    .expect("Error creating interaction response");
            }
            SwapError::CannotSwapCurrentSong => {
                command
                    .create_interaction_response(&ctx.http, |r| {
                        r.interaction_response_data(|d| {
                            d.content("Can't swap the song that's currently playing!")
                        })
                    })
                    .await
                    .expect("Error creating interaction response");
            }
            SwapError::CannotSwapSameSong => {
                command
                    .create_interaction_response(&ctx.http, |r| {
                        r.interaction_response_data(|d| d.content("Can't swap the same song!"))
                    })
                    .await
                    .expect("Error creating interaction response");
            }
        },
    }
}

fn get_track_numbers(command: &ApplicationCommandInteraction) -> Option<(i64, i64)> {
    let first_track_idx = command.data.options.get(0)?.resolved.as_ref()?;

    let first_track_idx =
        if let ApplicationCommandInteractionDataOptionValue::Integer(i) = first_track_idx {
            *i
        } else {
            0
        };

    let second_track_idx = command.data.options.get(1)?.resolved.as_ref()?;

    let second_track_idx =
        if let ApplicationCommandInteractionDataOptionValue::Integer(i) = second_track_idx {
            *i
        } else {
            0
        };

    Some((first_track_idx, second_track_idx))
}
