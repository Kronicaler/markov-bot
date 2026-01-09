use std::{
    cmp::{max, min},
    time::Duration,
};

use serenity::{
    all::Context,
    all::{CommandDataOptionValue, CommandInteraction},
    builder::EditInteractionResponse,
};
use songbird::tracks::TrackQueue;
use thiserror::Error;
use tokio::time::timeout;
use tracing::{Instrument, info_span};

use super::{
    MyAuxMetadata,
    helper_funcs::{
        get_call_lock, is_bot_in_another_voice_channel, voice_channel_not_same_response,
    },
};

pub trait Swapable {
    fn swap(&self, first_track_idx: usize, second_track_idx: usize) -> Result<(), SwapableError>;
}

#[derive(Debug, Error)]
pub enum SwapableError {
    #[error("Requested song that wasn't in the queue")]
    IndexOutOfBounds,
    #[error("Can't swap any songs if the queue is empty")]
    NothingIsPlaying,
    #[error("Can't swap a song with itself")]
    CannotSwapSameSong,
}

impl Swapable for TrackQueue {
    fn swap(&self, first_track_pos: usize, second_track_pos: usize) -> Result<(), SwapableError> {
        self.modify_queue(|q| {
            let first_track_pos = min(first_track_pos, second_track_pos);
            let second_track_pos = max(first_track_pos, second_track_pos);

            if q.len() < first_track_pos
                || q.len() < second_track_pos
                || first_track_pos < 1
                || second_track_pos < 1
            {
                return Err(SwapableError::IndexOutOfBounds);
            }

            if q.is_empty() {
                return Err(SwapableError::NothingIsPlaying);
            }

            if first_track_pos == second_track_pos {
                return Err(SwapableError::CannotSwapSameSong);
            }

            let first_track_idx = first_track_pos - 1;
            let second_track_idx = second_track_pos - 1;

            if first_track_pos == 1 {
                let first_track = q.get(first_track_idx).unwrap();
                _ = first_track.pause();
                _ = first_track.seek(Duration::from_secs(0));

                let second_track = q.get(second_track_idx).unwrap();
                second_track.play().unwrap();
            }

            q.swap(first_track_idx, second_track_idx);

            Ok(())
        })
    }
}

#[tracing::instrument(skip(ctx))]
pub async fn swap(ctx: &Context, command: &CommandInteraction) {
    let guild_id = command.guild_id.expect("Couldn't get guild ID");
    let guild = guild_id.to_guild_cached(&ctx.cache).map(|g| g.to_owned());

    command.defer(&ctx.http).await.unwrap();

    if let Some(guild) = guild
        && is_bot_in_another_voice_channel(ctx, &guild, command.user.id)
    {
        voice_channel_not_same_response(command, ctx).await;
        return;
    }

    let Some(call_lock) = get_call_lock(ctx, guild_id, command).await else {
        return;
    };

    let call = timeout(Duration::from_secs(30), call_lock.lock())
        .await
        .unwrap();

    let queue = call.queue();

    let Some((first_track_pos, second_track_pos)) = get_track_numbers(command) else {
        invalid_number_response(command, ctx).await;
        return;
    };

    let first_track = get_track_from_queue(queue, first_track_pos);
    let second_track = get_track_from_queue(queue, second_track_pos);

    let (Some(first_track), Some(second_track)) = (first_track, second_track) else {
        track_not_in_queue_response(command, ctx).await;
        return;
    };

    match queue.swap(first_track_pos, second_track_pos) {
        Ok(()) => {
            swapping_success_response(
                command,
                ctx,
                first_track_pos,
                first_track,
                second_track_pos,
                second_track,
            )
            .await;
        }
        Err(e) => swapping_error_response(e, command, ctx).await,
    }
}

async fn swapping_success_response(
    command: &CommandInteraction,
    ctx: &Context,
    first_track_idx: usize,
    first_track: MyAuxMetadata,
    second_track_idx: usize,
    second_track: MyAuxMetadata,
) {
    command
        .edit_response(
            &ctx.http,
            EditInteractionResponse::new().content(format!(
                "
Swapped tracks 
{}. {} 
and 
{}. {}
",
                first_track_idx,
                first_track
                    .aux_metadata
                    .title
                    .unwrap_or_else(|| "NO TITLE".to_string()),
                second_track_idx,
                second_track
                    .aux_metadata
                    .title
                    .unwrap_or_else(|| "NO TITLE".to_string())
            )),
        )
        .instrument(info_span!("Sending message"))
        .await
        .expect("Error creating interaction response");
}

fn get_track_from_queue(queue: &TrackQueue, track_number: usize) -> Option<MyAuxMetadata> {
    let track_metadata = (*queue
        .current_queue()
        .get(track_number - 1)?
        .data::<MyAuxMetadata>())
    .clone();

    Some(track_metadata)
}

async fn invalid_number_response(command: &CommandInteraction, ctx: &Context) {
    command
        .edit_response(
            &ctx.http,
            EditInteractionResponse::new().content("Invalid number!"),
        )
        .instrument(info_span!("Sending message"))
        .await
        .expect("Error creating interaction response");
}

async fn swapping_error_response(e: SwapableError, command: &CommandInteraction, ctx: &Context) {
    match e {
        SwapableError::IndexOutOfBounds => {
            track_not_in_queue_response(command, ctx).await;
        }
        SwapableError::NothingIsPlaying => {
            command
                .edit_response(
                    &ctx.http,
                    EditInteractionResponse::new().content("Nothing is playing!"),
                )
                .instrument(info_span!("Sending message"))
                .await
                .expect("Error creating interaction response");
        }
        SwapableError::CannotSwapSameSong => {
            command
                .edit_response(
                    &ctx.http,
                    EditInteractionResponse::new().content("Can't swap the same song!"),
                )
                .instrument(info_span!("Sending message"))
                .await
                .expect("Error creating interaction response");
        }
    }
}

async fn track_not_in_queue_response(command: &CommandInteraction, ctx: &Context) {
    command
        .edit_response(
            &ctx.http,
            EditInteractionResponse::new().content("That track isn't in the queue!"),
        )
        .instrument(info_span!("Sending message"))
        .await
        .expect("Error creating interaction response");
}

fn get_track_numbers(command: &CommandInteraction) -> Option<(usize, usize)> {
    let CommandDataOptionValue::Integer(first_track_idx) =
        command.data.options.first().unwrap().value
    else {
        return None;
    };

    let CommandDataOptionValue::Integer(second_track_idx) =
        command.data.options.get(1).unwrap().value
    else {
        return None;
    };

    Some((
        first_track_idx.try_into().ok()?,
        second_track_idx.try_into().ok()?,
    ))
}
