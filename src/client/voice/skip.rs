use serenity::{
    builder::{CreateEmbed, EditInteractionResponse},
    client::Context,
    model::prelude::{
        interaction::application_command::{ApplicationCommandInteraction, CommandDataOptionValue},
        Colour,
    },
};

use super::helper_funcs::{
    get_call_lock, is_bot_in_another_voice_channel, voice_channel_not_same_response,
};

/// Skip the track
pub async fn skip(ctx: &Context, command: &ApplicationCommandInteraction) {
    let guild_id = command.guild_id.expect("Couldn't get guild ID");

    command.defer(&ctx.http).await.unwrap();

    if is_bot_in_another_voice_channel(
        ctx,
        &guild_id.to_guild_cached(&ctx.cache).unwrap().clone(),
        command.user.id,
    ) {
        voice_channel_not_same_response(command, ctx).await;
        return;
    }

    let call_lock = match get_call_lock(ctx, guild_id, command).await {
        Some(value) => value,
        None => return,
    };
    let call = call_lock.lock().await;

    if call.queue().is_empty() {
        empty_queue_response(command, ctx).await;
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
                .edit_original_interaction_response(
                    &ctx.http,
                    EditInteractionResponse::new()
                        .embed(CreateEmbed::new().title("Couldn't skip song")),
                )
                .await
                .expect("Error creating interaction response");
            return;
        }
    } else {
        call.queue().skip().expect("Couldn't skip song");
    }

    skip_embed_response(&call, command, ctx).await;
}

async fn skip_embed_response(
    call: &songbird::Call,
    command: &ApplicationCommandInteraction,
    ctx: &Context,
) {
    let title = format!("Song skipped, {} left in queue.", call.queue().len() - 1);
    let colour = Colour::from_rgb(149, 8, 2);
    command
        .edit_original_interaction_response(
            &ctx.http,
            EditInteractionResponse::new().embed(CreateEmbed::new().title(title).colour(colour)),
        )
        .await
        .expect("Error creating interaction response");
}

async fn empty_queue_response(command: &ApplicationCommandInteraction, ctx: &Context) {
    command
        .edit_original_interaction_response(
            &ctx.http,
            EditInteractionResponse::new().content("The queue is empty."),
        )
        .await
        .expect("Couldn't create response");
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
