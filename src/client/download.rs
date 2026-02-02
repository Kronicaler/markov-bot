use std::str::FromStr;

use crate::client::{
    get_option_from_command::GetOptionFromCommand, helper_funcs::post_file_from_message,
};
use serenity::{
    all::{CommandInteraction, Context, Message},
    small_fixed_array::FixedString,
};

#[tracing::instrument(skip(ctx, command))]
pub async fn download_command(ctx: &Context, command: &CommandInteraction) {
    let query = command.data.options.get(0).unwrap().value.as_str().unwrap();
    let ephemeral = command
        .data
        .options
        .get(1)
        .and_then(|x| Some(x.value.as_bool().unwrap_or(true)))
        .unwrap_or(true);

    if ephemeral {
        command.defer_ephemeral(&ctx.http).await.unwrap();
    } else {
        command.defer(&ctx.http).await.unwrap();
    }

    let mut x = Message::default();
    x.content = FixedString::from_str(query).unwrap();

    post_file_from_message(ctx, command, &x).await;
}

#[tracing::instrument(skip(ctx, command))]
pub async fn download_from_message_command(ctx: &Context, command: &CommandInteraction) {
    command.defer(&ctx.http).await.unwrap();

    let message_id = command.data.target_id.unwrap();

    let message = command
        .data
        .resolved
        .messages
        .get(&message_id.into())
        .unwrap();

    post_file_from_message(ctx, command, message).await;
}
