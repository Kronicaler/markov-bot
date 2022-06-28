use std::ops::ControlFlow;

use serenity::{
    client::Context,
    model::interactions::application_command::{
        ApplicationCommandInteraction, ApplicationCommandInteractionDataOptionValue,
    },
    utils::Colour,
};

use super::helper_funcs::{is_bot_in_another_channel, get_call};

/// Skip the track
pub async fn skip(ctx: &Context, command: &ApplicationCommandInteraction) {
    let guild_id = command.guild_id.expect("Couldn't get guild ID");

    if let ControlFlow::Break(_) = respond_if_not_same_vc(guild_id, ctx, command).await {
        return;
    }

    let call = match get_call(ctx, guild_id, command).await {
        Some(value) => value,
        None => return,
    };

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

async fn respond_if_not_same_vc(
    guild_id: serenity::model::id::GuildId,
    ctx: &Context,
    command: &ApplicationCommandInteraction,
) -> ControlFlow<()> {
    if let Some(guild) = guild_id.to_guild_cached(&ctx.cache).await {
        if is_bot_in_another_channel(ctx, &guild, command.user.id).await {
            command
                .create_interaction_response(&ctx.http, |r| {
                    r.interaction_response_data(|d| {
                        d.content("Must be in the same voice channel to use that command!")
                    })
                })
                .await
                .expect("Error creating interaction response");
            return ControlFlow::Break(());
        }
    }
    ControlFlow::Continue(())
}

fn get_track_number(command: &ApplicationCommandInteraction) -> Option<usize> {
    let track_number = command
        .data
        .options
        .iter()
        .find(|opt| opt.name == "number")?;

    match track_number.resolved.as_ref().unwrap() {
        ApplicationCommandInteractionDataOptionValue::Integer(s) => {
            Some((*s).try_into().expect("invalid number"))
        }
        _ => panic!("expected an integer!"),
    }
}
