use serenity::{
    client::Context,
    model::prelude::interaction::application_command::ApplicationCommandInteraction,
};
use songbird::tracks::LoopState;

/// Loop the current track
pub async fn loop_song(ctx: &Context, command: &ApplicationCommandInteraction) {
    let guild_id = command.guild_id.expect("Couldn't get guild ID");

    let manager = songbird::get(ctx)
        .await
        .expect("Songbird Voice client placed in at initialisation.")
        .clone();

    // Get call
    let call_lock = match manager.get(guild_id.0) {
        Some(c) => c,
        None => {
            not_in_vc_response(command, ctx).await;
            return;
        }
    };
    let call = call_lock.lock().await;

    let track = match call.queue().current() {
        Some(c) => c,
        None => {
            nothing_playing_response(command, ctx).await;
            return;
        }
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
        .create_interaction_response(&ctx.http, |r| {
            r.interaction_response_data(|d| {
                d.content("Must be in a voice channel to use that command!")
            })
        })
        .await
        .expect("Error creating interaction response");
}

async fn nothing_playing_response(command: &ApplicationCommandInteraction, ctx: &Context) {
    command
        .create_interaction_response(&ctx.http, |r| {
            r.interaction_response_data(|d| d.content("Nothing is playing."))
        })
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
        .create_interaction_response(&ctx.http, |m| {
            m.interaction_response_data(|d| d.content("Looping the current song."))
        })
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
        .create_interaction_response(&ctx.http, |m| {
            m.interaction_response_data(|d| d.content("No longer looping the current song."))
        })
        .await
        .expect("Error creating interaction response");
}
