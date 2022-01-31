use serenity::{
    client::Context,
    model::interactions::application_command::{
        ApplicationCommandInteraction, ApplicationCommandInteractionDataOptionValue,
    },
    utils::Colour,
};

/// Skip the track
pub async fn skip(ctx: &Context, command: &ApplicationCommandInteraction) {
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

    if call.queue().is_empty() {
        command
            .create_interaction_response(&ctx.http, |r| {
                r.interaction_response_data(|d| d.content("The queue is empty."))
            })
            .await
            .expect("Couldn't create response");
        return;
    }

    let track_number = get_track_number(command);

    if track_number.is_some() {
        let success = if track_number.unwrap() == 1 {
            call.queue().skip().is_ok()
        } else {
            call.queue().dequeue(track_number.unwrap() - 1).is_some()
        };

        if !success {
            command
                .create_interaction_response(&ctx.http, |m| {
                    m.interaction_response_data(|d| {
                        d.create_embed(|e| e.title("Couldn't skip song"))
                    })
                })
                .await
                .expect("Error creating interaction response");
            return;
        }
    } else {
        call.queue().skip().expect("Couldn't skip song");
    }

    // Embed
    let title = format!("Song skipped, {} left in queue.", call.queue().len() - 1);
    let colour = Colour::from_rgb(149, 8, 2);

    command
        .create_interaction_response(&ctx.http, |m| {
            m.interaction_response_data(|d| d.create_embed(|e| e.title(title).colour(colour)))
        })
        .await
        .expect("Error creating interaction response");
}

fn get_track_number(command: &ApplicationCommandInteraction) -> Option<usize> {
    let track_number = command.data.options.iter().find(|opt| opt.name == "number");

    track_number?;

    let track_number = track_number.unwrap();

    match track_number.resolved.as_ref().unwrap() {
        ApplicationCommandInteractionDataOptionValue::Integer(s) => {
            Some((*s).try_into().expect("invalid number"))
        }
        _ => panic!("expected an integer!"),
    }
}
