use serenity::{
    builder::EditInteractionResponse,
    client::Context,
    model::prelude::interaction::application_command::{
        ApplicationCommandInteraction, CommandDataOptionValue,
    },
};
use songbird::tracks::TrackQueue;
use thiserror::Error;

use super::{
    helper_funcs::{
        get_call_lock, is_bot_in_another_voice_channel, voice_channel_not_same_response,
    },
    MyAuxMetadata,
};

pub trait Swapable {
    fn swap(
        &self,
        first_track_idx: usize,
        second_track_idx: usize,
    ) -> Result<(MyAuxMetadata, MyAuxMetadata), SwapableError>;
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
    fn swap(
        &self,
        first_track_idx: usize,
        second_track_idx: usize,
    ) -> Result<(MyAuxMetadata, MyAuxMetadata), SwapableError> {
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

            let first_track = q
                .get(first_track_idx)
                .unwrap()
                .typemap()
                .blocking_read()
                .get::<MyAuxMetadata>()
                .unwrap()
                .read()
                .unwrap()
                .clone();

            let second_track = q
                .get(second_track_idx)
                .unwrap()
                .typemap()
                .blocking_read()
                .get::<MyAuxMetadata>()
                .unwrap()
                .read()
                .unwrap()
                .clone();

            q.swap(first_track_idx, second_track_idx);

            Ok((first_track, second_track))
        })
    }
}

pub async fn swap(ctx: &Context, command: &ApplicationCommandInteraction) {
    let guild_id = command.guild_id.expect("Couldn't get guild ID");
    let guild = guild_id
        .to_guild_cached(&ctx.cache)
        .and_then(|g| Some(g.to_owned()));

    command.defer(&ctx.http).await.unwrap();

    if let Some(guild) = guild {
        if is_bot_in_another_voice_channel(ctx, &guild, command.user.id) {
            voice_channel_not_same_response(&command, &ctx).await;
            return;
        }
    }

    let call_lock = match get_call_lock(ctx, guild_id, command).await {
        Some(value) => value,
        None => return,
    };

    let call = call_lock.lock().await;

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
        Ok((first_track, second_track)) => {
            command
                .edit_original_interaction_response(
                    &ctx.http,
                    EditInteractionResponse::new().content(format!(
                        "Swapped track {} and {}.",
                        first_track.0.title.unwrap_or("No Title".to_string()),
                        second_track.0.title.unwrap_or("No Title".to_string())
                    )),
                )
                .await
                .expect("Error creating interaction response");
        }
        Err(e) => swapping_error_response(e, command, ctx).await,
    }
}

async fn invalid_number_response(command: &ApplicationCommandInteraction, ctx: &Context) {
    command
        .edit_original_interaction_response(
            &ctx.http,
            EditInteractionResponse::new().content("Invalid number!"),
        )
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

async fn swapping_error_response(
    e: SwapableError,
    command: &ApplicationCommandInteraction,
    ctx: &Context,
) {
    match e {
        SwapableError::IndexOutOfBounds => {
            command
                .edit_original_interaction_response(
                    &ctx.http,
                    EditInteractionResponse::new().content("That track isn't in the queue!"),
                )
                .await
                .expect("Error creating interaction response");
        }
        SwapableError::NothingIsPlaying => {
            command
                .edit_original_interaction_response(
                    &ctx.http,
                    EditInteractionResponse::new().content("Nothing is playing!"),
                )
                .await
                .expect("Error creating interaction response");
        }
        SwapableError::CannotSwapCurrentSong => {
            command
                .edit_original_interaction_response(
                    &ctx.http,
                    EditInteractionResponse::new()
                        .content("Can't swap the song that's currently playing!"),
                )
                .await
                .expect("Error creating interaction response");
        }
        SwapableError::CannotSwapSameSong => {
            command
                .edit_original_interaction_response(
                    &ctx.http,
                    EditInteractionResponse::new().content("Can't swap the same song!"),
                )
                .await
                .expect("Error creating interaction response");
        }
    }
}

fn get_track_numbers(command: &ApplicationCommandInteraction) -> Option<(i64, i64)> {
    let first_track_idx = match command.data.options.get(0).unwrap().value {
        CommandDataOptionValue::Integer(i) => i,
        _ => return None,
    };

    let second_track_idx = match command.data.options.get(1).unwrap().value {
        CommandDataOptionValue::Integer(i) => i,
        _ => return None,
    };

    Some((first_track_idx, second_track_idx))
}
