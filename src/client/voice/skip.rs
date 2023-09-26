use std::str::FromStr;

use anyhow::Context;
use serenity::{
    builder::{CreateEmbed, CreateInteractionResponse, EditInteractionResponse},
    client::Context as ClientContext,
    model::prelude::{
        interaction::{
            application_command::ApplicationCommandInteraction,
            message_component::MessageComponentInteraction,
        },
        Colour,
    },
};
use strum_macros::EnumString;
use tracing::{info_span, Instrument};

use super::helper_funcs::{
    get_call_lock, is_bot_in_another_voice_channel, voice_channel_not_same_response,
};

#[tracing::instrument(skip(ctx))]
pub async fn skip_button_press(ctx: &ClientContext, component: &MessageComponentInteraction) {
    let guild_id = component.guild_id.expect("Couldn't get guild ID");

    if is_bot_in_another_voice_channel(
        ctx,
        &guild_id.to_guild_cached(&ctx.cache).unwrap().clone(),
        component.user.id,
    ) {
        return;
    }

    let manager = songbird::get(ctx)
        .await
        .expect("Songbird Voice client placed in at initialization.")
        .clone();

    let Some(call_lock) = manager.get(guild_id) else {
        return;
    };
    let call = call_lock.lock().await;

    if call.queue().is_empty() {
        return;
    }

    call.queue().skip().expect("Couldn't skip song");

    component
        .create_interaction_response(&ctx.http, CreateInteractionResponse::new())
        .instrument(info_span!("Sending button response"))
        .await
        .unwrap();
}

/// Skip the track
#[tracing::instrument(skip(ctx))]
pub async fn skip(ctx: &ClientContext, command: &ApplicationCommandInteraction) {
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

    let Some(call_lock) = get_call_lock(ctx, guild_id, command).await else {
        return;
    };
    let call = call_lock.lock().await;

    if call.queue().is_empty() {
        empty_queue_response(command, ctx).await;
        return;
    }

    let skip_info = get_skip_info(command);

    if let Some(skip_info) = skip_info {
        let track_number = skip_info.1;

        match skip_info.0 {
            SkipType::Number => {
                let success = handle_skip_type_number(track_number, &call);

                if !success {
                    couldnt_skip_response(command, ctx).await;
                    return;
                }
            }
            SkipType::Until => {
                if handle_skip_type_until(&call, track_number).is_err() {
                    couldnt_skip_response(command, ctx).await;
                    return;
                }
            }
        }
    } else {
        call.queue().skip().expect("Couldn't skip song");
    }

    skip_embed_response(&call, command, ctx).await;
}

fn handle_skip_type_until(
    call: &tokio::sync::MutexGuard<songbird::Call>,
    track_number: i64,
) -> anyhow::Result<()> {
    call.queue().modify_queue(|q| -> anyhow::Result<()> {
        for _ in 1..track_number - 1 {
            q.pop_front()
                .context("No songs left in the queue")?
                .stop()?;
        }

        Ok(())
    })?;
    call.queue().skip()?;
    Ok(())
}

fn handle_skip_type_number(
    track_number: i64,
    call: &tokio::sync::MutexGuard<songbird::Call>,
) -> bool {
    let success = if track_number == 1 {
        call.queue().skip().is_ok()
    } else {
        call.queue()
            .dequeue((track_number - 1).try_into().unwrap())
            .is_some()
    };
    success
}

async fn couldnt_skip_response(command: &ApplicationCommandInteraction, ctx: &ClientContext) {
    command
        .edit_original_interaction_response(
            &ctx.http,
            EditInteractionResponse::new().embed(CreateEmbed::new().title("Couldn't skip song")),
        )
        .instrument(info_span!("Sending message"))
        .await
        .expect("Error creating interaction response");
}

async fn skip_embed_response(
    call: &songbird::Call,
    command: &ApplicationCommandInteraction,
    ctx: &ClientContext,
) {
    let title = format!("Song skipped, {} left in queue.", call.queue().len() - 1);
    let colour = Colour::from_rgb(149, 8, 2);
    command
        .edit_original_interaction_response(
            &ctx.http,
            EditInteractionResponse::new().embed(CreateEmbed::new().title(title).colour(colour)),
        )
        .instrument(info_span!("Sending message"))
        .await
        .expect("Error creating interaction response");
}

async fn empty_queue_response(command: &ApplicationCommandInteraction, ctx: &ClientContext) {
    command
        .edit_original_interaction_response(
            &ctx.http,
            EditInteractionResponse::new().content("The queue is empty."),
        )
        .instrument(info_span!("Sending message"))
        .await
        .expect("Couldn't create response");
}

fn get_skip_info(command: &ApplicationCommandInteraction) -> Option<(SkipType, i64)> {
    let command_data_option = command.data.options.get(0)?;

    let skip_type = SkipType::from_str(&command_data_option.name).unwrap();
    let track_number = command_data_option.value.as_i64().unwrap();

    Some((skip_type, track_number))
}

#[derive(EnumString)]
enum SkipType {
    #[strum(serialize = "number")]
    Number,
    #[strum(serialize = "until")]
    Until,
}
