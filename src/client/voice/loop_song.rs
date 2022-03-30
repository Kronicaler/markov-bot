use serenity::{
    client::Context, model::interactions::application_command::ApplicationCommandInteraction,
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
    let call_lock = manager.get(guild_id.0);
    if call_lock.is_none() {
        command
            .create_interaction_response(&ctx.http, |r| {
                r.interaction_response_data(|d| {
                    d.content("Must be in a voice channel to use that command!")
                })
            })
            .await
            .expect("Error creating interaction response");
        return;
    }
    let call_lock = call_lock.expect("Couldn't get handler lock");
    let call = call_lock.lock().await;

    if call.queue().current().is_none() {
        command
            .create_interaction_response(&ctx.http, |r| {
                r.interaction_response_data(|d| d.content("Nothing is playing."))
            })
            .await
            .expect("Couldn't create response");
        return;
    }

    let track = call.queue().current().unwrap();

    match track.get_info().await.unwrap().loops {
        LoopState::Finite(loop_state) => {
            if loop_state == 0 {
                track.enable_loop().unwrap();

                command
                    .create_interaction_response(&ctx.http, |m| {
                        m.interaction_response_data(|d| d.content("Looping the current song."))
                    })
                    .await
                    .expect("Error creating interaction response");
            } else {
                track.disable_loop().unwrap();

                command
                    .create_interaction_response(&ctx.http, |m| {
                        m.interaction_response_data(|d| {
                            d.content("No longer looping the current song.")
                        })
                    })
                    .await
                    .expect("Error creating interaction response");
            }
        }
        _ => {
            track.disable_loop().unwrap();

            command
                .create_interaction_response(&ctx.http, |m| {
                    m.interaction_response_data(|d| {
                        d.content("No longer looping the current song.")
                    })
                })
                .await
                .expect("Error creating interaction response");
        }
    }
}
