use serenity::{
    builder::EditInteractionResponse, client::Context,
    model::prelude::interaction::application_command::ApplicationCommandInteraction,
};
use songbird::tracks::LoopState;
use tracing::{info_span, Instrument};

use super::helper_funcs::{is_bot_in_another_voice_channel, voice_channel_not_same_response};

/// Loop the current track
#[tracing::instrument(skip(ctx))]
pub async fn loop_song(ctx: &Context, command: &ApplicationCommandInteraction) {
    let guild_id = command.guild_id.expect("Couldn't get guild ID");

    command.defer(&ctx.http).await.unwrap();

    if is_bot_in_another_voice_channel(
        ctx,
        &guild_id.to_guild_cached(&ctx.cache).unwrap(),
        command.user.id,
    ) {
        voice_channel_not_same_response(command, ctx).await;
        return;
    }

    let manager = songbird::get(ctx)
        .await
        .expect("Songbird Voice client placed in at initialization.")
        .clone();

    // Get call
    let Some(call_lock) = manager.get(guild_id) else {
        not_in_vc_response(command, ctx).await;
        return;
    };
    let call = call_lock.lock().await;

    let Some(track) = call.queue().current() else {
        nothing_playing_response(command, ctx).await;
        return;
    };

    match track.get_info().await.unwrap().loops {
        LoopState::Finite(loop_state) => {
            if loop_state == 0 {
                enable_looping(&track, command, ctx).await;
            } else {
                disable_looping(&track, command, ctx).await;
            }
        }
        LoopState::Infinite => {
            disable_looping(&track, command, ctx).await;
        }
    }
}

async fn not_in_vc_response(command: &ApplicationCommandInteraction, ctx: &Context) {
    command
        .edit_original_interaction_response(
            &ctx.http,
            EditInteractionResponse::new()
                .content("Must be in a voice channel to use that command!"),
        )
        .instrument(info_span!("Sending message"))
        .await
        .expect("Error creating interaction response");
}

async fn nothing_playing_response(command: &ApplicationCommandInteraction, ctx: &Context) {
    command
        .edit_original_interaction_response(
            &ctx.http,
            EditInteractionResponse::new().content("Nothing is playing."),
        )
        .instrument(info_span!("Sending message"))
        .await
        .expect("Couldn't create response");
}

async fn enable_looping(
    track: &songbird::tracks::TrackHandle,
    command: &ApplicationCommandInteraction,
    ctx: &Context,
) {
    track.enable_loop().unwrap();
    command
        .edit_original_interaction_response(
            &ctx.http,
            EditInteractionResponse::new().content("Looping the current song."),
        )
        .instrument(info_span!("Sending message"))
        .await
        .expect("Error creating interaction response");
}

async fn disable_looping(
    track: &songbird::tracks::TrackHandle,
    command: &ApplicationCommandInteraction,
    ctx: &Context,
) {
    track.disable_loop().unwrap();
    command
        .edit_original_interaction_response(
            &ctx.http,
            EditInteractionResponse::new().content("No longer looping the current song."),
        )
        .instrument(info_span!("Sending message"))
        .await
        .expect("Error creating interaction response");
}
