use std::ops::ControlFlow;

use serenity::{
    builder::{CreateEmbed, CreateInteractionResponse, CreateInteractionResponseData},
    client::Context,
    model::prelude::{
        interaction::application_command::{ApplicationCommandInteraction, CommandDataOptionValue},
        Colour,
    },
};

use super::helper_funcs::{get_call_lock, is_bot_in_another_channel};

/// Skip the track
pub async fn skip(ctx: &Context, command: &ApplicationCommandInteraction) {
    let guild_id = command.guild_id.expect("Couldn't get guild ID");

    if let ControlFlow::Break(_) = respond_if_not_same_vc(guild_id, ctx, command).await {
        return;
    }

    let call_lock = match get_call_lock(ctx, guild_id, command).await {
        Some(value) => value,
        None => return,
    };
    let call = call_lock.lock().await;

    if call.queue().is_empty() {
        command
            .create_interaction_response(
                &ctx.http,
                CreateInteractionResponse::new().interaction_response_data(
                    CreateInteractionResponseData::new().content("The queue is empty."),
                ),
            )
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
                .create_interaction_response(
                    &ctx.http,
                    CreateInteractionResponse::new().interaction_response_data(
                        CreateInteractionResponseData::new()
                            .embed(CreateEmbed::new().title("Couldn't skip song")),
                    ),
                )
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
        .create_interaction_response(
            &ctx.http,
            CreateInteractionResponse::new().interaction_response_data(
                CreateInteractionResponseData::new()
                    .embed(CreateEmbed::new().title(title).colour(colour)),
            ),
        )
        .await
        .expect("Error creating interaction response");
}

async fn respond_if_not_same_vc(
    guild_id: serenity::model::id::GuildId,
    ctx: &Context,
    command: &ApplicationCommandInteraction,
) -> ControlFlow<()> {
    let guild = guild_id
        .to_guild_cached(&ctx.cache)
        .and_then(|g| Some(g.to_owned()));

    if let Some(guild) = guild {
        if is_bot_in_another_channel(ctx, &guild, command.user.id) {
            command
                .create_interaction_response(
                    &ctx.http,
                    CreateInteractionResponse::new().interaction_response_data(
                        CreateInteractionResponseData::new()
                            .content("Must be in the same voice channel to use that command!"),
                    ),
                )
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

    match track_number.value {
        CommandDataOptionValue::Integer(s) => Some((s).try_into().expect("invalid number")),
        _ => panic!("expected an integer!"),
    }
}
